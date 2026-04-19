use anchor_lang::prelude::*;
use anchor_spl::token::{self, Mint, Token, TokenAccount, Transfer};

declare_id!("2KKj6Fxmm9pnNn9tHxEi6MPHggoAKkmi2MJiMQiVg5aL");

#[program]
pub mod solana_project {
    use super::*;

    pub fn initialize_profile(
        ctx: Context<InitializeProfile>,
        split_main_pct: u8,
        split_yield_pct: u8,
        split_lock_pct: u8,
        weekly_release_amount: u64,
        yield_protocol: u8,
    ) -> Result<()> {
        require!(
            (split_main_pct as u16) + (split_yield_pct as u16) + (split_lock_pct as u16) == 100,
            RouterError::InvalidPercentages
        );

        ctx.accounts.profile.set_inner(UserProfile {
            owner: ctx.accounts.owner.key(),
            split_main_pct,
            split_yield_pct,
            split_lock_pct,
            weekly_release_amount,
            last_claim_timestamp: Clock::get()?.unix_timestamp,
            yield_protocol: YieldProtocol::from_u8(yield_protocol)?,
            bump: ctx.bumps.profile,
        });
        
        Ok(())
    }

    pub fn update_profile(
        ctx: Context<UpdateProfile>,
        split_main_pct: u8,
        split_yield_pct: u8,
        split_lock_pct: u8,
        weekly_release_amount: u64,
        yield_protocol: u8,
    ) -> Result<()> {
        let profile = &mut ctx.accounts.profile;
        
        require!(
            (split_main_pct as u16) + (split_yield_pct as u16) + (split_lock_pct as u16) == 100,
            RouterError::InvalidPercentages
        );

        profile.split_main_pct = split_main_pct;
        profile.split_yield_pct = split_yield_pct;
        profile.split_lock_pct = split_lock_pct;
        profile.weekly_release_amount = weekly_release_amount;
        profile.yield_protocol = YieldProtocol::from_u8(yield_protocol)?;
        
        Ok(())
    }

    pub fn process_payment(ctx: Context<ProcessPayment>) -> Result<()> {
        let profile = &ctx.accounts.profile;
        let total_amount = ctx.accounts.router_vault.amount;
        
        require!(total_amount > 0, RouterError::NoFundsToProcess);

        let main_amount = (total_amount as u128)
            .checked_mul(profile.split_main_pct as u128)
            .unwrap()
            .checked_div(100)
            .unwrap() as u64;

        let yield_amount = (total_amount as u128)
            .checked_mul(profile.split_yield_pct as u128)
            .unwrap()
            .checked_div(100)
            .unwrap() as u64;

        let lock_amount = (total_amount as u128)
            .checked_mul(profile.split_lock_pct as u128)
            .unwrap()
            .checked_div(100)
            .unwrap() as u64;

        let owner_key = profile.owner;
        let seeds = &[b"profile".as_ref(), owner_key.as_ref(), &[profile.bump]];
        let signer = &[&seeds[..]];

        // 1. Transfer to main wallet
        if main_amount > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.router_vault.to_account_info(),
                to: ctx.accounts.owner_token_account.to_account_info(),
                authority: profile.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            token::transfer(cpi_ctx, main_amount)?;
        }

