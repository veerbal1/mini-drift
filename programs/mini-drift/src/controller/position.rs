use std::cmp::{max, min};

use crate::{
    error::{ErrorCode, MiniDriftResult},
    state::user::{PerpPosition, PositionDirection},
};

#[derive(Debug, Default, PartialEq, Eq)]
pub struct PositionDelta {
    pub base_asset_amount: i64,
    pub quote_asset_amount: i64,
}

pub fn increase_open_bids_and_asks(
    position: &mut PerpPosition,
    direction: &PositionDirection,
    base_asset_amount_unfilled: u64,
) -> MiniDriftResult<()> {
    let base_asset_amount_unfilled_i64 =
        i64::try_from(base_asset_amount_unfilled).map_err(|_| ErrorCode::MathError)?;
    match *direction {
        PositionDirection::Long => {
            position.open_bids = position
                .open_bids
                .checked_add(base_asset_amount_unfilled_i64)
                .ok_or(ErrorCode::MathError)?
        }
        PositionDirection::Short => {
            position.open_asks = position
                .open_asks
                .checked_sub(base_asset_amount_unfilled_i64)
                .ok_or(ErrorCode::MathError)?
        }
    }
    Ok(())
}

pub fn decrease_open_bids_and_asks(
    position: &mut PerpPosition,
    direction: &PositionDirection,
    base_asset_amount: u64,
    update: bool,
) -> MiniDriftResult<()> {
    if !update {
        return Ok(());
    }
    let base_asset_amount_i64 =
        i64::try_from(base_asset_amount).map_err(|_| ErrorCode::MathError)?;
    if *direction == PositionDirection::Long {
        position.open_bids = max(
            position
                .open_bids
                .checked_sub(base_asset_amount_i64)
                .ok_or(ErrorCode::MathError)?,
            0,
        );
        Ok(())
    } else {
        position.open_asks = min(
            position
                .open_asks
                .checked_add(base_asset_amount_i64)
                .ok_or(ErrorCode::MathError)?,
            0,
        );
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use anchor_lang::prelude::Pubkey;

    use crate::{
        controller::orders::{place_perp_order, update_order_after_fill},
        state::{
            order_params::OrderParams,
            user::{OrderType, User},
        },
    };

    use super::*;

    #[test]
    fn decrease_open_bids_and_asks_decreases_open_bids_for_long() {
        let mut user = User::default();
        let order_params = OrderParams {
            order_type: OrderType::Market,
            direction: PositionDirection::Long,
            base_asset_amount: 5,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, Pubkey::default(), order_params, 0, 10);
        assert!(res.is_ok());
        assert_eq!(user.perp_positions[0].open_bids, 5);

        // fill the order
        let res = update_order_after_fill(&mut user.orders[0], 4, 100).unwrap();
        assert!(!res);
        let res2 = decrease_open_bids_and_asks(
            &mut user.perp_positions[0],
            &PositionDirection::Long,
            4,
            true,
        );
        assert!(res2.is_ok());
        assert_eq!(user.perp_positions[0].open_bids, 1);
    }

    #[test]
    fn decrease_open_bids_and_asks_increases_open_asks_for_short() {
        let mut user = User::default();
        let order_params = OrderParams {
            order_type: OrderType::Market,
            direction: PositionDirection::Short,
            base_asset_amount: 5,
            price: 100,
            market_index: 2,
            reduce_only: false,
            post_only: false,
            immediate_or_cancel: false,
            max_ts: 100,
        };

        let res = place_perp_order(&mut user, Pubkey::default(), order_params, 0, 10);
        assert!(res.is_ok());
        assert_eq!(user.perp_positions[0].open_asks, -5);

        // fill the order
        let res = update_order_after_fill(&mut user.orders[0], 4, 100).unwrap();
        assert!(!res);
        let res2 = decrease_open_bids_and_asks(
            &mut user.perp_positions[0],
            &PositionDirection::Short,
            4,
            true,
        );
        assert!(res2.is_ok());
        assert_eq!(user.perp_positions[0].open_asks, -1);
    }

    #[test]
    fn decrease_open_bids_and_asks_clamps_open_bids_to_zero() {
        let mut position = PerpPosition::default();
        position.open_bids = 3;

        let res = decrease_open_bids_and_asks(&mut position, &PositionDirection::Long, 5, true);

        assert!(res.is_ok());
        assert_eq!(position.open_bids, 0);
    }

    #[test]
    fn decrease_open_bids_and_asks_clamps_open_asks_to_zero() {
        let mut position = PerpPosition::default();
        position.open_asks = -3;

        let res = decrease_open_bids_and_asks(&mut position, &PositionDirection::Short, 5, true);

        assert!(res.is_ok());
        assert_eq!(position.open_asks, 0);
    }

    #[test]
    fn decrease_open_bids_and_asks_does_nothing_when_update_is_false() {
        let mut position = PerpPosition::default();
        position.open_bids = 10;

        let res = decrease_open_bids_and_asks(&mut position, &PositionDirection::Long, 5, false);

        assert!(res.is_ok());
        assert_eq!(position.open_bids, 10);
    }
}
