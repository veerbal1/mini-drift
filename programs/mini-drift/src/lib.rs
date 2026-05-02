use anchor_lang::prelude::*;

use crate::state::user::User;

declare_id!("9ehbjawRhTfkRncCbVfSJDMKb2vZPtrA9vRzdf6EoVS5");

pub mod error;
pub mod math;
pub mod state;
pub mod controller;
#[program]
pub mod mini_drift {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }

    pub fn initialize_user(ctx: Context<InitializeUser>) -> Result<()> {
        let user_account = &mut ctx.accounts.user;
        user_account.authority = ctx.accounts.authority.key();
        user_account.next_order_id = 1;
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}

#[derive(Accounts)]
pub struct InitializeUser<'info> {
    /// Balance will be deducted to pay for user account, that's why I put `mut` here.
    #[account(mut)]
    pub authority: Signer<'info>,

    /// TODO: PDA is pending, I have to know seeds
    #[account(init, payer = authority, space = User::LEN)]
    pub user: Account<'info, User>,

    pub system_program: Program<'info, System>,
}
