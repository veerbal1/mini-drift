use crate::state::user::Order;
use anchor_lang::prelude::*;

#[event]
pub struct OrderRecord {
    pub ts: i64,
    pub user: Pubkey,
    pub order: Order,
}
