// protocol-v2-master/programs/drift/src/state/user.rs

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
