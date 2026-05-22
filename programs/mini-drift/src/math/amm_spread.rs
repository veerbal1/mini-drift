use crate::{
    error::{ErrorCode, MiniDriftResult},
    math::constants::PRICE_PRECISION,
    state::user::PositionDirection,
};

#[allow(clippy::too_many_arguments)]
pub fn calculate_spread(
    base_spread: u32,
    max_spread: u32,
    base_asset_amount_with_amm: i128,
    reserve_price: u64,
    oracle_conf_pct: u64,
    mark_std: u64,
    oracle_std: u64,
    long_intensity_volume: u64,
    short_intensity_volume: u64,
    volume_24h: u64,
    base_asset_amount_long: i128,
    base_asset_amount_short: i128,
) -> MiniDriftResult<(u32, u32)> {
    let (long_vol, short_vol) = calculate_long_short_vol_spread(
        mark_std,
        oracle_std,
        long_intensity_volume,
        short_intensity_volume,
        volume_24h,
        reserve_price,
        oracle_conf_pct,
    )?;
    let half_base = base_spread.checked_div(2).ok_or(ErrorCode::MathError)? as u64;
    let mut long_spread = half_base.max(long_vol as u64);
    let mut short_spread = half_base.max(short_vol as u64);

    let inventory_scale = calculate_inventory_scale(
        base_spread,
        base_asset_amount_with_amm,
        base_asset_amount_long,
        base_asset_amount_short,
    )?;
    long_spread = long_spread.saturating_add(inventory_scale as u64);
    short_spread = short_spread.saturating_add(inventory_scale as u64);

    long_spread = long_spread.min(max_spread as u64);
    short_spread = short_spread.min(max_spread as u64);

    Ok((long_spread as u32, short_spread as u32))
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

pub fn calculate_inventory_scale(
    base_spread: u32,
    base_asset_amount_with_amm: i128,
    base_asset_amount_long: i128,
    base_asset_amount_short: i128,
) -> MiniDriftResult<u32> {
    let imbalance = base_asset_amount_with_amm.unsigned_abs();
    let total_oi = base_asset_amount_long
        .unsigned_abs()
        .checked_add(base_asset_amount_short.unsigned_abs())
        .ok_or(ErrorCode::MathError)?;
    if total_oi == 0 {
        return Ok(0);
    }

    let num = (base_spread as u128)
        .checked_mul(imbalance)
        .ok_or(ErrorCode::MathError)?;
    let adjustment = num.checked_div(total_oi).ok_or(ErrorCode::MathError)?;
    let adjustment_u32 = u32::try_from(adjustment).map_err(|_| ErrorCode::MathError)?;
    Ok(adjustment_u32)
}

pub fn calculate_long_short_vol_spread(
    mark_std: u64,
    oracle_std: u64,
    long_intensity_volume: u64,
    short_intensity_volume: u64,
    volume_24h: u64,
    reserve_price: u64,
    oracle_conf_pct: u64,
) -> MiniDriftResult<(u32, u32)> {
    if reserve_price == 0 {
        return Err(ErrorCode::MathError);
    }

    let market_avg_std_pct = u128::from(oracle_std)
        .checked_add(u128::from(mark_std))
        .ok_or(ErrorCode::MathError)?
        .checked_mul(PRICE_PRECISION)
        .ok_or(ErrorCode::MathError)?
        .checked_div(u128::from(reserve_price))
        .ok_or(ErrorCode::MathError)?
        .checked_div(2)
        .ok_or(ErrorCode::MathError)?;
    let std_component = market_avg_std_pct
        .checked_div(4)
        .ok_or(ErrorCode::MathError)?;
    let vol_spread = std_component.max(u128::from(oracle_conf_pct));
    let volume = volume_24h.max(1);
    let long_factor = u128::from(long_intensity_volume)
        .checked_mul(PRICE_PRECISION)
        .ok_or(ErrorCode::MathError)?
        .checked_div(u128::from(volume))
        .ok_or(ErrorCode::MathError)?
        .clamp(PRICE_PRECISION / 100, PRICE_PRECISION);
    let short_factor = u128::from(short_intensity_volume)
        .checked_mul(PRICE_PRECISION)
        .ok_or(ErrorCode::MathError)?
        .checked_div(u128::from(volume))
        .ok_or(ErrorCode::MathError)?
        .clamp(PRICE_PRECISION / 100, PRICE_PRECISION);
    let long_vol_spread = vol_spread
        .checked_mul(long_factor)
        .ok_or(ErrorCode::MathError)?
        .checked_div(PRICE_PRECISION)
        .ok_or(ErrorCode::MathError)?;
    let short_vol_spread = vol_spread
        .checked_mul(short_factor)
        .ok_or(ErrorCode::MathError)?
        .checked_div(PRICE_PRECISION)
        .ok_or(ErrorCode::MathError)?;
    let conf_component = if u128::from(oracle_conf_pct) > PRICE_PRECISION / 400 {
        u128::from(oracle_conf_pct)
    } else {
        u128::from(oracle_conf_pct)
            .checked_div(20)
            .ok_or(ErrorCode::MathError)?
    };
    let long_vol_spread_u32 =
        u32::try_from(long_vol_spread.max(conf_component)).map_err(|_| ErrorCode::MathError)?;
    let short_vol_spread_u32 =
        u32::try_from(short_vol_spread.max(conf_component)).map_err(|_| ErrorCode::MathError)?;

    Ok((long_vol_spread_u32, short_vol_spread_u32))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn calculate_spread_uses_half_base_spread_when_no_adjustments() {
        let result = calculate_spread(400, 500, 0, 1_000_000, 0, 0, 0, 0, 0, 0, 0, 0);

        assert_eq!(result, Ok((200, 200)));
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

    #[test]
    fn calculate_long_short_vol_spread_weights_by_side_intensity() {
        let result = calculate_long_short_vol_spread(0, 0, 500, 100, 1000, 1_000_000, 100);

        assert_eq!(result, Ok((50, 10)));
    }

    #[test]
    fn calculate_long_short_vol_spread_applies_confidence_floor() {
        let result = calculate_long_short_vol_spread(0, 0, 1, 1, 1_000, 1_000_000, 100);

        assert_eq!(result, Ok((5, 5)));
    }
}
