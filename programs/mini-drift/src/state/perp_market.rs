use anchor_lang::prelude::*;

use crate::state::oracle::{OraclePriceData, OracleSource};

#[derive(
    AnchorSerialize, AnchorDeserialize, Default, Debug, PartialEq, Eq, Clone, Copy, InitSpace,
)]
pub struct Amm {
    pub base_asset_reserve: u128,
    pub quote_asset_reserve: u128,
    pub sqrt_k: u128,
    pub peg_multiplier: u128,
    pub terminal_quote_asset_reserve: u128,
    pub base_asset_amount_with_amm: i128,
    pub base_asset_amount_long: i128,
    pub base_asset_amount_short: i128,
    pub quote_entry_amount_long: i128,
    pub quote_entry_amount_short: i128,
    pub quote_break_even_amount_long: i128,
    pub quote_break_even_amount_short: i128,
    pub concentration_coef: u128,
    pub min_base_asset_reserve: u128,
    pub max_base_asset_reserve: u128,
    pub order_step_size: u64,
    pub base_spread: u32,
    pub max_spread: u32,
    pub long_spread: u32,
    pub short_spread: u32,
}

#[account]
#[derive(Default, Debug, PartialEq, Eq, InitSpace)]
pub struct PerpMarket {
    pub number_of_users: u32,
    pub number_of_users_with_base: u32,
    pub market_index: u16,
    pub amm: Amm,
    pub oracle: Pubkey,
    pub oracle_source: OracleSource,
    pub oracle_max_delay: i64,
    pub oracle_max_confidence: u64,
    pub mock_oracle_price_data: OraclePriceData,
}
