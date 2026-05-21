use crate::{
    error::{ErrorCode, MiniDriftResult},
    state::oracle::OraclePriceData,
};

pub fn get_strict_oracle_price(oracle_price_data: &OraclePriceData) -> MiniDriftResult<(i64, i64)> {
    let confidence_i64 =
        i64::try_from(oracle_price_data.confidence).map_err(|_| ErrorCode::MathError)?;
    let min = oracle_price_data
        .price
        .checked_sub(confidence_i64)
        .ok_or(ErrorCode::MathError)?;

    let max = oracle_price_data
        .price
        .checked_add(confidence_i64)
        .ok_or(ErrorCode::MathError)?;
    Ok((min, max))
}

pub fn is_oracle_valid(
    oracle_price_data: &OraclePriceData,
    max_delay: i64,
    max_confidence: u64,
) -> MiniDriftResult<()> {
    if !oracle_price_data.has_sufficient_number_of_data_points {
        return Err(ErrorCode::OracleInvalid);
    } else if oracle_price_data.delay > max_delay {
        return Err(ErrorCode::OracleStale);
    } else if oracle_price_data.confidence > max_confidence {
        return Err(ErrorCode::OracleInsufficientConfidence);
    }

    Ok(())
}
