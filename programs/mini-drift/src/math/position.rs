use crate::{
    controller::position::PositionDelta,
    error::{ErrorCode, MiniDriftResult},
    state::user::PerpPosition,
};

#[derive(Debug, PartialEq, Eq)]
pub enum PositionUpdateType {
    Open,
    Increase,
    Reduce,
    Close,
    Flip,
}

pub fn get_position_update_type(
    position: &PerpPosition,
    delta: &PositionDelta,
) -> PositionUpdateType {
    if position.base_asset_amount == 0 {
        PositionUpdateType::Open
    } else if (position.base_asset_amount > 0 && delta.base_asset_amount > 0)
        || (position.base_asset_amount < 0 && delta.base_asset_amount < 0)
    {
        PositionUpdateType::Increase
    } else {
        let old_size = position.base_asset_amount.abs();
        let delta_size = delta.base_asset_amount.abs();

        if old_size > delta_size {
            PositionUpdateType::Reduce
        } else if old_size == delta_size {
            PositionUpdateType::Close
        } else {
            PositionUpdateType::Flip
        }
    }
}

pub fn get_new_position_amounts(
    position: &PerpPosition,
    delta: &PositionDelta,
) -> MiniDriftResult<(i64, i64)> {
    let new_base_asset_amount = position
        .base_asset_amount
        .checked_add(delta.base_asset_amount)
        .ok_or(ErrorCode::MathError)?;
    let new_quote_asset_amount = position
        .quote_asset_amount
        .checked_add(delta.quote_asset_amount)
        .ok_or(ErrorCode::MathError)?;

    Ok((new_base_asset_amount, new_quote_asset_amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn position(base_asset_amount: i64) -> PerpPosition {
        let mut position = PerpPosition::default();
        position.base_asset_amount = base_asset_amount;
        position
    }

    fn position_with_quote(base_asset_amount: i64, quote_asset_amount: i64) -> PerpPosition {
        let mut position = PerpPosition::default();
        position.base_asset_amount = base_asset_amount;
        position.quote_asset_amount = quote_asset_amount;
        position
    }

    fn delta(base_asset_amount: i64) -> PositionDelta {
        PositionDelta {
            base_asset_amount,
            ..PositionDelta::default()
        }
    }

    fn delta_with_quote(base_asset_amount: i64, quote_asset_amount: i64) -> PositionDelta {
        PositionDelta {
            base_asset_amount,
            quote_asset_amount,
        }
    }

    #[test]
    fn gets_position_update_type() {
        use PositionUpdateType::*;

        let cases = [
            (0, 5, Open),
            (5, 2, Increase),
            (5, -2, Reduce),
            (5, -5, Close),
            (5, -8, Flip),
        ];

        for (old_base_asset_amount, delta_base_asset_amount, expected) in cases {
            assert_eq!(
                get_position_update_type(
                    &position(old_base_asset_amount),
                    &delta(delta_base_asset_amount),
                ),
                expected,
                "old {old_base_asset_amount}, delta {delta_base_asset_amount}"
            );
        }
    }

    #[test]
    fn gets_new_position_amounts() {
        assert_eq!(
            get_new_position_amounts(&position_with_quote(5, -900), &delta_with_quote(-2, 400)),
            Ok((3, -500))
        );
    }

    #[test]
    fn get_new_position_amounts_errors_on_base_overflow() {
        assert_eq!(
            get_new_position_amounts(&position(i64::MAX), &delta(1)),
            Err(ErrorCode::MathError)
        );
    }
}
