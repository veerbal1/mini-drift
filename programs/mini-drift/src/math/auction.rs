use crate::{
    error::{ErrorCode, MiniDriftResult},
    state::user::{Order, PositionDirection},
};

pub fn calculate_auction_price(order: &Order, current_slot: u64) -> MiniDriftResult<u64> {
    let slots_elapsed = current_slot
        .checked_sub(order.slot)
        .ok_or(ErrorCode::MathError)?;
    let clamped_slot = slots_elapsed.min(order.auction_duration as u64);
    let price_gap = if order.direction == PositionDirection::Short {
        u64::try_from(
            order
                .auction_start_price
                .checked_sub(order.auction_end_price)
                .ok_or(ErrorCode::MathError)?,
        )
        .map_err(|_| ErrorCode::MathError)?
    } else {
        u64::try_from(
            order
                .auction_end_price
                .checked_sub(order.auction_start_price)
                .ok_or(ErrorCode::MathError)?,
        )
        .map_err(|_| ErrorCode::MathError)?
    };

    let elapsed_price_gap = (price_gap
        .checked_mul(clamped_slot)
        .ok_or(ErrorCode::MathError)?)
    .checked_div(order.auction_duration as u64)
    .ok_or(ErrorCode::MathError)?;
    if order.direction == PositionDirection::Long {
        (order.auction_start_price as u64)
            .checked_add(elapsed_price_gap)
            .ok_or(ErrorCode::MathError)
    } else {
        (order.auction_start_price as u64)
            .checked_sub(elapsed_price_gap)
            .ok_or(ErrorCode::MathError)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn calculate_auction_price_returns_long_price_partway_through_auction() {
        let mut order = Order::default();

        order.auction_duration = 10;
        order.auction_start_price = 100;
        order.auction_end_price = 110;
        order.direction = PositionDirection::Long;

        order.slot = 1000;

        let res = calculate_auction_price(&order, 1005);
        assert!(res.is_ok());
        let result = res.unwrap();
        assert_eq!(result, 105);
    }

    #[test]
    pub fn calculate_auction_price_clamps_long_price_after_duration() {
        let mut order = Order::default();

        order.auction_duration = 10;
        order.auction_start_price = 100;
        order.auction_end_price = 110;
        order.direction = PositionDirection::Long;

        order.slot = 1000;

        let res = calculate_auction_price(&order, 1015);
        assert!(res.is_ok());
        let result = res.unwrap();
        assert_eq!(result, 110);
    }

    #[test]
    pub fn calculate_auction_price_returns_short_price_partway_through_auction() {
        let mut order = Order::default();

        order.auction_duration = 10;
        order.auction_start_price = 100;
        order.auction_end_price = 90;
        order.direction = PositionDirection::Short;

        order.slot = 1000;

        let res = calculate_auction_price(&order, 1005);
        assert!(res.is_ok());
        let result = res.unwrap();
        assert_eq!(result, 95);
    }
}
