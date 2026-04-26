// protocol-v2-master/programs/drift/src/state/user.rs

use anchor_lang::prelude::*;

#[derive(Default, Debug, PartialEq, Eq, Clone, Copy)]
pub struct PerpPosition {
    /// Market like SOL-PERP, BTC-PERP
    pub market_index: u16,

    /// Base Asset like SOL, BTC, ETH. It also info about long or short that's why it is i64 (signed)
    /// precision: BASE_PRECISION
    pub base_asset_amount: i64,

    /// Quote Asset like USDC. It always have opp. sign from base_asset_amount
    /// precision: QUOTE_PRECISION
    pub quote_asset_amount: i64,

    /// Quote entry means at what price I opened the postition. I'll be used to calculate the PnL based on Position direction
    /// precision: QUOTE_PRECISION
    pub quote_entry_amount: i64,

    /// It is an amount that is after cutting, excluding all the fees. It include opening position, closing position fee in it. It include closing fee even in the beginning.
    /// precision: QUOTE_PRECISION
    pub quote_break_even_amount: i64,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default,
)]
pub enum PositionDirection {
    #[default]
    Long,
    Short,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default,
)]
pub enum OrderType {
    Market,

    #[default]
    Limit,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Default,
)]
pub enum OrderStatus {
    #[default]
    Init,
    Open,
    Filled,
    Canceled,
}

#[derive(AnchorSerialize, AnchorDeserialize, PartialEq, Debug, Eq, Default, Clone, Copy)]
pub struct Order {
    pub order_id: u32,
    pub market_index: u16,

    /// precision: PRICE_PRECISION
    pub price: u64,

    /// precision: BASE_PRECISION
    pub base_asset_amount: u64,

    /// precision: BASE_PRECISION
    pub base_asset_amount_filled: u64,
    pub direction: PositionDirection,
    pub order_type: OrderType,
    pub status: OrderStatus,
    pub max_ts: i64,
    pub reduce_only: bool,
    pub post_only: bool,
    pub immediate_or_cancel: bool,
}
