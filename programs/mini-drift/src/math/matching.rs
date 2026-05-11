use crate::state::user::{Order, PositionDirection};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
