use anchor_lang::prelude::*;

#[derive(
    AnchorSerialize, AnchorDeserialize, Default, Debug, PartialEq, Eq, Clone, Copy, InitSpace,
)]
pub struct PerpMarket {
    pub number_of_users: u32,
    pub number_of_users_with_base: u32,
    pub market_index: u16,
    pub order_step_size: u64,
}
