use crate::{
    error::{ErrorCode, MiniDriftResult},
    math::safe_math::SafeMath,
    state::perp_market::Amm,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SwapDirection {
    Add,
    Subtract,
}

pub fn swap_base_asset(
    amm: &Amm,
    swap_amount: u64,
    direction: SwapDirection,
) -> MiniDriftResult<u64> {
    if amm.base_asset_reserve == 0 || amm.quote_asset_reserve == 0 {
        return Err(ErrorCode::InvalidAmmDetected);
    }

    let swap_amount = u128::from(swap_amount);
    let new_base_asset_reserve = match direction {
        SwapDirection::Add => amm.base_asset_reserve.safe_add(swap_amount)?,
        SwapDirection::Subtract => amm.base_asset_reserve.safe_sub(swap_amount)?,
    };

    if new_base_asset_reserve < amm.min_base_asset_reserve
        || new_base_asset_reserve > amm.max_base_asset_reserve
        || new_base_asset_reserve == 0
    {
        return Err(ErrorCode::InvalidAmmDetected);
    }

    let invariant = amm.base_asset_reserve.safe_mul(amm.quote_asset_reserve)?;
    let new_quote_asset_reserve = invariant.safe_div(new_base_asset_reserve)?;
    let quote_asset_amount = if new_quote_asset_reserve > amm.quote_asset_reserve {
        new_quote_asset_reserve.safe_sub(amm.quote_asset_reserve)?
    } else {
        amm.quote_asset_reserve.safe_sub(new_quote_asset_reserve)?
    };

    u64::try_from(quote_asset_amount).map_err(|_| ErrorCode::MathError)
}

pub fn update_amm_reserves(
    amm: &mut Amm,
    swap_amount: u64,
    direction: SwapDirection,
) -> MiniDriftResult<u64> {
    let quote_asset_amount = swap_base_asset(amm, swap_amount, direction)?;
    let swap_amount_u128 = u128::from(swap_amount);
    let quote_asset_amount_u128 = u128::from(quote_asset_amount);
    let swap_amount_i128 = i128::from(swap_amount);

    let (
        new_base_asset_reserve,
        new_quote_asset_reserve,
        new_base_asset_amount_with_amm,
        new_base_asset_amount_long,
        new_base_asset_amount_short,
    ) = match direction {
        SwapDirection::Add => (
            amm.base_asset_reserve.safe_add(swap_amount_u128)?,
            amm.quote_asset_reserve.safe_sub(quote_asset_amount_u128)?,
            amm.base_asset_amount_with_amm
                .checked_sub(swap_amount_i128)
                .ok_or(ErrorCode::MathError)?,
            amm.base_asset_amount_long,
            amm.base_asset_amount_short
                .checked_sub(swap_amount_i128)
                .ok_or(ErrorCode::MathError)?,
        ),
        SwapDirection::Subtract => (
            amm.base_asset_reserve.safe_sub(swap_amount_u128)?,
            amm.quote_asset_reserve.safe_add(quote_asset_amount_u128)?,
            amm.base_asset_amount_with_amm
                .checked_add(swap_amount_i128)
                .ok_or(ErrorCode::MathError)?,
            amm.base_asset_amount_long
                .checked_add(swap_amount_i128)
                .ok_or(ErrorCode::MathError)?,
            amm.base_asset_amount_short,
        ),
    };

    amm.base_asset_reserve = new_base_asset_reserve;
    amm.quote_asset_reserve = new_quote_asset_reserve;
    amm.base_asset_amount_with_amm = new_base_asset_amount_with_amm;
    amm.base_asset_amount_long = new_base_asset_amount_long;
    amm.base_asset_amount_short = new_base_asset_amount_short;

    Ok(quote_asset_amount)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn amm() -> Amm {
        Amm {
            base_asset_reserve: 100,
            quote_asset_reserve: 100,
            min_base_asset_reserve: 50,
            max_base_asset_reserve: 150,
            ..Amm::default()
        }
    }

    #[test]
    fn swap_base_asset_add_returns_quote_delta() {
        let quote_asset_amount = swap_base_asset(&amm(), 25, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Ok(20));
    }

    #[test]
    fn swap_base_asset_subtract_returns_quote_delta() {
        let quote_asset_amount = swap_base_asset(&amm(), 20, SwapDirection::Subtract);

        assert_eq!(quote_asset_amount, Ok(25));
    }

    #[test]
    fn swap_base_asset_errors_when_new_base_is_below_min() {
        let quote_asset_amount = swap_base_asset(&amm(), 60, SwapDirection::Subtract);

        assert_eq!(quote_asset_amount, Err(ErrorCode::InvalidAmmDetected));
    }

    #[test]
    fn swap_base_asset_errors_when_new_base_is_above_max() {
        let quote_asset_amount = swap_base_asset(&amm(), 60, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Err(ErrorCode::InvalidAmmDetected));
    }

    #[test]
    fn swap_base_asset_errors_when_reserves_are_zero() {
        let amm = Amm {
            base_asset_reserve: 0,
            quote_asset_reserve: 100,
            min_base_asset_reserve: 50,
            max_base_asset_reserve: 150,
            ..Amm::default()
        };

        let quote_asset_amount = swap_base_asset(&amm, 10, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Err(ErrorCode::InvalidAmmDetected));
    }

    #[test]
    fn update_amm_reserves_add_updates_reserves_and_short_exposure() {
        let mut amm = amm();

        let quote_asset_amount = update_amm_reserves(&mut amm, 25, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Ok(20));
        assert_eq!(amm.base_asset_reserve, 125);
        assert_eq!(amm.quote_asset_reserve, 80);
        assert_eq!(amm.base_asset_amount_with_amm, -25);
        assert_eq!(amm.base_asset_amount_long, 0);
        assert_eq!(amm.base_asset_amount_short, -25);
    }

    #[test]
    fn update_amm_reserves_subtract_updates_reserves_and_long_exposure() {
        let mut amm = amm();

        let quote_asset_amount = update_amm_reserves(&mut amm, 20, SwapDirection::Subtract);

        assert_eq!(quote_asset_amount, Ok(25));
        assert_eq!(amm.base_asset_reserve, 80);
        assert_eq!(amm.quote_asset_reserve, 125);
        assert_eq!(amm.base_asset_amount_with_amm, 20);
        assert_eq!(amm.base_asset_amount_long, 20);
        assert_eq!(amm.base_asset_amount_short, 0);
    }

    #[test]
    fn update_amm_reserves_does_not_mutate_when_swap_is_invalid() {
        let mut amm = amm();
        let original_amm = amm;

        let quote_asset_amount = update_amm_reserves(&mut amm, 60, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Err(ErrorCode::InvalidAmmDetected));
        assert_eq!(amm, original_amm);
    }
}
