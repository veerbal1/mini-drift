use anchor_lang::prelude::*;

declare_id!("9ehbjawRhTfkRncCbVfSJDMKb2vZPtrA9vRzdf6EoVS5");

pub mod math;
pub mod error;
pub mod state;

#[program]
pub mod mini_drift {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
