use crate::{
    error::{ErrorCode, MiniDriftResult},
    state::user::{PerpPosition, PositionDirection},
};

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
