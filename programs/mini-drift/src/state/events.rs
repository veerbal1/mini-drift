use crate::state::user::Order;
use anchor_lang::prelude::*;

#[event]
pub struct OrderRecord {
    pub ts: i64,
    pub user: Pubkey,
    pub order: Order,
}

#[event]
pub struct OrderActionRecord {
    pub ts: i64,
    pub action: OrderAction,
    pub action_explanation: OrderActionExplanation,
    pub market_index: u16,
    pub filler: Pubkey,
    pub base_asset_amount_filled: u64,
    pub quote_asset_amount_filled: u64,
    pub taker: Pubkey,
    pub taker_order: Order,
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Copy,
    Default,
    PartialOrd,
    Ord,
    InitSpace,
)]
pub enum OrderAction {
    #[default]
    Fill,
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    PartialEq,
    Eq,
    Debug,
    Clone,
    Copy,
    Default,
    PartialOrd,
    Ord,
    InitSpace,
)]
pub enum OrderActionExplanation {
    #[default]
    OrderFilledWithAMM,
}

pub fn get_order_action_record(
    ts: i64,
    market_index: u16,
    filler: Pubkey,
    base_asset_amount_filled: u64,
    quote_asset_amount_filled: u64,
    taker: Pubkey,
    taker_order: Order,
) -> OrderActionRecord {
    OrderActionRecord {
        ts,
        action: OrderAction::Fill,
        action_explanation: OrderActionExplanation::OrderFilledWithAMM,
        market_index,
        filler,
        base_asset_amount_filled,
        quote_asset_amount_filled,
        taker,
        taker_order,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_order_action_record_builds_fill_receipt() {
        let mut taker_order = Order::default();
        taker_order.market_index = 2;
        taker_order.base_asset_amount = 10;
        taker_order.base_asset_amount_filled = 7;
        taker_order.quote_asset_amount_filled = 700;
        let filler = Pubkey::new_unique();
        let taker = Pubkey::new_unique();

        let record = get_order_action_record(
            1, // ts
            2, // market_index
            filler,
            3,   // base_asset_amount_filled on record
            300, // quote_asset_amount_filled on record
            taker,
            taker_order,
        );

        assert_eq!(record.action, OrderAction::Fill);
        assert_eq!(
            record.action_explanation,
            OrderActionExplanation::OrderFilledWithAMM
        );
        assert_eq!(record.market_index, 2);
        assert_eq!(record.base_asset_amount_filled, 3);
        assert_eq!(record.quote_asset_amount_filled, 300);
        assert_eq!(record.taker_order.base_asset_amount_filled, 7);
        assert_eq!(record.filler, filler);
        assert_eq!(record.taker, taker);
    }
}
