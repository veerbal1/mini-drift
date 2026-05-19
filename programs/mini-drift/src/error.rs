use anchor_lang::prelude::*;
use std::result::Result;

#[derive(PartialEq)]
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

    #[msg("Unexpected Error")]
    UnexpectedError,

    #[msg("Invalid Perp Position Detected")]
    InvalidPerpPositionDetected,

    #[msg("Invalid AMM Detected")]
    InvalidAmmDetected,

    #[msg("Invalid Order Status")]
    InvalidOrderStatus,

    #[msg("Invalid Market Account")]
    InvalidMarketAccount,

    #[msg("Invalid Fill Price")]
    InvalidFillPrice,
}

pub type MiniDriftResult<T = ()> = Result<T, ErrorCode>;
