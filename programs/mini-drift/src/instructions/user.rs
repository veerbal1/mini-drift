use crate::controller::orders::place_perp_order;
use crate::state::order_params::OrderParams;
use crate::state::user::User;
use anchor_lang::prelude::*;

pub fn handle_place_perp_order(
    ctx: Context<PlacePerpOrder>,
    order_params: OrderParams,
) -> Result<()> {
    let now = Clock::get()?.unix_timestamp;
    let user_key = ctx.accounts.user.key();
    let user = &mut ctx.accounts.user;
    place_perp_order(user, user_key, order_params, now)?;

    Ok(())
}

#[derive(Accounts)]
pub struct PlacePerpOrder<'info> {
    #[account(mut)]
    pub authority: Signer<'info>,

    #[account(mut, has_one = authority)]
    pub user: Account<'info, User>,
}
