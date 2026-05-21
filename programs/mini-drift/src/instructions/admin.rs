use crate::{
    error::{ErrorCode, MiniDriftResult},
    math::amm::calculate_mark_price,
    state::oracle::OraclePriceData,
    state::perp_market::{Amm, PerpMarket},
};

#[allow(clippy::too_many_arguments)]
pub fn initialize_perp_market(
    market: &mut PerpMarket,
    market_index: u16,
    initial_base_reserve: u128,
    initial_quote_reserve: u128,
    initial_sqrt_k: u128,
    initial_peg_multiplier: u128,
    concentration_coef: u128,
    min_base_asset_reserve: u128,
    max_base_asset_reserve: u128,
    order_step_size: u64,
) -> MiniDriftResult<()> {
    let terminal_quote_asset_reserve = initial_peg_multiplier
        .checked_mul(initial_sqrt_k)
        .ok_or(ErrorCode::MathError)?;

    market.market_index = market_index;
    market.amm = Amm {
        base_asset_reserve: initial_base_reserve,
        quote_asset_reserve: initial_quote_reserve,
        sqrt_k: initial_sqrt_k,
        peg_multiplier: initial_peg_multiplier,
        terminal_quote_asset_reserve,
        base_asset_amount_with_amm: 0,
        base_asset_amount_long: 0,
        base_asset_amount_short: 0,
        quote_entry_amount_long: 0,
        quote_entry_amount_short: 0,
        quote_break_even_amount_long: 0,
        quote_break_even_amount_short: 0,
        concentration_coef,
        min_base_asset_reserve,
        max_base_asset_reserve,
        order_step_size,
        base_spread: 0,
        max_spread: 0,
        long_spread: 0,
        short_spread: 0,
    };
    let mock_oracle_price = calculate_mark_price(&market.amm)?;
    market.mock_oracle_price_data = OraclePriceData {
        price: mock_oracle_price,
        confidence: 0,
        delay: 0,
        has_sufficient_number_of_data_points: true,
        sequence_id: 0,
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_perp_market_sets_market_and_amm_fields() {
        let mut market = PerpMarket {
            number_of_users: 3,
            number_of_users_with_base: 2,
            ..PerpMarket::default()
        };

        let result =
            initialize_perp_market(&mut market, 7, 1_000, 2_000, 1_500, 12, 3, 900, 1_100, 5);

        assert_eq!(result, Ok(()));
        assert_eq!(market.market_index, 7);
        assert_eq!(market.number_of_users, 3);
        assert_eq!(market.number_of_users_with_base, 2);
        assert_eq!(market.amm.base_asset_reserve, 1_000);
        assert_eq!(market.amm.quote_asset_reserve, 2_000);
        assert_eq!(market.amm.sqrt_k, 1_500);
        assert_eq!(market.amm.peg_multiplier, 12);
        assert_eq!(market.amm.terminal_quote_asset_reserve, 18_000);
        assert_eq!(market.amm.concentration_coef, 3);
        assert_eq!(market.amm.min_base_asset_reserve, 900);
        assert_eq!(market.amm.max_base_asset_reserve, 1_100);
        assert_eq!(market.amm.order_step_size, 5);
        assert_eq!(
            market.mock_oracle_price_data,
            OraclePriceData {
                price: 24,
                confidence: 0,
                delay: 0,
                has_sufficient_number_of_data_points: true,
                sequence_id: 0,
            }
        );
    }

    #[test]
    fn initialize_perp_market_resets_open_interest_fields() {
        let mut market = PerpMarket {
            amm: Amm {
                base_asset_amount_with_amm: 10,
                base_asset_amount_long: 20,
                base_asset_amount_short: -30,
                quote_entry_amount_long: 40,
                quote_entry_amount_short: -50,
                quote_break_even_amount_long: 60,
                quote_break_even_amount_short: -70,
                ..Amm::default()
            },
            ..PerpMarket::default()
        };

        let result = initialize_perp_market(&mut market, 1, 100, 100, 100, 2, 1, 50, 150, 1);

        assert_eq!(result, Ok(()));
        assert_eq!(market.amm.base_asset_amount_with_amm, 0);
        assert_eq!(market.amm.base_asset_amount_long, 0);
        assert_eq!(market.amm.base_asset_amount_short, 0);
        assert_eq!(market.amm.quote_entry_amount_long, 0);
        assert_eq!(market.amm.quote_entry_amount_short, 0);
        assert_eq!(market.amm.quote_break_even_amount_long, 0);
        assert_eq!(market.amm.quote_break_even_amount_short, 0);
    }

    #[test]
    fn initialize_perp_market_errors_on_terminal_quote_overflow() {
        let mut market = PerpMarket::default();
        let original_market = market.clone();

        let result = initialize_perp_market(&mut market, 1, 100, 100, u128::MAX, 2, 1, 50, 150, 1);

        assert_eq!(result, Err(ErrorCode::MathError));
        assert_eq!(market, original_market);
    }
}
