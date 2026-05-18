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

fn calculate_quote_portion(
    quote_asset_amount: i64,
    numerator: i64,
    denominator: i64,
) -> MiniDriftResult<i64> {
    let quote_asset_amount_i128 = i128::from(quote_asset_amount);
    let numerator_i128 = i128::from(numerator);
    let denominator_i128 = i128::from(denominator);
    let quote_portion = quote_asset_amount_i128
        .checked_mul(numerator_i128)
        .ok_or(ErrorCode::MathError)?
        .checked_div(denominator_i128)
        .ok_or(ErrorCode::MathError)?;

    i64::try_from(quote_portion).map_err(|_| ErrorCode::MathError)
}

pub fn calculate_position_delta(
    position: &PerpPosition,
    delta: &PositionDelta,
    update_type: &PositionUpdateType,
) -> MiniDriftResult<(i64, i64, i64)> {
    match update_type {
        PositionUpdateType::Open | PositionUpdateType::Increase => {
            let new_quote_entry_amount = position
                .quote_entry_amount
                .checked_add(delta.quote_asset_amount)
                .ok_or(ErrorCode::MathError)?;
            let new_quote_break_even_amount = position
                .quote_break_even_amount
                .checked_add(delta.quote_asset_amount)
                .ok_or(ErrorCode::MathError)?;

            Ok((new_quote_entry_amount, new_quote_break_even_amount, 0))
        }
        PositionUpdateType::Reduce | PositionUpdateType::Close => {
            let old_size = position
                .base_asset_amount
                .checked_abs()
                .ok_or(ErrorCode::MathError)?;
            let delta_size = delta
                .base_asset_amount
                .checked_abs()
                .ok_or(ErrorCode::MathError)?;
            let removed_quote_entry_amount =
                calculate_quote_portion(position.quote_entry_amount, delta_size, old_size)?;
            let removed_quote_break_even_amount =
                calculate_quote_portion(position.quote_break_even_amount, delta_size, old_size)?;
            let new_quote_entry_amount = position
                .quote_entry_amount
                .checked_sub(removed_quote_entry_amount)
                .ok_or(ErrorCode::MathError)?;
            let new_quote_break_even_amount = position
                .quote_break_even_amount
                .checked_sub(removed_quote_break_even_amount)
                .ok_or(ErrorCode::MathError)?;
            let realized_pnl = removed_quote_entry_amount
                .checked_add(delta.quote_asset_amount)
                .ok_or(ErrorCode::MathError)?;

            Ok((
                new_quote_entry_amount,
                new_quote_break_even_amount,
                realized_pnl,
            ))
        }
        PositionUpdateType::Flip => {
            let old_size = position
                .base_asset_amount
                .checked_abs()
                .ok_or(ErrorCode::MathError)?;
            let delta_size = delta
                .base_asset_amount
                .checked_abs()
                .ok_or(ErrorCode::MathError)?;
            let closing_quote_amount =
                calculate_quote_portion(delta.quote_asset_amount, old_size, delta_size)?;
            let new_leftover_quote_amount = delta
                .quote_asset_amount
                .checked_sub(closing_quote_amount)
                .ok_or(ErrorCode::MathError)?;
            let realized_pnl = position
                .quote_entry_amount
                .checked_add(closing_quote_amount)
                .ok_or(ErrorCode::MathError)?;

            Ok((
                new_leftover_quote_amount,
                new_leftover_quote_amount,
                realized_pnl,
            ))
        }
    }
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

    fn position_with_entry(
        base_asset_amount: i64,
        quote_entry_amount: i64,
        quote_break_even_amount: i64,
    ) -> PerpPosition {
        let mut position = PerpPosition::default();
        position.base_asset_amount = base_asset_amount;
        position.quote_entry_amount = quote_entry_amount;
        position.quote_break_even_amount = quote_break_even_amount;
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

    #[test]
    fn calculates_quote_portion_with_i128_intermediate() {
        assert_eq!(calculate_quote_portion(i64::MAX, 2, 2), Ok(i64::MAX));
    }

    #[test]
    fn calculates_position_delta_for_increase() {
        assert_eq!(
            calculate_position_delta(
                &position_with_entry(5, -900, -920),
                &delta_with_quote(2, -400),
                &PositionUpdateType::Increase,
            ),
            Ok((-1300, -1320, 0))
        );
    }

    #[test]
    fn calculates_position_delta_for_long_reduce_profit() {
        assert_eq!(
            calculate_position_delta(
                &position_with_entry(10, -1000, -1000),
                &delta_with_quote(-4, 440),
                &PositionUpdateType::Reduce,
            ),
            Ok((-600, -600, 40))
        );
    }

    #[test]
    fn calculates_position_delta_for_short_reduce_profit() {
        assert_eq!(
            calculate_position_delta(
                &position_with_entry(-10, 1000, 1000),
                &delta_with_quote(4, -360),
                &PositionUpdateType::Reduce,
            ),
            Ok((600, 600, 40))
        );
    }

    #[test]
    fn calculates_position_delta_for_close() {
        assert_eq!(
            calculate_position_delta(
                &position_with_entry(10, -1000, -1050),
                &delta_with_quote(-10, 1100),
                &PositionUpdateType::Close,
            ),
            Ok((0, 0, 100))
        );
    }

    #[test]
    fn calculates_position_delta_for_flip() {
        assert_eq!(
            calculate_position_delta(
                &position_with_entry(5, -500, -500),
                &delta_with_quote(-8, 880),
                &PositionUpdateType::Flip,
            ),
            Ok((330, 330, 50))
        );
    }
}
