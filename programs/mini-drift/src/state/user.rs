// protocol-v2-master/programs/drift/src/state/user.rs

use anchor_lang::prelude::*;

use crate::error::{ErrorCode, MiniDriftResult};
use crate::state::order_params::OrderParams;

pub const MAX_PERP_POSITIONS: usize = 8;
pub const MAX_ORDERS: usize = 16;
pub const POSITION_FLAG_BEING_LIQUIDATED: u8 = 0b00000010;
pub const POSITION_FLAG_BANKRUPT: u8 = 0b00000100;

#[derive(
    AnchorSerialize, AnchorDeserialize, Default, Debug, PartialEq, Eq, Clone, Copy, InitSpace,
)]
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

    /// It will tell (count) number of OPEN (Unfilled Orders)
    pub open_orders: u8,

    pub open_bids: i64,

    pub open_asks: i64,

    pub position_flag: u8,

    pub isolated_position_scaled_balance: u64,
}

impl PerpPosition {
    pub fn is_open_position(&self) -> bool {
        // non-zero base means there is a live position.
        self.base_asset_amount != 0
    }

    pub fn has_open_order(&self) -> bool {
        self.open_orders != 0 || self.open_bids != 0 || self.open_asks != 0
    }

    pub fn has_unsettled_pnl(&self) -> bool {
        self.base_asset_amount == 0 && self.quote_asset_amount != 0
    }

    pub fn is_being_liquidated(&self) -> bool {
        self.position_flag & POSITION_FLAG_BEING_LIQUIDATED != 0
            || self.position_flag & POSITION_FLAG_BANKRUPT != 0
    }

    pub fn is_available(&self) -> bool {
        !self.is_open_position()
            && !self.has_open_order()
            && !self.has_unsettled_pnl()
            && !self.is_being_liquidated()
            && self.isolated_position_scaled_balance == 0
    }

    pub fn is_for(&self, market_index: u16) -> bool {
        self.market_index == market_index && !self.is_available()
    }
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Default,
    InitSpace,
)]
pub enum PositionDirection {
    #[default]
    Long,
    Short,
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Default,
    InitSpace,
)]
pub enum OrderType {
    Market,

    #[default]
    Limit,

    TriggerMarket,

    TriggerLimit,

    Oracle,
}

#[derive(
    AnchorSerialize,
    AnchorDeserialize,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Debug,
    Default,
    InitSpace,
)]
pub enum OrderStatus {
    #[default]
    Init,
    Open,
    Filled,
    Canceled,
}

#[derive(
    AnchorSerialize, AnchorDeserialize, PartialEq, Debug, Eq, Default, Clone, Copy, InitSpace,
)]
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

    pub quote_asset_amount_filled: u64,
    pub existing_position_direction: PositionDirection,

    pub auction_duration: u8,
    pub auction_start_price: i64,
    pub auction_end_price: i64,

    pub bit_flags: u8,
}

impl Order {
    pub fn is_available(&self) -> bool {
        self.status != OrderStatus::Open
    }

    pub fn new_from_params(
        params: OrderParams,
        order_id: u32,
        existing_position_direction: PositionDirection,
    ) -> Self {
        let mut order = Order::default();
        order.order_type = params.order_type;
        order.direction = params.direction;
        order.base_asset_amount = params.base_asset_amount;
        order.price = params.price;
        order.market_index = params.market_index;
        order.reduce_only = params.reduce_only;
        order.post_only = params.post_only;
        order.immediate_or_cancel = params.immediate_or_cancel;
        order.max_ts = params.max_ts;

        // Protocol level fields:
        order.order_id = order_id;
        order.status = OrderStatus::Open;
        order.existing_position_direction = existing_position_direction;

        order
    }
}

#[account]
#[derive(PartialEq, Debug, Eq, Default, InitSpace)]
pub struct User {
    pub authority: Pubkey,

    pub perp_positions: [PerpPosition; MAX_PERP_POSITIONS],

    pub orders: [Order; MAX_ORDERS],

    pub next_order_id: u32,

    pub open_orders: u8,
}

impl User {
    pub const LEN: usize = 8 + User::INIT_SPACE;

    pub fn get_perp_position_index(&self, market_index: u16) -> Option<usize> {
        self.perp_positions
            .iter()
            .position(|&position| position.is_for(market_index))
    }

    pub fn get_available_perp_position_index(&self) -> Option<usize> {
        self.perp_positions
            .iter()
            .position(|&position| position.is_available())
    }

    pub fn get_available_order_index(&self) -> Option<usize> {
        self.orders.iter().position(|&order| order.is_available())
    }

    pub fn force_get_perp_position_index(&mut self, market_index: u16) -> MiniDriftResult<usize> {
        let active_position_index = self.get_perp_position_index(market_index);

        if let Some(index) = active_position_index {
            return Ok(index);
        } else {
            let available_free_index = self.get_available_perp_position_index();
            if let Some(available_index) = available_free_index {
                self.perp_positions[available_index].market_index = market_index;
                Ok(available_index)
            } else {
                Err(ErrorCode::NoPerpPositionSlotAvailable)
            }
        }
    }

