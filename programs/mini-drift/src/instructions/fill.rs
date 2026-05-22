use crate::{
    controller::{
        orders::update_order_after_fill,
        position::{decrease_open_bids_and_asks, update_position_and_market, PositionDelta},
    },
    error::{ErrorCode, MiniDriftResult},
    math::{
        amm::{calculate_mark_price, swap_base_asset, update_amm_reserves, SwapDirection},
        amm_spread::{apply_spread_to_quote, calculate_spread},
        constants::{BASE_DECIMALS, PRICE_PRECISION},
        oracle::{is_oracle_valid, validate_mark_oracle_divergence},
        orders::calculate_quote_asset_amount_for_maker_order,
    },
    state::{
        events::get_order_action_record,
        fill_mode::FillMode,
        oracle::OraclePriceData,
        perp_market::PerpMarket,
        user::{OrderStatus, PositionDirection, User},
    },
};
use anchor_lang::prelude::*;

fn validate_amm_fill_price(
    direction: PositionDirection,
    base_asset_amount: u64,
    quote_asset_amount: u64,
    limit_price: Option<u64>,
) -> MiniDriftResult<()> {
    let Some(limit_price) = limit_price else {
        return Ok(());
    };

    let limit_quote_asset_amount = calculate_quote_asset_amount_for_maker_order(
        base_asset_amount,
        limit_price,
        BASE_DECIMALS,
        direction,
    )?;

    match direction {
        PositionDirection::Long if quote_asset_amount > limit_quote_asset_amount => {
            Err(ErrorCode::InvalidFillPrice)
        }
        PositionDirection::Short if quote_asset_amount < limit_quote_asset_amount => {
            Err(ErrorCode::InvalidFillPrice)
        }
        _ => Ok(()),
    }
}

