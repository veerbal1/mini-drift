use crate::{
    error::{ErrorCode, MiniDriftResult},
    state::oracle::OraclePriceData,
};

const BPS_DENOMINATOR: i128 = 10_000;
const MAX_MARK_ORACLE_DIVERGENCE_BPS: i128 = 1_000;

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

pub fn validate_mark_oracle_divergence(
    mark_price: i64,
    oracle_price_data: &OraclePriceData,
) -> MiniDriftResult<()> {
    if mark_price <= 0 || oracle_price_data.price <= 0 {
        return Err(ErrorCode::OracleInvalid);
    }

    let mark_price = i128::from(mark_price);
    let oracle_price = i128::from(oracle_price_data.price);
    let divergence = if mark_price > oracle_price {
        mark_price
            .checked_sub(oracle_price)
            .ok_or(ErrorCode::MathError)?
    } else {
        oracle_price
            .checked_sub(mark_price)
            .ok_or(ErrorCode::MathError)?
    };
    let max_divergence = oracle_price
        .checked_mul(MAX_MARK_ORACLE_DIVERGENCE_BPS)
        .ok_or(ErrorCode::MathError)?
        .checked_div(BPS_DENOMINATOR)
        .ok_or(ErrorCode::MathError)?;

    if divergence > max_divergence {
        return Err(ErrorCode::OracleMarkTooDivergent);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oracle_price_data() -> OraclePriceData {
        OraclePriceData {
            price: 100,
            confidence: 10,
            delay: 5,
            has_sufficient_number_of_data_points: true,
            sequence_id: 1,
        }
    }

    #[test]
    fn is_oracle_valid_accepts_fresh_oracle() {
        let result = is_oracle_valid(&oracle_price_data(), 5, 10);

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn is_oracle_valid_rejects_stale_oracle() {
        let result = is_oracle_valid(&oracle_price_data(), 4, 10);

        assert_eq!(result, Err(ErrorCode::OracleStale));
    }

    #[test]
    fn is_oracle_valid_rejects_high_confidence_oracle() {
        let result = is_oracle_valid(&oracle_price_data(), 5, 9);

        assert_eq!(result, Err(ErrorCode::OracleInsufficientConfidence));
    }

    #[test]
    fn is_oracle_valid_rejects_insufficient_data_points() {
        let oracle_price_data = OraclePriceData {
            has_sufficient_number_of_data_points: false,
            ..oracle_price_data()
        };

        let result = is_oracle_valid(&oracle_price_data, 5, 10);

        assert_eq!(result, Err(ErrorCode::OracleInvalid));
    }

    #[test]
    fn get_strict_oracle_price_returns_confidence_interval() {
        let result = get_strict_oracle_price(&oracle_price_data());

        assert_eq!(result, Ok((90, 110)));
    }

    #[test]
    fn validate_mark_oracle_divergence_accepts_mark_within_guard() {
        let result = validate_mark_oracle_divergence(110, &oracle_price_data());

        assert_eq!(result, Ok(()));
    }

    #[test]
    fn validate_mark_oracle_divergence_rejects_mark_outside_guard() {
        let result = validate_mark_oracle_divergence(111, &oracle_price_data());

        assert_eq!(result, Err(ErrorCode::OracleMarkTooDivergent));
    }
}