    pub fn force_get_available_order_index(&self) -> MiniDriftResult<usize> {
        self.get_available_order_index()
            .ok_or(ErrorCode::NoOrderSlotAvailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn force_get_perp_position_index_prefers_existing_market_position() {
        let mut user = User::default();
        const SOL_MARKET_INDEX: u16 = 0;
        const BTC_MARKET_INDEX: u16 = 1;
        user.perp_positions[0].base_asset_amount = 0;
        user.perp_positions[0].market_index = BTC_MARKET_INDEX;
        user.perp_positions[1].base_asset_amount = 10;
        user.perp_positions[1].market_index = SOL_MARKET_INDEX;

        let res = user
            .force_get_perp_position_index(SOL_MARKET_INDEX)
            .unwrap();
        assert_eq!(res, 1);
        assert_ne!(res, 0);
    }

    #[test]
    fn force_get_perp_position_index_reuses_available_position() {
        let mut user = User::default();
        const SOL_MARKET_INDEX: u16 = 0;
        const BTC_MARKET_INDEX: u16 = 1;
        const ETH_MARKET_INDEX: u16 = 2;

        user.perp_positions[0].base_asset_amount = 0;
        user.perp_positions[0].market_index = BTC_MARKET_INDEX;
        user.perp_positions[1].base_asset_amount = 10;
        user.perp_positions[1].market_index = ETH_MARKET_INDEX;

        // old SOL label is ignored if available; first free folder is reused.
        user.perp_positions[2].base_asset_amount = 0;
        user.perp_positions[2].market_index = SOL_MARKET_INDEX;

        // Check if function pick index 0, because index 0 is free.
        let res = user
            .force_get_perp_position_index(SOL_MARKET_INDEX)
            .unwrap();
        assert_eq!(res, 0);
        assert_ne!(res, 2);
        assert_eq!(user.perp_positions[0].market_index, SOL_MARKET_INDEX);
    }

    #[test]
    fn force_get_perp_position_index_errors_when_no_position_slot_available() {
        let mut user = User::default();
        const SOL_MARKET_INDEX: u16 = 0;
        user.perp_positions
            .iter_mut()
            .enumerate()
            .for_each(|(index, pos)| {
                pos.market_index = index as u16 + 1;
                pos.base_asset_amount = 1;
            });

        let res = user.force_get_perp_position_index(SOL_MARKET_INDEX);
        let err = res.unwrap_err();
        assert_eq!(err, ErrorCode::NoPerpPositionSlotAvailable);
    }

    // Test Order
    #[test]
    fn get_available_order_index_returns_first_non_open_order() {
        let mut user = User::default();
        user.orders[0].status = OrderStatus::Open;
        user.orders[1].status = OrderStatus::Open;
        user.orders[2].status = OrderStatus::Filled;
        user.orders[3].status = OrderStatus::Canceled;

        let index = user.get_available_order_index().unwrap();
        assert_eq!(index, 2);
        assert_ne!(index, 0);
        assert_ne!(index, 1);
    }

    #[test]
    fn get_available_order_index_returns_none_when_all_orders_open() {
        let mut user = User::default();

        user.orders
            .iter_mut()
            .for_each(|order| order.status = OrderStatus::Open);

        let index = user.get_available_order_index();
        assert!(index.is_none());
    }

    #[test]
    fn force_get_available_order_index_errors_when_no_order_slot_available() {
        let mut user = User::default();

        user.orders
            .iter_mut()
            .for_each(|order| order.status = OrderStatus::Open);

        let err = user.force_get_available_order_index().unwrap_err();
        assert_eq!(err, ErrorCode::NoOrderSlotAvailable);
    }

    #[test]
    fn order_new_from_params_stores_params_and_protocol_fields() {
        let order_params = OrderParams {
            order_type: OrderType::Limit,
            direction: PositionDirection::Long,
            base_asset_amount: 10,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: true,
            immediate_or_cancel: false,
            max_ts: 12345,
        };
        let order = Order::new_from_params(order_params, 7, PositionDirection::Short);
        assert_eq!(order.status, OrderStatus::Open);
        assert_eq!(order.order_id, 7);
        assert_eq!(order.existing_position_direction, PositionDirection::Short);
        assert_eq!(order.base_asset_amount_filled, 0);
        assert_eq!(order.quote_asset_amount_filled, 0);

        assert_eq!(order.price, order_params.price);
        assert_eq!(order.market_index, order_params.market_index);
        assert_eq!(order.base_asset_amount, order_params.base_asset_amount);
        assert_eq!(order.order_type, order_params.order_type);
        assert_eq!(order.direction, order_params.direction);
        assert_eq!(order.reduce_only, order_params.reduce_only);
        assert_eq!(order.post_only, order_params.post_only);
        assert_eq!(order.immediate_or_cancel, order_params.immediate_or_cancel);
        assert_eq!(order.max_ts, order_params.max_ts);
    }
}
