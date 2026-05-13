use crate::{
    error::{ErrorCode, MiniDriftResult},
    state::user::{Order, PositionDirection},
};

pub fn do_orders_cross(
    maker_direction: PositionDirection,
    maker_price: u64,
    taker_price: u64,
) -> bool {
    if maker_direction == PositionDirection::Long {
        taker_price <= maker_price
    } else {
        taker_price >= maker_price
    }
}

pub fn are_orders_same_market_but_different_sides(
    maker_order: &Order,
    taker_order: &Order,
) -> bool {
    if maker_order.market_index != taker_order.market_index {
        false
    } else if maker_order.direction == taker_order.direction {
        false
    } else {
        true
    }
}

pub fn is_maker_for_taker(
    maker_order: &Order,
    taker_order: &Order,
    slot: u64,
) -> MiniDriftResult<bool> {
    if taker_order.post_only {
        return Ok(false);
    } else if !maker_order.is_resting_limit_order(slot)? {
        return Ok(false);
    } else if !taker_order.is_resting_limit_order(slot)? {
        return Ok(true);
    } else if maker_order.post_only {
        return Ok(true);
    } else {
        return Ok(maker_order
            .slot
            .checked_add(maker_order.auction_duration as u64)
            .ok_or(ErrorCode::MathError)?
            <= taker_order
                .slot
                .checked_add(taker_order.auction_duration as u64)
                .ok_or(ErrorCode::MathError)?);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::user::OrderType;

    #[test]
    fn do_orders_cross_returns_true_when_maker_short_price_is_inside_taker_buy_wall() {
        assert_eq!(do_orders_cross(PositionDirection::Short, 110, 112), true)
    }

    #[test]
    fn do_orders_cross_returns_false_when_maker_short_price_is_above_taker_buy_wall() {
        assert_eq!(do_orders_cross(PositionDirection::Short, 110, 109), false)
    }

    #[test]
    fn do_orders_cross_returns_true_when_maker_long_price_is_inside_taker_sell_wall() {
        assert_eq!(do_orders_cross(PositionDirection::Long, 110, 109), true)
    }

    #[test]
    fn do_orders_cross_returns_false_when_maker_long_price_is_below_taker_sell_wall() {
        assert_eq!(do_orders_cross(PositionDirection::Long, 110, 112), false)
    }

    #[test]
    fn are_orders_same_market_but_different_sides_returns_true_for_same_market_opposite_side() {
        let maker_order = Order {
            market_index: 1,
            direction: PositionDirection::Long,
            ..Order::default()
        };
        let taker_order = Order {
            market_index: 1,
            direction: PositionDirection::Short,
            ..Order::default()
        };

        assert_eq!(
            are_orders_same_market_but_different_sides(&maker_order, &taker_order),
            true
        );
    }

    #[test]
    fn are_orders_same_market_but_different_sides_returns_false_for_different_market() {
        let maker_order = Order {
            market_index: 1,
            direction: PositionDirection::Long,
            ..Order::default()
        };
        let taker_order = Order {
            market_index: 2,
            direction: PositionDirection::Short,
            ..Order::default()
        };

        assert_eq!(
            are_orders_same_market_but_different_sides(&maker_order, &taker_order),
            false
        );
    }

    #[test]
    fn are_orders_same_market_but_different_sides_returns_false_for_same_direction() {
        let maker_order = Order {
            market_index: 1,
            direction: PositionDirection::Long,
            ..Order::default()
        };
        let taker_order = Order {
            market_index: 1,
            direction: PositionDirection::Long,
            ..Order::default()
        };

        assert_eq!(
            are_orders_same_market_but_different_sides(&maker_order, &taker_order),
            false
        );
    }

    #[test]
    fn is_maker_for_taker_returns_false_when_taker_is_post_only() {
        let maker = Order {
            order_type: OrderType::Limit,
            post_only: true,
            slot: 0,
            auction_duration: 0,
            ..Order::default()
        };
        let taker = Order {
            post_only: true,
            ..Order::default()
        };

        assert_eq!(is_maker_for_taker(&maker, &taker, 0), Ok(false));
    }

    #[test]
    fn is_maker_for_taker_returns_false_when_maker_is_not_resting_limit_order() {
        let maker = Order {
            order_type: OrderType::Market,
            ..Order::default()
        };
        let taker = Order {
            order_type: OrderType::Limit,
            post_only: false,
            slot: 0,
            auction_duration: 0,
            ..Order::default()
        };

        assert_eq!(is_maker_for_taker(&maker, &taker, 100), Ok(false));
    }

    #[test]
    fn is_maker_for_taker_returns_true_when_maker_is_resting_and_taker_is_not_resting() {
        let maker = Order {
            order_type: OrderType::Limit,
            post_only: true,
            slot: 0,
            auction_duration: 0,
            ..Order::default()
        };
        let taker = Order {
            order_type: OrderType::Market,
            post_only: false,
            ..Order::default()
        };

        assert_eq!(is_maker_for_taker(&maker, &taker, 0), Ok(true));
    }

    #[test]
    fn is_maker_for_taker_returns_true_when_maker_is_post_only() {
        let maker = Order {
            order_type: OrderType::Limit,
            post_only: true,
            slot: 100,
            auction_duration: 10,
            ..Order::default()
        };
        let taker = Order {
            order_type: OrderType::Limit,
            post_only: false,
            slot: 150,
            auction_duration: 10,
            ..Order::default()
        };

        let slot = 200;
        assert_eq!(is_maker_for_taker(&maker, &taker, slot), Ok(true));
    }

    #[test]
    fn is_maker_for_taker_returns_true_when_maker_ready_time_is_older() {
        let maker = Order {
            order_type: OrderType::Limit,
            post_only: false,
            slot: 100,
            auction_duration: 10,
            ..Order::default()
        };
        let taker = Order {
            order_type: OrderType::Limit,
            post_only: false,
            slot: 150,
            auction_duration: 10,
            ..Order::default()
        };

        let slot = 200;
        assert_eq!(is_maker_for_taker(&maker, &taker, slot), Ok(true));
    }

    #[test]
    fn is_maker_for_taker_returns_false_when_maker_ready_time_is_newer() {
        let maker = Order {
            order_type: OrderType::Limit,
            post_only: false,
            slot: 150,
            auction_duration: 10,
            ..Order::default()
        };
        let taker = Order {
            order_type: OrderType::Limit,
            post_only: false,
            slot: 100,
            auction_duration: 10,
            ..Order::default()
        };

        let slot = 200;
        assert_eq!(is_maker_for_taker(&maker, &taker, slot), Ok(false));
    }
}
