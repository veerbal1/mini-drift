use anchor_lang::prelude::*;
use std::result::Result;

#[error_code]
pub enum ErrorCode {
    #[msg("Math Error")]
    MathError,
}

pub type MiniDriftResult<T = ()> = Result<T, ErrorCode>;
