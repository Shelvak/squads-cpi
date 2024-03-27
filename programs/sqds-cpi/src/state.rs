use anchor_lang::prelude::*;

#[account(zero_copy)]
#[repr(C)]
pub struct StakePool {
    /// The original creator of the StakePool. Necessary for signer seeds
    pub creator: Pubkey,
    /** Token Account to store the staked SPL Token */
    pub mint: Pubkey,
    /** Bump seed for stake_mint */
    pub bump_seed: u8,
}

impl StakePool {
    pub const LEN: usize = std::mem::size_of::<StakePool>();
}
