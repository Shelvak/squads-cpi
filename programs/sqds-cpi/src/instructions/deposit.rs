use anchor_lang::prelude::*;
use anchor_spl::token_interface::{
    // Mint,
    // MintTo,
    TokenInterface,
    // TokenAccount,
    // TransferChecked,
    // transfer_checked,
    // mint_to,
};

use crate::stake_pool_signer_seeds;
use crate::state::StakePool;

use squads_multisig_program as Sqds;

#[derive(Accounts)]
#[instruction(amount: u64)]
pub struct Deposit<'info> {
    #[account(mut)]
    pub payer: Signer<'info>,

    /// StakePool owning the vault that will receive the deposit
    #[account(mut)]
    pub stake_pool: AccountLoader<'info, StakePool>,

    pub token_program: Interface<'info, TokenInterface>,
    pub rent: Sysvar<'info, Rent>,
    pub system_program: Program<'info, System>,

    #[account(mut)]
    pub multisig_pda: Account<'info, Sqds::state::Multisig>,
    #[account(mut)]
    /// CHECK: testing
    pub sqds_vault_pda: UncheckedAccount<'info>,
    #[account(mut)]
    /// CHECK: testing
    pub transaction_pda: UncheckedAccount<'info>,
    /// CHECK: testing
    #[account(mut)]
    pub proposal_pda: UncheckedAccount<'info>,

    #[account(address = squads_multisig_program::ID)]
    pub sqds_program: Program<'info, squads_multisig_program::program::SquadsMultisigProgram>,
}

impl<'info> Deposit<'info> {

    pub fn transfer_from_user(&self, amount: u64) -> Result<()> {
        msg!("deposit.rs:50");
        anchor_lang::solana_program::program::invoke_signed(
            &anchor_lang::solana_program::system_instruction::transfer(
                &self.payer.key(), &self.sqds_vault_pda.key(), amount
            ),
            &[
                self.payer.to_account_info(),
                self.sqds_vault_pda.to_account_info(),
                self.system_program.to_account_info(),
            ],
            &[],
        )?;

        Ok(())
    }

    pub fn mint_token_to_user(&self, amount: u64) -> Result<()> {
        let stake_pool = self.stake_pool.load()?;
        let signer_seeds: &[&[&[u8]]] = &[stake_pool_signer_seeds!(stake_pool)];

        let message = anchor_lang::solana_program::message::Message::new(
            &[
                anchor_lang::solana_program::system_instruction::transfer(
                    &self.sqds_vault_pda.key(),
                    &self.payer.key(),
                    amount,
                ),
            ],
            Some(&self.payer.key()),
        );
        {
            msg!("About to create TX");
            Sqds::cpi::vault_transaction_create(
                CpiContext::new_with_signer(
                    self.sqds_program.to_account_info(),
                    Sqds::cpi::accounts::VaultTransactionCreate {
                        multisig: self.multisig_pda.to_account_info(),
                        creator: self.stake_pool.to_account_info(),
                        transaction: self.transaction_pda.to_account_info(),
                        rent_payer: self.payer.to_account_info(),
                        system_program: self.system_program.to_account_info(),
                    },
                    signer_seeds
                ),
                Sqds::VaultTransactionCreateArgs {
                    vault_index: 0,
                    ephemeral_signers: 0,
                    memo: None,
                    transaction_message: message.serialize().to_vec(),
                },
            )?;
            msg!(" Create TX DONE!");
        }
        {
            let mult = self.multisig_pda.clone();
            let prop = self.proposal_pda.clone();
            Sqds::cpi::proposal_create(
                CpiContext::new_with_signer(
                    self.sqds_program.to_account_info(),
                    Sqds::cpi::accounts::ProposalCreate {
                        multisig: mult.to_account_info(),
                        creator: self.stake_pool.to_account_info(),
                        proposal: prop.to_account_info(),
                        rent_payer: self.payer.to_account_info(),
                        system_program: self.system_program.to_account_info(),
                    },
                    signer_seeds
                ),
                Sqds::ProposalCreateArgs {
                    transaction_index: 2, draft: false // this should be a param or check the index
                },
            )?;
            msg!(" Create Proposal DONE!");
            drop(mult);
            drop(prop);
        }
        {
            let mult = self.multisig_pda.clone();
            let prop = self.proposal_pda.clone();

            Sqds::cpi::proposal_approve(
                CpiContext::new_with_signer(
                    self.sqds_program.to_account_info(),
                    Sqds::cpi::accounts::ProposalVote {
                        multisig: mult.to_account_info(),
                        proposal: prop.to_account_info(),
                        member: self.stake_pool.to_account_info(),
                    },
                    signer_seeds
                ),
                Sqds::ProposalVoteArgs { memo: None },
            )?;
            msg!(" Approve Proposal DONE!");
            drop(mult);
            drop(prop);
        }
        {
            Sqds::cpi::vault_transaction_execute(
                CpiContext::new_with_signer(
                    self.sqds_program.to_account_info(),
                    Sqds::cpi::accounts::VaultTransactionExecute {
                        multisig: self.multisig_pda.to_account_info(),
                        proposal: self.proposal_pda.clone().to_account_info(),
                        transaction: self.transaction_pda.clone().to_account_info(),
                        member: self.stake_pool.to_account_info(),
                    },
                    signer_seeds
                ),
            )?;
            msg!(" Execute Proposal DONE!");
        }

        Ok(())
    }
}

pub fn deposit<'info>(ctx: Context<Deposit<'info>>, amount: u64) -> Result<()> {
    ctx.accounts.transfer_from_user(amount)?;
    ctx.accounts.mint_token_to_user(amount)?;
    Ok(())
}
