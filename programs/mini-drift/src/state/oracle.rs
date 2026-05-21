use anchor_lang::prelude::*;

#[derive(
    AnchorSerialize, AnchorDeserialize, Default, Debug, PartialEq, Eq, Clone, Copy, InitSpace,
)]
pub enum OracleSource {
    #[default]
    Pyth,
    Switchboard,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Default, Debug, PartialEq, Eq, Clone, Copy, InitSpace,
)]
pub struct OraclePriceData {
    pub price: i64,
    pub confidence: u64,
    pub delay: i64,
    pub has_sufficient_number_of_data_points: bool,
    pub sequence_id: u64,
}

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct HistoricalOracleData {
    pub last_oracle_price_twap: i64,
    pub last_oracle_price_twap_5min: i64,
    pub last_oracle_price_twap_ts: i64,
}
