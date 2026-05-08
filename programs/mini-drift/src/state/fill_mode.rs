use crate::{
    error::{ErrorCode, MiniDriftResult},
    math::auction::calculate_auction_price,
    state::user::Order,
};

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub enum FillMode {
    Fill,
    PlaceAndMake,
    PlaceAndTake(bool, u8),
    Liquidation,
}

impl FillMode {
    pub fn is_liquidation(&self) -> bool {
        self == &FillMode::Liquidation
    }

    pub fn is_ioc(&self) -> bool {
        matches!(self, FillMode::PlaceAndTake(true, _))
    }

    pub fn get_limit_price(&self, order: &Order) -> MiniDriftResult<Option<u64>> {
        match self {
            FillMode::Fill => order.get_limit_price(),
            FillMode::Liquidation => order.get_limit_price(),
            FillMode::PlaceAndMake => order.get_limit_price(),
            FillMode::PlaceAndTake(_, percentage) => {
                if !order.has_auction() {
                    order.get_limit_price()
                } else {
                    let capped_percentage = percentage.min(&100);
                    let duration = order.auction_duration as u64;
                    let percentage = *capped_percentage as u64;
                    let calc_slot = duration
                        .checked_mul(percentage)
                        .ok_or(ErrorCode::MathError)?
                        .checked_div(100)
                        .ok_or(ErrorCode::MathError)?;
                    let pretended_slots = order
                        .slot
                        .checked_add(calc_slot as u64)
                        .ok_or(ErrorCode::MathError)?;
                    let auction_price = calculate_auction_price(order, pretended_slots)?;
                    Ok(Some(auction_price))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    pub fn fill_mode_is_liquidation_returns_true_for_liquidation() {
        let mode = FillMode::Liquidation;
        assert!(mode.is_liquidation());
    }

    #[test]
    pub fn fill_mode_is_liquidation_returns_false_for_normal_fill() {
        let mode = FillMode::Fill;
        assert!(!mode.is_liquidation());
    }

    #[test]
    pub fn fill_mode_is_ioc_returns_true_for_place_and_take_true() {
        let mode = FillMode::PlaceAndTake(true, 50);
        assert!(mode.is_ioc());
    }

    #[test]
    pub fn fill_mode_is_ioc_returns_false_for_place_and_take_false() {
        let mode = FillMode::PlaceAndTake(false, 50);
        assert!(!mode.is_ioc());
    }

    #[test]
    pub fn fill_mode_is_ioc_returns_false_for_normal_fill() {
        let mode = FillMode::Fill;
        assert!(!mode.is_ioc());
    }

    #[test]
    fn fill_mode_get_limit_price_uses_order_limit_price_for_fill() {
        let mut order = Order::default();
        order.price = 100;

        let mode = FillMode::Fill;
        let res = mode.get_limit_price(&order).unwrap().unwrap();
        assert_eq!(res, 100);
    }

    #[test]
    fn fill_mode_get_limit_price_uses_place_and_take_auction_percentage() {
        use crate::state::user::PositionDirection;

        let mut order = Order::default();
        order.slot = 1000;
        order.auction_duration = 40;
        order.auction_start_price = 100;
        order.auction_end_price = 140;
        order.direction = PositionDirection::Long;

        let mode = FillMode::PlaceAndTake(false, 25);

        let result = mode.get_limit_price(&order).unwrap();

        assert_eq!(result, Some(110));
    }

    #[test]
    fn fill_mode_get_limit_price_caps_place_and_take_percentage_at_100() {
        use crate::state::user::PositionDirection;

        let mut order = Order::default();
        order.slot = 1000;
        order.auction_duration = 40;
        order.auction_start_price = 100;
        order.auction_end_price = 140;
        order.direction = PositionDirection::Long;

        let mode = FillMode::PlaceAndTake(false, 150);

        let result = mode.get_limit_price(&order).unwrap();

        assert_eq!(result, Some(140));
    }

    #[test]
    fn fill_mode_get_limit_price_falls_back_to_order_limit_price_when_place_and_take_has_no_auction(
    ) {
        let mut order = Order::default();
        order.price = 120;
        order.auction_duration = 0;

        let mode = FillMode::PlaceAndTake(false, 50);

        let result = mode.get_limit_price(&order).unwrap();

        assert_eq!(result, Some(120));
    }
}
