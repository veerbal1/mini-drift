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
}
