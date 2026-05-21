use crate::{
    error::{ErrorCode, MiniDriftResult},
    math::{
        constants::{PEG_PRECISION, PRICE_PRECISION},
        safe_math::SafeMath,
    },
    state::perp_market::Amm,
};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum SwapDirection {
    Add,
    Subtract,
}

pub fn calculate_mark_price(amm: &Amm) -> MiniDriftResult<i64> {
    if amm.base_asset_reserve == 0 || amm.quote_asset_reserve == 0 || amm.peg_multiplier == 0 {
        return Err(ErrorCode::InvalidAmmDetected);
    }

    let mark_price = amm
        .quote_asset_reserve
        .safe_mul(amm.peg_multiplier)?
        .safe_mul(PRICE_PRECISION)?
        .safe_div(PEG_PRECISION)?
        .safe_div(amm.base_asset_reserve)?;

    i64::try_from(mark_price).map_err(|_| ErrorCode::MathError)
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
            peg_multiplier: PEG_PRECISION,
            min_base_asset_reserve: 50,
            max_base_asset_reserve: 150,
            ..Amm::default()
        }
    }

    fn amm_with_bounds(
        base_asset_reserve: u128,
        quote_asset_reserve: u128,
        min_base_asset_reserve: u128,
        max_base_asset_reserve: u128,
    ) -> Amm {
        Amm {
            base_asset_reserve,
            quote_asset_reserve,
            peg_multiplier: PEG_PRECISION,
            min_base_asset_reserve,
            max_base_asset_reserve,
            ..Amm::default()
        }
    }

    #[test]
    fn calculate_mark_price_uses_reserves_and_peg() {
        let amm = Amm {
            base_asset_reserve: 100,
            quote_asset_reserve: 120,
            peg_multiplier: PEG_PRECISION,
            ..Amm::default()
        };

        let mark_price = calculate_mark_price(&amm);

        assert_eq!(mark_price, Ok(1_200_000));
    }

    #[test]
    fn calculate_mark_price_rejects_invalid_amm() {
        let amm = Amm {
            base_asset_reserve: 0,
            quote_asset_reserve: 120,
            peg_multiplier: PEG_PRECISION,
            ..Amm::default()
        };

        let mark_price = calculate_mark_price(&amm);

        assert_eq!(mark_price, Err(ErrorCode::InvalidAmmDetected));
    }

    #[test]
    fn update_amm_reserves_subtract_vector_buy_long() {
        let mut amm = amm_with_bounds(1000, 1000, 1, 2000);

        let quote_asset_amount = update_amm_reserves(&mut amm, 100, SwapDirection::Subtract);

        assert_eq!(quote_asset_amount, Ok(111));
        assert_eq!(amm.base_asset_reserve, 900);
        assert_eq!(amm.quote_asset_reserve, 1111);
    }

    #[test]
    fn update_amm_reserves_add_vector_sell_short() {
        let mut amm = amm_with_bounds(1000, 1000, 1, 2000);

        let quote_asset_amount = update_amm_reserves(&mut amm, 100, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Ok(91));
        assert_eq!(amm.base_asset_reserve, 1100);
        assert_eq!(amm.quote_asset_reserve, 909);
    }

    #[test]
    fn update_amm_reserves_succeeds_at_exact_min_base_bound() {
        let mut amm = amm_with_bounds(100, 100, 50, 150);

        let quote_asset_amount = update_amm_reserves(&mut amm, 50, SwapDirection::Subtract);

        assert_eq!(quote_asset_amount, Ok(100));
        assert_eq!(amm.base_asset_reserve, 50);
        assert_eq!(amm.quote_asset_reserve, 200);
    }

    #[test]
    fn update_amm_reserves_succeeds_at_exact_max_base_bound() {
        let mut amm = amm_with_bounds(100, 100, 50, 150);

        let quote_asset_amount = update_amm_reserves(&mut amm, 50, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Ok(34));
        assert_eq!(amm.base_asset_reserve, 150);
        assert_eq!(amm.quote_asset_reserve, 66);
    }

    #[test]
    fn update_amm_reserves_rejects_below_min_base_without_mutation() {
        let mut amm = amm_with_bounds(100, 100, 50, 150);
        let original_amm = amm;

        let quote_asset_amount = update_amm_reserves(&mut amm, 51, SwapDirection::Subtract);

        assert_eq!(quote_asset_amount, Err(ErrorCode::InvalidAmmDetected));
        assert_eq!(amm, original_amm);
    }

    #[test]
    fn update_amm_reserves_rejects_above_max_base_without_mutation() {
        let mut amm = amm_with_bounds(100, 100, 50, 150);
        let original_amm = amm;

        let quote_asset_amount = update_amm_reserves(&mut amm, 51, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Err(ErrorCode::InvalidAmmDetected));
        assert_eq!(amm, original_amm);
    }

    #[test]
    fn update_amm_reserves_rejects_zero_quote_reserve_without_mutation() {
        let mut amm = amm_with_bounds(100, 0, 50, 150);
        let original_amm = amm;

        let quote_asset_amount = update_amm_reserves(&mut amm, 10, SwapDirection::Add);

        assert_eq!(quote_asset_amount, Err(ErrorCode::InvalidAmmDetected));
        assert_eq!(amm, original_amm);
    }

    #[test]
    fn update_amm_reserves_rejects_invariant_overflow_without_mutation() {
        let mut amm = amm_with_bounds(u128::MAX, 2, 1, u128::MAX);
        let original_amm = amm;

        let quote_asset_amount = update_amm_reserves(&mut amm, 1, SwapDirection::Subtract);

        assert_eq!(quote_asset_amount, Err(ErrorCode::MathError));
        assert_eq!(amm, original_amm);
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
