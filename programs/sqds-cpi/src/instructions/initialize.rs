use anchor_lang::prelude::*;
use anchor_spl::token_interface::TokenInterface;

use crate::state::StakePool;

#[derive(Accounts)]
pub struct Initialize<'info> {
    /// Payer of rent
    #[account(mut)]
    pub payer: Signer<'info>,

    #[account(
      init,
      seeds = [
        payer.key().as_ref(),
        b"stakePool",
      ],
      bump,
      payer = payer,
      space = 8 + StakePool::LEN,
    )]
    pub stake_pool: AccountLoader<'info, StakePool>,

    pub token_program: Interface<'info, TokenInterface>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,
}

pub fn initialize<'info>(ctx: Context<Initialize<'info>>) -> Result<()> {
    let mut stake_pool = ctx.accounts.stake_pool.load_init()?;
    stake_pool.creator = ctx.accounts.payer.key();
    stake_pool.bump_seed = ctx.bumps.stake_pool;

    Ok(())
}
