use crate::state::user::{OrderType, PositionDirection};
use anchor_lang::prelude::*;

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct OrderParams {
    pub order_type: OrderType,
    pub direction: PositionDirection,

    /// precision: BASE_PRECISION
    pub base_asset_amount: u64,

    /// precision: PRICE_PRECISION
    pub price: u64,

    pub market_index: u16,

    pub reduce_only: bool,

    pub post_only: bool,

    pub immediate_or_cancel: bool,

    pub max_ts: i64,
}
