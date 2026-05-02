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
    let position_index = user.force_get_perp_position_index(order_params.market_index)?;
    let order_index = user.force_get_available_order_index()?;
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
}
