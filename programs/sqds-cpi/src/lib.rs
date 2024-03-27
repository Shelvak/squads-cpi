use anchor_lang::prelude::*;
use instructions::*;

pub mod errors;
pub mod instructions;
pub mod macros;
pub mod state;

declare_id!("ZuE1yg15X2GkuWLLNh1gqh2y5NwYEHD11r5CWftsbEW");

#[program]
pub mod sqds_cpi {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        initialize::initialize(ctx)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        deposit::deposit(ctx, amount)
    }
}
