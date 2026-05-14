use crate::{
    error::{ErrorCode, MiniDriftResult},
    state::user::{Order, OrderStatus, PositionDirection},
};

pub fn calculate_quote_asset_amount_for_maker_order(
    base_asset_amount: u64,
    fill_price: u64,
    base_decimals: u32,
    maker_direction: PositionDirection,
) -> MiniDriftResult<u64> {
    let base_precision = 10_u128
        .checked_pow(base_decimals)
        .ok_or(ErrorCode::MathError)?;
    let calculated_quote_raw = (base_asset_amount as u128)
        .checked_mul(fill_price as u128)
        .ok_or(ErrorCode::MathError)?;
    if maker_direction == PositionDirection::Long {
        let r = calculated_quote_raw
            .checked_div(base_precision)
            .ok_or(ErrorCode::MathError)?;
        u64::try_from(r).map_err(|_| ErrorCode::MathError)
    } else {
        let num = calculated_quote_raw
            .checked_add(base_precision - 1)
            .ok_or(ErrorCode::MathError)?;
        let r = num
            .checked_div(base_precision)
            .ok_or(ErrorCode::MathError)?;
        u64::try_from(r).map_err(|_| ErrorCode::MathError)
    }
}

pub fn should_expire_order(order: &Order, ts: i64) -> bool {
    if order.max_ts == 0 {
        false
    } else {
        ts > order.max_ts
    }
}

pub fn should_cancel_reduce_only_order(
    order: &Order,
    existing_base_asset_amount: i64,
    step_size: u64,
) -> MiniDriftResult<bool> {
    if order.status != OrderStatus::Open {
        return Ok(false);
    }

    if !order.reduce_only {
        return Ok(false);
    }

    let base_asset_amount_unfilled =
        order.get_base_asset_amount_unfilled(Some(existing_base_asset_amount))?;

    Ok(base_asset_amount_unfilled < step_size)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_quote_asset_amount_for_maker_order_returns_quote_precision_value() {
        let quote_asset_amount = calculate_quote_asset_amount_for_maker_order(
            3_000_000_000,
            120_000_000,
            9,
            PositionDirection::Long,
        )
        .unwrap();

        assert_eq!(quote_asset_amount, 360_000_000);
    }

    #[test]
    fn calculate_quote_asset_amount_for_maker_order_rounds_down_for_maker_long() {
        let quote_asset_amount =
            calculate_quote_asset_amount_for_maker_order(11, 10, 2, PositionDirection::Long)
                .unwrap();

        assert_eq!(quote_asset_amount, 1);
    }

    #[test]
    fn calculate_quote_asset_amount_for_maker_order_rounds_up_for_maker_short() {
        let quote_asset_amount =
            calculate_quote_asset_amount_for_maker_order(11, 10, 2, PositionDirection::Short)
                .unwrap();

        assert_eq!(quote_asset_amount, 2);
    }

    #[test]
    fn should_expire_order_returns_false_when_max_ts_is_zero() {
        let order = Order {
            max_ts: 0,
            ..Order::default()
        };

        assert_eq!(should_expire_order(&order, 10_000), false);
    }

    #[test]
    fn should_expire_order_returns_false_before_or_at_max_ts() {
        let order = Order {
            max_ts: 1000,
            ..Order::default()
        };

        assert_eq!(should_expire_order(&order, 1000), false);
    }

    #[test]
    fn should_expire_order_returns_true_after_max_ts() {
        let order = Order {
            max_ts: 1000,
            ..Order::default()
        };

        assert_eq!(should_expire_order(&order, 1001), true);
    }

    #[test]
    fn should_cancel_reduce_only_order_returns_false_for_non_reduce_only_order() {
        let order = Order {
            reduce_only: false,
            status: OrderStatus::Open,
            direction: PositionDirection::Long,
            base_asset_amount: 10,
            ..Order::default()
        };

        assert_eq!(should_cancel_reduce_only_order(&order, 5, 1), Ok(false));
    }

    #[test]
    fn should_cancel_reduce_only_order_returns_true_when_position_is_zero() {
        let order = Order {
            reduce_only: true,
            status: OrderStatus::Open,
            direction: PositionDirection::Short,
            base_asset_amount: 10,
            ..Order::default()
        };

        assert_eq!(should_cancel_reduce_only_order(&order, 0, 1), Ok(true));
    }

    #[test]
    fn should_cancel_reduce_only_order_returns_true_when_order_same_direction_as_position() {
        let order = Order {
            reduce_only: true,
            status: OrderStatus::Open,
            direction: PositionDirection::Long,
            base_asset_amount: 10,
            ..Order::default()
        };

        assert_eq!(should_cancel_reduce_only_order(&order, 5, 1), Ok(true));
    }

    #[test]
    fn should_cancel_reduce_only_order_returns_false_when_opposite_direction_has_enough_position() {
        let order = Order {
            reduce_only: true,
            status: OrderStatus::Open,
            direction: PositionDirection::Short,
            base_asset_amount: 10,
            ..Order::default()
        };

        assert_eq!(should_cancel_reduce_only_order(&order, 5, 1), Ok(false));
    }

    #[test]
    fn should_cancel_reduce_only_order_returns_true_when_remaining_position_is_smaller_than_step_size(
    ) {
        let order = Order {
            reduce_only: true,
            status: OrderStatus::Open,
            direction: PositionDirection::Short,
            base_asset_amount: 10,
            ..Order::default()
        };

        assert_eq!(should_cancel_reduce_only_order(&order, 5, 10), Ok(true));
    }

    #[test]
    fn should_cancel_reduce_only_order_returns_false_when_order_is_not_open() {
        let order = Order {
            reduce_only: true,
            status: OrderStatus::Filled,
            direction: PositionDirection::Long,
            base_asset_amount: 10,
            ..Order::default()
        };

        assert_eq!(should_cancel_reduce_only_order(&order, 0, 1), Ok(false));
    }
}