        // 2. Transfer to lock vault
        if lock_amount > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.router_vault.to_account_info(),
                to: ctx.accounts.lock_vault.to_account_info(),
                authority: profile.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            token::transfer(cpi_ctx, lock_amount)?;
        }

        // 3. Handle Yield (Mocked as transferring to a Yield Vault for now until CPI is fully integrated)
        // In a production setup, this is where Kamino Finance's deposit instruction is invoked.
        if yield_amount > 0 {
            let cpi_accounts = Transfer {
                from: ctx.accounts.router_vault.to_account_info(),
                to: ctx.accounts.yield_vault.to_account_info(),
                authority: profile.to_account_info(),
            };
            let cpi_program = ctx.accounts.token_program.to_account_info();
            let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
            token::transfer(cpi_ctx, yield_amount)?;
        }

        Ok(())
    }

    pub fn claim_locked_funds(ctx: Context<ClaimLockedFunds>) -> Result<()> {
        let profile = &mut ctx.accounts.profile;
        let current_time = Clock::get()?.unix_timestamp;
        
        let seconds_passed = current_time.saturating_sub(profile.last_claim_timestamp);
        let seven_days = 7 * 24 * 60 * 60;
        
        require!(seconds_passed >= seven_days, RouterError::LockPeriodNotPassed);

        let amount_to_claim = profile.weekly_release_amount.min(ctx.accounts.lock_vault.amount);
        require!(amount_to_claim > 0, RouterError::NoFundsToProcess);

        let owner_key = profile.owner;
        let seeds = &[b"profile".as_ref(), owner_key.as_ref(), &[profile.bump]];
        let signer = &[&seeds[..]];

        let cpi_accounts = Transfer {
            from: ctx.accounts.lock_vault.to_account_info(),
            to: ctx.accounts.owner_token_account.to_account_info(),
            authority: profile.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount_to_claim)?;

        profile.last_claim_timestamp = current_time;

        Ok(())
    }

    pub fn withdraw_yield(ctx: Context<WithdrawYield>, amount: u64) -> Result<()> {
        let profile = &ctx.accounts.profile;
        
        let owner_key = profile.owner;
        let seeds = &[b"profile".as_ref(), owner_key.as_ref(), &[profile.bump]];
        let signer = &[&seeds[..]];

        // Handle Yield Withdrawal (Mocked as transferring back from Yield Vault)
        // In production, this invokes Kamino Finance's withdraw instruction to burn kTokens for USDC.
        let cpi_accounts = Transfer {
            from: ctx.accounts.yield_vault.to_account_info(),
            to: ctx.accounts.owner_token_account.to_account_info(),
            authority: profile.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer);
        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct ProcessPayment<'info> {
    #[account(
        seeds = [b"profile", profile.owner.as_ref()],
        bump = profile.bump,
    )]
    pub profile: Account<'info, UserProfile>,
    
    #[account(
        mut,
        seeds = [b"router_vault", profile.key().as_ref()],
        bump
    )]
    pub router_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        seeds = [b"lock_vault", profile.key().as_ref()],
        bump
    )]
    pub lock_vault: Account<'info, TokenAccount>,

    /// CHECK: For now, mock yield vault as just a token account.
    #[account(mut)]
    pub yield_vault: AccountInfo<'info>,

    #[account(
        mut,
        constraint = owner_token_account.owner == profile.owner
    )]
    pub owner_token_account: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct ClaimLockedFunds<'info> {
    #[account(
        mut,
        seeds = [b"profile", owner.key().as_ref()],
        bump = profile.bump,
        has_one = owner @ RouterError::Unauthorized
    )]
    pub profile: Account<'info, UserProfile>,
    
    #[account(
        mut,
        seeds = [b"lock_vault", profile.key().as_ref()],
        bump
    )]
    pub lock_vault: Account<'info, TokenAccount>,

    #[account(
        mut,
        constraint = owner_token_account.owner == profile.owner
    )]
    pub owner_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct WithdrawYield<'info> {
    #[account(
        seeds = [b"profile", owner.key().as_ref()],
        bump = profile.bump,
        has_one = owner @ RouterError::Unauthorized
    )]
    pub profile: Account<'info, UserProfile>,
    
    /// CHECK: mocked
    #[account(mut)]
    pub yield_vault: AccountInfo<'info>,

    #[account(
        mut,
        constraint = owner_token_account.owner == profile.owner
    )]
    pub owner_token_account: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub owner: Signer<'info>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(split_main_pct: u8, split_yield_pct: u8, split_lock_pct: u8, weekly_release_amount: u64, yield_protocol: u8)]
pub struct UpdateProfile<'info> {
    #[account(
        mut,
        seeds = [b"profile", owner.key().as_ref()],
        bump = profile.bump,
        has_one = owner @ RouterError::Unauthorized
    )]
    pub profile: Account<'info, UserProfile>,
    
    #[account(mut)]
    pub owner: Signer<'info>,
}

#[derive(Accounts)]
#[instruction(split_main_pct: u8, split_yield_pct: u8, split_lock_pct: u8, weekly_release_amount: u64, yield_protocol: u8)]
pub struct InitializeProfile<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        init,
        payer = owner,
        space = 8 + UserProfile::INIT_SPACE,
        seeds = [b"profile", owner.key().as_ref()],
        bump
    )]
    pub profile: Box<Account<'info, UserProfile>>,
    
    pub usdc_mint: Box<Account<'info, Mint>>,

    #[account(
        init,
        payer = owner,
        token::mint = usdc_mint,
        token::authority = profile,
        seeds = [b"router_vault", profile.key().as_ref()],
        bump
    )]
    pub router_vault: Box<Account<'info, TokenAccount>>,

    #[account(
        init,
        payer = owner,
        token::mint = usdc_mint,
        token::authority = profile,
        seeds = [b"lock_vault", profile.key().as_ref()],
        bump
    )]
    pub lock_vault: Box<Account<'info, TokenAccount>>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[account]
#[derive(InitSpace)]
pub struct UserProfile {
    pub owner: Pubkey,
    pub split_main_pct: u8,
    pub split_yield_pct: u8,
    pub split_lock_pct: u8,
    pub weekly_release_amount: u64,
    pub last_claim_timestamp: i64,
    pub yield_protocol: YieldProtocol,
    pub bump: u8,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, PartialEq, Eq, InitSpace)]
pub enum YieldProtocol {
    Kamino,
    Drift,
}

impl YieldProtocol {
    pub fn from_u8(val: u8) -> Result<Self> {
        match val {
            0 => Ok(YieldProtocol::Kamino),
            1 => Ok(YieldProtocol::Drift),
            _ => Err(RouterError::InvalidYieldProtocol.into()),
        }
    }
}

#[error_code]
pub enum RouterError {
    #[msg("Split percentages must sum to exactly 100.")]
    InvalidPercentages,
    #[msg("Invalid yield protocol selected.")]
    InvalidYieldProtocol,
    #[msg("You are not authorized to perform this action.")]
    Unauthorized,
    #[msg("No funds available to process.")]
    NoFundsToProcess,
    #[msg("The 7-day lock period has not passed yet.")]
    LockPeriodNotPassed,
}
