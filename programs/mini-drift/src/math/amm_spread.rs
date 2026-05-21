use crate::{
    error::{ErrorCode, MiniDriftResult},
    math::constants::PRICE_PRECISION,
    state::{perp_market::Amm, user::PositionDirection},
};

pub fn calculate_spread(amm: &Amm) -> MiniDriftResult<(u32, u32)> {
    Ok((amm.long_spread, amm.short_spread))
}

pub fn apply_spread_to_quote(
    quote_asset_amount: u64,
    long_spread: u32,
    short_spread: u32,
    direction: PositionDirection,
) -> MiniDriftResult<u64> {
    let quote_asset_amount_u128 = quote_asset_amount as u128;
    match direction {
        PositionDirection::Long => {
            let res_raw = quote_asset_amount_u128
                .checked_mul(long_spread as u128)
                .ok_or(ErrorCode::MathError)?;
            let res = res_raw
                .checked_div(PRICE_PRECISION)
                .ok_or(ErrorCode::MathError)?;
            let quote = quote_asset_amount_u128
                .checked_add(res)
                .ok_or(ErrorCode::MathError)?;
            let quote_u64 = u64::try_from(quote).map_err(|_| ErrorCode::MathError)?;
            Ok(quote_u64)
        }
        PositionDirection::Short => {
            let res_raw = quote_asset_amount_u128
                .checked_mul(short_spread as u128)
                .ok_or(ErrorCode::MathError)?;
            let res = res_raw
                .checked_div(PRICE_PRECISION)
                .ok_or(ErrorCode::MathError)?;
            let quote = quote_asset_amount_u128
                .checked_sub(res)
                .ok_or(ErrorCode::MathError)?;
            let quote_u64 = u64::try_from(quote).map_err(|_| ErrorCode::MathError)?;
            Ok(quote_u64)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_spread_returns_stored_values() {
        let amm = Amm {
            base_spread: 100,
            max_spread: 500,
            long_spread: 200,
            short_spread: 300,
            ..Amm::default()
        };

        let result = calculate_spread(&amm);

        assert_eq!(result, Ok((200, 300)));
    }

    #[test]
    fn apply_spread_long_adds_to_quote() {
        let result = apply_spread_to_quote(1000, 50_000, 0, PositionDirection::Long);

        assert_eq!(result, Ok(1050));
    }

    #[test]
    fn apply_spread_short_subtracts_from_quote() {
        let result = apply_spread_to_quote(1000, 0, 50_000, PositionDirection::Short);

        assert_eq!(result, Ok(950));
    }

    #[test]
    fn apply_spread_zero_changes_nothing() {
        let result = apply_spread_to_quote(1000, 0, 0, PositionDirection::Long);

        assert_eq!(result, Ok(1000));
    }
}
