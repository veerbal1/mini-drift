use crate::state::user::PositionDirection;

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
}
