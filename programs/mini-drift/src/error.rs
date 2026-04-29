use anchor_lang::prelude::*;
use std::result::Result;

#[error_code]
pub enum ErrorCode {
    #[msg("Math Error")]
    MathError,

    #[msg("No order slot available")]
    NoOrderSlotAvailable,

    #[msg("No perp position slot available")]
    NoPerpPositionSlotAvailable,

    #[msg("Reduce-only order would increase or flip position")]
    ReduceOnlyOrderWouldIncreasePosition,

    #[msg("Unsupported order type")]
    UnsupportedOrderType,
}

pub type MiniDriftResult<T = ()> = Result<T, ErrorCode>;
