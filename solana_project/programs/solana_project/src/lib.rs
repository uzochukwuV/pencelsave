use anchor_lang::prelude::*;

declare_id!("2KKj6Fxmm9pnNn9tHxEi6MPHggoAKkmi2MJiMQiVg5aL");

#[program]
pub mod solana_project {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
