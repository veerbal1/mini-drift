use std::cmp::{max, min};

use crate::math::position::PositionUpdateType;
use crate::state::perp_market::PerpMarket;
use crate::{
    error::{ErrorCode, MiniDriftResult},
    math::position::{
        calculate_position_delta, get_new_position_amounts, get_position_update_type,
    },
    state::user::{PerpPosition, PositionDirection},
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PositionDelta {
    pub base_asset_amount: i64,
    pub quote_asset_amount: i64,
}

pub fn update_position_and_market(
    position: &mut PerpPosition,
    delta: &PositionDelta,
    market: &mut PerpMarket,
) -> MiniDriftResult<i64> {
    if market.order_step_size != 0
        && !position
            .base_asset_amount
            .unsigned_abs()
            .is_multiple_of(market.order_step_size)
    {
        return Err(ErrorCode::InvalidPerpPositionDetected);
    };
    let old_position_was_empty =
        position.base_asset_amount == 0 && position.quote_asset_amount == 0;
    let update_type = get_position_update_type(position, delta);
    let (new_base_asset_amount, new_quote_asset_amount) =
        get_new_position_amounts(position, delta)?;
    let (new_quote_entry_amount, new_quote_break_even_amount, realized_pnl) =
        calculate_position_delta(position, delta, &update_type)?;

    let mut number_of_users = market.number_of_users;
    let mut number_of_users_with_base = market.number_of_users_with_base;

    match update_type {
        PositionUpdateType::Open => {
            if old_position_was_empty {
                number_of_users = number_of_users.checked_add(1).ok_or(ErrorCode::MathError)?;
            }
            number_of_users_with_base = number_of_users_with_base
                .checked_add(1)
                .ok_or(ErrorCode::MathError)?;
        }
        PositionUpdateType::Close => {
            if new_base_asset_amount == 0 && new_quote_asset_amount == 0 {
                number_of_users = number_of_users.saturating_sub(1);
            }
            number_of_users_with_base = number_of_users_with_base.saturating_sub(1);
        }
        _ => {}
    }

    position.base_asset_amount = new_base_asset_amount;
    position.quote_asset_amount = new_quote_asset_amount;
    position.quote_entry_amount = new_quote_entry_amount;
    position.quote_break_even_amount = new_quote_break_even_amount;
    market.number_of_users = number_of_users;
    market.number_of_users_with_base = number_of_users_with_base;

    Ok(realized_pnl)
}

pub fn increase_open_bids_and_asks(
    position: &mut PerpPosition,
    direction: &PositionDirection,
    base_asset_amount_unfilled: u64,
) -> MiniDriftResult<()> {
    let base_asset_amount_unfilled_i64 =
        i64::try_from(base_asset_amount_unfilled).map_err(|_| ErrorCode::MathError)?;
    match *direction {
        PositionDirection::Long => {
            position.open_bids = position
                .open_bids
                .checked_add(base_asset_amount_unfilled_i64)
                .ok_or(ErrorCode::MathError)?
        }
        PositionDirection::Short => {
            position.open_asks = position
                .open_asks
                .checked_sub(base_asset_amount_unfilled_i64)
                .ok_or(ErrorCode::MathError)?
        }
    }
    Ok(())
}

pub fn decrease_open_bids_and_asks(
    position: &mut PerpPosition,
    direction: &PositionDirection,
    base_asset_amount: u64,
    update: bool,
) -> MiniDriftResult<()> {
    if !update {
        return Ok(());
    }
    let base_asset_amount_i64 =
        i64::try_from(base_asset_amount).map_err(|_| ErrorCode::MathError)?;
    if *direction == PositionDirection::Long {
        position.open_bids = max(
            position
                .open_bids
                .checked_sub(base_asset_amount_i64)
                .ok_or(ErrorCode::MathError)?,
            0,
        );
        Ok(())
    } else {
        position.open_asks = min(
            position
                .open_asks
                .checked_add(base_asset_amount_i64)
                .ok_or(ErrorCode::MathError)?,
            0,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::Pubkey;

    use crate::{
        controller::orders::{place_perp_order, update_order_after_fill},
        state::{
            order_params::OrderParams,
            user::{OrderType, User},
        },
    };

    use super::*;

    fn position_with_amounts(
        base_asset_amount: i64,
        quote_asset_amount: i64,
        quote_entry_amount: i64,
        quote_break_even_amount: i64,
    ) -> PerpPosition {
        let mut position = PerpPosition::default();
        position.base_asset_amount = base_asset_amount;
        position.quote_asset_amount = quote_asset_amount;
        position.quote_entry_amount = quote_entry_amount;
        position.quote_break_even_amount = quote_break_even_amount;
        position
    }

    fn delta(base_asset_amount: i64, quote_asset_amount: i64) -> PositionDelta {
        PositionDelta {
            base_asset_amount,
            quote_asset_amount,
        }
    }

    #[test]
    fn update_position_and_market_increases_position() {
        let mut position = position_with_amounts(5, -900, -900, -920);
        let mut perp_market = PerpMarket::default();
        perp_market.order_step_size = 1;

        let realized_pnl =
            update_position_and_market(&mut position, &delta(2, -400), &mut perp_market);

        assert_eq!(realized_pnl, Ok(0));
        assert_eq!(position.base_asset_amount, 7);
        assert_eq!(position.quote_asset_amount, -1300);
        assert_eq!(position.quote_entry_amount, -1300);
        assert_eq!(position.quote_break_even_amount, -1320);
    }

    #[test]
    fn update_position_and_market_reduces_long_position_with_profit() {
        let mut position = position_with_amounts(10, -1000, -1000, -1000);
        let mut perp_market = PerpMarket::default();
        perp_market.order_step_size = 1;

        let realized_pnl =
            update_position_and_market(&mut position, &delta(-4, 440), &mut perp_market);

        assert_eq!(realized_pnl, Ok(40));
        assert_eq!(position.base_asset_amount, 6);
        assert_eq!(position.quote_asset_amount, -560);
        assert_eq!(position.quote_entry_amount, -600);
        assert_eq!(position.quote_break_even_amount, -600);
    }

    #[test]
    fn update_position_and_market_closes_position() {
        let mut position = position_with_amounts(10, -1000, -1000, -1000);
        let mut perp_market = PerpMarket::default();
        perp_market.order_step_size = 1;

        let realized_pnl =
            update_position_and_market(&mut position, &delta(-10, 1100), &mut perp_market);

        assert_eq!(realized_pnl, Ok(100));
        assert_eq!(position.base_asset_amount, 0);
        assert_eq!(position.quote_asset_amount, 100);
        assert_eq!(position.quote_entry_amount, 0);
        assert_eq!(position.quote_break_even_amount, 0);
    }

    #[test]
    fn update_position_and_market_flips_position() {
        let mut position = position_with_amounts(5, -500, -500, -500);
        let mut perp_market = PerpMarket::default();
        perp_market.order_step_size = 1;

        let realized_pnl =
            update_position_and_market(&mut position, &delta(-8, 880), &mut perp_market);

        assert_eq!(realized_pnl, Ok(50));
        assert_eq!(position.base_asset_amount, -3);
        assert_eq!(position.quote_asset_amount, 380);
        assert_eq!(position.quote_entry_amount, 330);
        assert_eq!(position.quote_break_even_amount, 330);
    }

    #[test]
    fn update_position_and_market_rejects_invalid_step_size_before_mutation() {
        let mut position = position_with_amounts(5, -500, -500, -500);
        let original_position = position;
        let mut perp_market = PerpMarket {
            order_step_size: 2,
            number_of_users: 7,
            number_of_users_with_base: 3,
            ..PerpMarket::default()
        };
        let original_market = perp_market;

        let realized_pnl =
            update_position_and_market(&mut position, &delta(1, -100), &mut perp_market);

        assert_eq!(realized_pnl, Err(ErrorCode::InvalidPerpPositionDetected));
        assert_eq!(position, original_position);
        assert_eq!(perp_market, original_market);
    }

    #[test]
    fn update_position_and_market_open_increments_user_counters() {
        let mut position = position_with_amounts(0, 0, 0, 0);
        let mut perp_market = PerpMarket {
            order_step_size: 1,
            number_of_users: 7,
            number_of_users_with_base: 3,
            ..PerpMarket::default()
        };

        let realized_pnl =
            update_position_and_market(&mut position, &delta(5, -500), &mut perp_market);

        assert_eq!(realized_pnl, Ok(0));
        assert_eq!(perp_market.number_of_users, 8);
        assert_eq!(perp_market.number_of_users_with_base, 4);
    }

    #[test]
    fn update_position_and_market_close_decrements_user_counters() {
        let mut position = position_with_amounts(5, -500, -500, -500);
        let mut perp_market = PerpMarket {
            order_step_size: 1,
            number_of_users: 7,
            number_of_users_with_base: 3,
            ..PerpMarket::default()
        };

        let realized_pnl =
            update_position_and_market(&mut position, &delta(-5, 500), &mut perp_market);

        assert_eq!(realized_pnl, Ok(0));
        assert_eq!(position.base_asset_amount, 0);
        assert_eq!(position.quote_asset_amount, 0);
        assert_eq!(perp_market.number_of_users, 6);
        assert_eq!(perp_market.number_of_users_with_base, 2);
    }

    #[test]
    fn update_position_and_market_close_counter_decrement_saturates() {
        let mut position = position_with_amounts(5, -500, -500, -500);
        let mut perp_market = PerpMarket {
            order_step_size: 1,
            number_of_users: 0,
            number_of_users_with_base: 0,
            ..PerpMarket::default()
        };

        let realized_pnl =
            update_position_and_market(&mut position, &delta(-5, 500), &mut perp_market);

        assert_eq!(realized_pnl, Ok(0));
        assert_eq!(perp_market.number_of_users, 0);
        assert_eq!(perp_market.number_of_users_with_base, 0);
    }

    #[test]
    fn update_position_and_market_math_error_does_not_half_update_position() {
        let mut position = position_with_amounts(i64::MAX, -1000, -1000, -1000);
        let original_position = position;
        let mut perp_market = PerpMarket::default();
        perp_market.order_step_size = 1;

        let realized_pnl =
            update_position_and_market(&mut position, &delta(1, -100), &mut perp_market);

        assert_eq!(realized_pnl, Err(ErrorCode::MathError));
        assert_eq!(position, original_position);
    }

    #[test]
    fn decrease_open_bids_and_asks_decreases_open_bids_for_long() {
        let mut user = User::default();
        let order_params = OrderParams {
            order_type: OrderType::Market,
            direction: PositionDirection::Long,
            base_asset_amount: 5,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, Pubkey::default(), order_params, 0, 10);
        assert!(res.is_ok());
        assert_eq!(user.perp_positions[0].open_bids, 5);

        // fill the order
        let res = update_order_after_fill(&mut user.orders[0], 4, 100).unwrap();
        assert!(!res);
        let res2 = decrease_open_bids_and_asks(
            &mut user.perp_positions[0],
            &PositionDirection::Long,
            4,
            true,
        );
        assert!(res2.is_ok());
        assert_eq!(user.perp_positions[0].open_bids, 1);
    }

    #[test]
    fn decrease_open_bids_and_asks_increases_open_asks_for_short() {
        let mut user = User::default();
        let order_params = OrderParams {
            order_type: OrderType::Market,
            direction: PositionDirection::Short,
            base_asset_amount: 5,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, Pubkey::default(), order_params, 0, 10);
        assert!(res.is_ok());
        assert_eq!(user.perp_positions[0].open_asks, -5);

        // fill the order
        let res = update_order_after_fill(&mut user.orders[0], 4, 100).unwrap();
        assert!(!res);
        let res2 = decrease_open_bids_and_asks(
            &mut user.perp_positions[0],
            &PositionDirection::Short,
            4,
            true,
        );
        assert!(res2.is_ok());
        assert_eq!(user.perp_positions[0].open_asks, -1);
    }

    #[test]
    fn decrease_open_bids_and_asks_clamps_open_bids_to_zero() {
        let mut position = PerpPosition::default();
        position.open_bids = 3;

        let res = decrease_open_bids_and_asks(&mut position, &PositionDirection::Long, 5, true);

        assert!(res.is_ok());
        assert_eq!(position.open_bids, 0);
    }

    #[test]
    fn decrease_open_bids_and_asks_clamps_open_asks_to_zero() {
        let mut position = PerpPosition::default();
        position.open_asks = -3;

        let res = decrease_open_bids_and_asks(&mut position, &PositionDirection::Short, 5, true);

        assert!(res.is_ok());
        assert_eq!(position.open_asks, 0);
    }

    #[test]
    fn decrease_open_bids_and_asks_does_nothing_when_update_is_false() {
        let mut position = PerpPosition::default();
        position.open_bids = 10;

        let res = decrease_open_bids_and_asks(&mut position, &PositionDirection::Long, 5, false);

        assert!(res.is_ok());
        assert_eq!(position.open_bids, 10);
    }
}