#[allow(clippy::too_many_arguments)]
pub fn handle_fill_perp_order_amm(
    taker_user: &mut User,
    taker: Pubkey,
    filler: Pubkey,
    taker_order_index: usize,
    taker_position_index: usize,
    market: &mut PerpMarket,
    oracle_price_data: &OraclePriceData,
    now: i64,
) -> MiniDriftResult<()> {
    let order = *taker_user
        .orders
        .get(taker_order_index)
        .ok_or(ErrorCode::UnexpectedError)?;
    if order.status != OrderStatus::Open {
        return Err(ErrorCode::InvalidOrderStatus);
    }

    if order.market_index != market.market_index {
        return Err(ErrorCode::InvalidMarketAccount);
    }

    let position = *taker_user
        .perp_positions
        .get(taker_position_index)
        .ok_or(ErrorCode::UnexpectedError)?;
    if !position.is_for(order.market_index) {
        return Err(ErrorCode::InvalidPerpPositionDetected);
    }

    is_oracle_valid(
        oracle_price_data,
        market.oracle_max_delay,
        market.oracle_max_confidence,
    )?;
    let mark_price = calculate_mark_price(&market.amm)?;
    validate_mark_oracle_divergence(mark_price, oracle_price_data)?;

    let base_asset_amount = order.get_base_asset_amount_unfilled(None)?;
    let order_direction = order.direction;
    let update_open_bids_and_asks = order.update_open_bids_and_asks();
    let swap_direction = match order_direction {
        PositionDirection::Long => SwapDirection::Subtract,
        PositionDirection::Short => SwapDirection::Add,
    };

    let quote_delta = swap_base_asset(&market.amm, base_asset_amount, swap_direction)?;
    let reserve_price = u64::try_from(mark_price).map_err(|_| ErrorCode::MathError)?;
    let oracle_price = u128::try_from(oracle_price_data.price).map_err(|_| ErrorCode::MathError)?;
    let oracle_conf_pct = u64::try_from(
        u128::from(oracle_price_data.confidence)
            .checked_mul(PRICE_PRECISION)
            .ok_or(ErrorCode::MathError)?
            .checked_div(oracle_price)
            .ok_or(ErrorCode::MathError)?,
    )
    .map_err(|_| ErrorCode::MathError)?;
    let (long_spread, short_spread) = calculate_spread(
        market.amm.base_spread,
        market.amm.max_spread,
        market.amm.base_asset_amount_with_amm,
        reserve_price,
        oracle_conf_pct,
        market.amm.mark_std,
        market.amm.oracle_std,
        market.amm.long_intensity_volume,
        market.amm.short_intensity_volume,
        market.amm.volume_24h,
        market.amm.base_asset_amount_long,
        market.amm.base_asset_amount_short,
    )?;
    let quote_with_spread =
        apply_spread_to_quote(quote_delta, long_spread, short_spread, order_direction)?;
    let base_asset_amount_i64 =
        i64::try_from(base_asset_amount).map_err(|_| ErrorCode::MathError)?;
    let quote_delta_i64 = i64::try_from(quote_with_spread).map_err(|_| ErrorCode::MathError)?;
    let limit_price = FillMode::Fill.get_limit_price(&order)?;
    validate_amm_fill_price(order_direction, base_asset_amount, quote_delta, limit_price)?;

    let delta = match order_direction {
        PositionDirection::Long => PositionDelta {
            base_asset_amount: base_asset_amount_i64,
            quote_asset_amount: quote_delta_i64.checked_neg().ok_or(ErrorCode::MathError)?,
        },
        PositionDirection::Short => PositionDelta {
            base_asset_amount: base_asset_amount_i64
                .checked_neg()
                .ok_or(ErrorCode::MathError)?,
            quote_asset_amount: quote_delta_i64,
        },
    };

    let mut market_after_fill = market.clone();
    let amm_quote_delta = update_amm_reserves(
        &mut market_after_fill.amm,
        base_asset_amount,
        swap_direction,
    )?;
    if amm_quote_delta != quote_delta {
        return Err(ErrorCode::MathError);
    }

    let mut position_after_fill = position;
    let _realized_pnl =
        update_position_and_market(&mut position_after_fill, &delta, &mut market_after_fill)?;
    decrease_open_bids_and_asks(
        &mut position_after_fill,
        &order_direction,
        base_asset_amount,
        update_open_bids_and_asks,
    )?;

    let mut order_after_fill = order;
    let is_filled =
        update_order_after_fill(&mut order_after_fill, base_asset_amount, quote_with_spread)?;
    let mut open_orders_after_fill = taker_user.open_orders;
    if is_filled {
        position_after_fill.open_orders = position_after_fill
            .open_orders
            .checked_sub(1)
            .ok_or(ErrorCode::MathError)?;
        open_orders_after_fill = open_orders_after_fill.saturating_sub(1);
    }

    *market = market_after_fill;
    taker_user.perp_positions[taker_position_index] = position_after_fill;
    taker_user.orders[taker_order_index] = order_after_fill;
    taker_user.open_orders = open_orders_after_fill;

    emit!(get_order_action_record(
        now,
        order.market_index,
        filler,
        base_asset_amount,
        quote_delta,
        taker,
        order_after_fill,
    ));

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        math::constants::{PEG_PRECISION, PRICE_PRECISION},
        state::{
            perp_market::Amm,
            user::{Order, OrderType, PerpPosition},
        },
    };

    fn test_market(market_index: u16) -> PerpMarket {
        PerpMarket {
            market_index,
            amm: Amm {
                base_asset_reserve: 100,
                quote_asset_reserve: 100,
                peg_multiplier: PEG_PRECISION,
                min_base_asset_reserve: 50,
                max_base_asset_reserve: 150,
                order_step_size: 1,
                ..Amm::default()
            },
            ..PerpMarket::default()
        }
    }

    fn valid_oracle_price_data() -> OraclePriceData {
        OraclePriceData {
            price: i64::try_from(PRICE_PRECISION).unwrap(),
            confidence: 0,
            delay: 0,
            has_sufficient_number_of_data_points: true,
            sequence_id: 1,
        }
    }

    fn open_order(
        market_index: u16,
        direction: PositionDirection,
        base_asset_amount: u64,
    ) -> Order {
        Order {
            market_index,
            direction,
            base_asset_amount,
            order_type: OrderType::Limit,
            status: OrderStatus::Open,
            ..Order::default()
        }
    }

    fn position_with_open_order(
        market_index: u16,
        direction: PositionDirection,
        base_asset_amount: u64,
    ) -> PerpPosition {
        let mut position = PerpPosition {
            market_index,
            open_orders: 1,
            ..PerpPosition::default()
        };

        match direction {
            PositionDirection::Long => {
                position.open_bids = i64::try_from(base_asset_amount).unwrap();
            }
            PositionDirection::Short => {
                position.open_asks = -i64::try_from(base_asset_amount).unwrap();
            }
        }

        position
    }

    #[test]
    fn validate_amm_fill_price_uses_scaled_price_precision() {
        let result = validate_amm_fill_price(
            PositionDirection::Long,
            3_000_000_000,
            360_000_000,
            Some(120_000_000),
        );

        assert_eq!(result, Ok(()));

        let result = validate_amm_fill_price(
            PositionDirection::Long,
            3_000_000_000,
            360_000_001,
            Some(120_000_000),
        );

        assert_eq!(result, Err(ErrorCode::InvalidFillPrice));
    }

    #[test]
    fn handle_fill_perp_order_amm_fills_long_order_and_decreases_open_bids() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(0, PositionDirection::Long, 20);
        user.perp_positions[0] = position_with_open_order(0, PositionDirection::Long, 20);
        let mut market = test_market(0);

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &valid_oracle_price_data(),
            0,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(market.amm.base_asset_reserve, 80);
        assert_eq!(market.amm.quote_asset_reserve, 125);
        assert_eq!(user.perp_positions[0].base_asset_amount, 20);
        assert_eq!(user.perp_positions[0].quote_asset_amount, -25);
        assert_eq!(user.perp_positions[0].open_orders, 0);
        assert_eq!(user.perp_positions[0].open_bids, 0);
        assert_eq!(user.open_orders, 0);
        assert_eq!(user.orders[0].base_asset_amount_filled, 20);
        assert_eq!(user.orders[0].quote_asset_amount_filled, 25);
        assert_eq!(user.orders[0].status, OrderStatus::Filled);
    }

    #[test]
    fn handle_fill_perp_order_amm_fills_short_order_and_decreases_open_asks() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(0, PositionDirection::Short, 25);
        user.perp_positions[0] = position_with_open_order(0, PositionDirection::Short, 25);
        let mut market = test_market(0);

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &valid_oracle_price_data(),
            0,
        );

        assert_eq!(result, Ok(()));
        assert_eq!(market.amm.base_asset_reserve, 125);
        assert_eq!(market.amm.quote_asset_reserve, 80);
        assert_eq!(user.perp_positions[0].base_asset_amount, -25);
        assert_eq!(user.perp_positions[0].quote_asset_amount, 20);
        assert_eq!(user.perp_positions[0].open_orders, 0);
        assert_eq!(user.perp_positions[0].open_asks, 0);
        assert_eq!(user.open_orders, 0);
        assert_eq!(user.orders[0].base_asset_amount_filled, 25);
        assert_eq!(user.orders[0].quote_asset_amount_filled, 20);
        assert_eq!(user.orders[0].status, OrderStatus::Filled);
    }

    #[test]
    fn handle_fill_perp_order_amm_rejects_non_open_order_before_mutation() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(0, PositionDirection::Long, 20);
        user.orders[0].status = OrderStatus::Canceled;
        user.perp_positions[0] = position_with_open_order(0, PositionDirection::Long, 20);
        let mut market = test_market(0);
        let original_amm = market.amm;

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &valid_oracle_price_data(),
            0,
        );

        assert_eq!(result, Err(ErrorCode::InvalidOrderStatus));
        assert_eq!(market.amm, original_amm);
        assert_eq!(user.orders[0].base_asset_amount_filled, 0);
        assert_eq!(user.perp_positions[0].base_asset_amount, 0);
        assert_eq!(user.perp_positions[0].open_bids, 20);
    }

    #[test]
    fn handle_fill_perp_order_amm_rejects_wrong_market_before_mutation() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(1, PositionDirection::Long, 20);
        user.perp_positions[0] = position_with_open_order(1, PositionDirection::Long, 20);
        let mut market = test_market(0);
        let original_amm = market.amm;

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &valid_oracle_price_data(),
            0,
        );

        assert_eq!(result, Err(ErrorCode::InvalidMarketAccount));
        assert_eq!(market.amm, original_amm);
        assert_eq!(user.orders[0].base_asset_amount_filled, 0);
        assert_eq!(user.perp_positions[0].base_asset_amount, 0);
        assert_eq!(user.perp_positions[0].open_bids, 20);
    }

    #[test]
    fn handle_fill_perp_order_amm_rejects_wrong_position_before_mutation() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(0, PositionDirection::Long, 20);
        user.perp_positions[0] = position_with_open_order(1, PositionDirection::Long, 20);
        let mut market = test_market(0);
        let original_amm = market.amm;

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &valid_oracle_price_data(),
            0,
        );

        assert_eq!(result, Err(ErrorCode::InvalidPerpPositionDetected));
        assert_eq!(market.amm, original_amm);
        assert_eq!(user.orders[0].base_asset_amount_filled, 0);
        assert_eq!(user.perp_positions[0].base_asset_amount, 0);
        assert_eq!(user.perp_positions[0].open_bids, 20);
    }

    #[test]
    fn handle_fill_perp_order_amm_rejects_invalid_oracle_before_mutation() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(0, PositionDirection::Long, 20);
        user.perp_positions[0] = position_with_open_order(0, PositionDirection::Long, 20);
        let mut market = test_market(0);
        let original_amm = market.amm;
        let invalid_oracle_price_data = OraclePriceData {
            has_sufficient_number_of_data_points: false,
            ..valid_oracle_price_data()
        };

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &invalid_oracle_price_data,
            0,
        );

        assert_eq!(result, Err(ErrorCode::OracleInvalid));
        assert_eq!(market.amm, original_amm);
        assert_eq!(user.orders[0].base_asset_amount_filled, 0);
        assert_eq!(user.perp_positions[0].base_asset_amount, 0);
        assert_eq!(user.perp_positions[0].open_bids, 20);
    }

    #[test]
    fn handle_fill_perp_order_amm_rejects_divergent_mark_before_mutation() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(0, PositionDirection::Long, 20);
        user.perp_positions[0] = position_with_open_order(0, PositionDirection::Long, 20);
        let mut market = test_market(0);
        let original_amm = market.amm;
        let divergent_oracle_price_data = OraclePriceData {
            price: i64::try_from(PRICE_PRECISION / 2).unwrap(),
            ..valid_oracle_price_data()
        };

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &divergent_oracle_price_data,
            0,
        );

        assert_eq!(result, Err(ErrorCode::OracleMarkTooDivergent));
        assert_eq!(market.amm, original_amm);
        assert_eq!(user.orders[0].base_asset_amount_filled, 0);
        assert_eq!(user.perp_positions[0].base_asset_amount, 0);
        assert_eq!(user.perp_positions[0].open_bids, 20);
    }

    #[test]
    fn handle_fill_perp_order_amm_rejects_long_fill_above_limit_price_before_mutation() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(0, PositionDirection::Long, 20);
        user.orders[0].price = 1;
        user.perp_positions[0] = position_with_open_order(0, PositionDirection::Long, 20);
        let mut market = test_market(0);
        let original_amm = market.amm;

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &valid_oracle_price_data(),
            0,
        );

        assert_eq!(result, Err(ErrorCode::InvalidFillPrice));
        assert_eq!(market.amm, original_amm);
        assert_eq!(user.orders[0].base_asset_amount_filled, 0);
        assert_eq!(user.perp_positions[0].base_asset_amount, 0);
        assert_eq!(user.perp_positions[0].open_bids, 20);
    }

    #[test]
    fn handle_fill_perp_order_amm_rejects_short_fill_below_limit_price_before_mutation() {
        let mut user = User::default();
        user.open_orders = 1;
        user.orders[0] = open_order(0, PositionDirection::Short, 25);
        user.orders[0].price = 1_000_000_000;
        user.perp_positions[0] = position_with_open_order(0, PositionDirection::Short, 25);
        let mut market = test_market(0);
        let original_amm = market.amm;

        let result = handle_fill_perp_order_amm(
            &mut user,
            Pubkey::default(),
            Pubkey::default(),
            0,
            0,
            &mut market,
            &valid_oracle_price_data(),
            0,
        );

        assert_eq!(result, Err(ErrorCode::InvalidFillPrice));
        assert_eq!(market.amm, original_amm);
        assert_eq!(user.orders[0].base_asset_amount_filled, 0);
        assert_eq!(user.perp_positions[0].base_asset_amount, 0);
        assert_eq!(user.perp_positions[0].open_asks, -25);
    }
}
