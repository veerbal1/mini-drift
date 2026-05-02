use crate::controller::position::increase_open_bids_and_asks;
use crate::error::ErrorCode;

use crate::state::user::OrderType;
use crate::{
    error::MiniDriftResult,
    state::{
        order_params::OrderParams,
        user::{Order, PositionDirection, User},
    },
};

pub fn place_perp_order(user: &mut User, order_params: OrderParams) -> MiniDriftResult<()> {
    if order_params.order_type != OrderType::Market && order_params.order_type != OrderType::Limit {
        return Err(ErrorCode::UnsupportedOrderType);
    }
    let order_index = user.force_get_available_order_index()?;
    let position_index = user.force_get_perp_position_index(order_params.market_index)?;
    let existing_position_direction = if user.perp_positions[position_index].base_asset_amount >= 0
    {
        PositionDirection::Long
    } else {
        PositionDirection::Short
    };

    let new_order = Order::new_from_params(
        order_params,
        user.next_order_id,
        existing_position_direction,
    );
    user.orders[order_index] = new_order;
    user.next_order_id = user
        .next_order_id
        .checked_add(1)
        .ok_or(ErrorCode::MathError)?;

    user.open_orders = user
        .open_orders
        .checked_add(1)
        .ok_or(ErrorCode::MathError)?;
    user.perp_positions[position_index].open_orders = user.perp_positions[position_index]
        .open_orders
        .checked_add(1)
        .ok_or(ErrorCode::MathError)?;

    increase_open_bids_and_asks(
        &mut user.perp_positions[position_index],
        &new_order.direction,
        new_order.base_asset_amount,
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::state::user::OrderStatus;

    use super::*;

    #[test]
    pub fn place_perp_order_rejects_oracle_order() {
        let mut user = User::default();

        let order_params = OrderParams {
            order_type: OrderType::Oracle,
            direction: PositionDirection::Long,
            base_asset_amount: 0,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };
        let res = place_perp_order(&mut user, order_params);
        let err = res.unwrap_err();
        assert_eq!(err, ErrorCode::UnsupportedOrderType);
        assert_eq!(user.open_orders, 0);
        assert_eq!(user.perp_positions[0].market_index, 0);
    }

    #[test]
    pub fn place_perp_order_stores_limit_order() {
        let mut user = User::default();

        let order_params = OrderParams {
            order_type: OrderType::Limit,
            direction: PositionDirection::Long,
            base_asset_amount: 10,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, order_params);
        assert!(res.is_ok());
        assert_eq!(user.open_orders, 1);
        assert_eq!(user.orders[0].market_index, 2);
        assert_eq!(user.orders[0].status, OrderStatus::Open);
        assert_eq!(user.orders[0].base_asset_amount, 10);
        assert_eq!(user.perp_positions[0].market_index, 2);
        assert_eq!(user.perp_positions[0].open_orders, 1);
        assert_eq!(user.perp_positions[0].open_asks, 0);
        assert_eq!(user.perp_positions[0].open_bids, 10);
    }

    #[test]
    pub fn place_perp_order_stores_short_limit_as_open_ask() {
        let mut user = User::default();

        let order_params = OrderParams {
            order_type: OrderType::Limit,
            direction: PositionDirection::Short,
            base_asset_amount: 10,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, order_params);
        assert!(res.is_ok());
        assert_eq!(user.perp_positions[0].open_asks, -10);
        assert_eq!(user.perp_positions[0].open_bids, 0);
    }

    #[test]
    fn place_perp_order_stores_existing_position_direction() {
        let mut user = User::default();
        user.perp_positions[0].market_index = 2;
        user.perp_positions[0].base_asset_amount = -5;

        let order_params = OrderParams {
            order_type: OrderType::Limit,
            direction: PositionDirection::Long,
            base_asset_amount: 10,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, order_params);
        assert!(res.is_ok());
        assert_eq!(
            user.orders[0].existing_position_direction,
            PositionDirection::Short
        );
        assert_eq!(user.orders[0].market_index, 2);
        assert_eq!(user.orders[0].status, OrderStatus::Open);
        assert_eq!(user.next_order_id, 1);
    }

    #[test]
    fn place_perp_order_errors_when_no_order_slot_available() {
        let mut user = User::default();

        for order in user.orders.iter_mut() {
            order.status = OrderStatus::Open;
        }

        let order_params = OrderParams {
            order_type: OrderType::Limit,
            direction: PositionDirection::Long,
            base_asset_amount: 10,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, order_params);
        let err = res.unwrap_err();
        assert_eq!(err, ErrorCode::NoOrderSlotAvailable);
        assert_eq!(user.perp_positions[0].market_index, 0);
    }

    #[test]
    fn place_perp_order_errors_when_no_position_slot_available() {
        let mut user = User::default();

        // Make every perp position unavailable.
        // base_asset_amount != 0 => is_open_position() => is_available() == false
        user.perp_positions
            .iter_mut()
            .enumerate()
            .for_each(|(i, pos)| {
                pos.market_index = (i as u16) + 1; // occupied by other markets
                pos.base_asset_amount = 1; // simplest way to make slot unavailable
            });

        // Keep orders available (default statuses are non-Open).
        let order_params = OrderParams {
            order_type: OrderType::Limit, // pass order-type gate
            direction: PositionDirection::Long,
            base_asset_amount: 10,
            price: 100,
            market_index: 999, // "new" market not currently active
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, order_params);
        let err = res.unwrap_err();

        assert_eq!(err, ErrorCode::NoPerpPositionSlotAvailable);
        assert_eq!(user.orders[0].status, OrderStatus::Init);
    }
}
