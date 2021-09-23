use anchor_lang::prelude::*;

declare_id!("Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS");

pub mod utils {
    use anchor_lang::prelude::{AccountInfo, ProgramResult};
    use anchor_lang::ToAccountInfo;

    pub fn transfer(
        source: &AccountInfo,
        dest: &AccountInfo,
        amount_in_lamports: u64,
    ) -> ProgramResult {
        let source_info_ = source.to_account_info();
        **source_info_.try_borrow_mut_lamports()? -= amount_in_lamports;
        let dest_info = &mut dest.to_account_info();
        **dest_info.try_borrow_mut_lamports()? += amount_in_lamports;

        Ok(())
    }

    pub fn calculate_rate(lamports: u64, seconds: u64) -> u64 {
        lamports / seconds
    }
}
#[program]
pub mod paystream {
    use super::*;
    pub fn create(
        ctx: Context<Create>,
        amount_in_lamports: u64,
        time_in_seconds: u64,
    ) -> ProgramResult {
        // Get the stream account
        let stream = &mut ctx.accounts.stream;
        // Check that we have enough lamports in the stream account to continue
        let dest_info = &mut stream.to_account_info();
        if **dest_info.lamports.borrow() < amount_in_lamports {
            return Err(ProgramError::InsufficientFunds);
        }

        // Populate the account with the following data
        stream.start_timestamp_in_seconds = Clock::get()?.unix_timestamp as u64;
        stream.amount_in_lamports = amount_in_lamports;
        stream.remaining_lamports = amount_in_lamports;
        stream.time_in_seconds = time_in_seconds;
        stream.payer = *ctx.accounts.payer.key;
        stream.receiver = *ctx.accounts.receiver.key;

        Ok(())
    }

    pub fn withdraw(ctx: Context<Withdraw>, amount_in_lamports: u64) -> ProgramResult {
        // Locate the stream account
        let stream = &mut ctx.accounts.stream;
        // Calculate what *can* be withdrawn
        // TODO Check that this won't panic on 0
        let lamport_per_second = stream.amount_in_lamports / stream.time_in_seconds;
        // How much time has passed
        let time_passed = Clock::get()?.unix_timestamp as u64 - stream.start_timestamp_in_seconds;
        // TODO Check that this won't overflow
        let maximum_amount = lamport_per_second * time_passed;
        let amount_in_lamports = if amount_in_lamports > maximum_amount {
            maximum_amount
        } else {
            amount_in_lamports
        };

        // Get the balance of the account and check that this can be withdrawn
        let stream_info = &mut stream.to_account_info();
        if **stream_info.lamports.borrow() < amount_in_lamports {
            return Err(ProgramError::InsufficientFunds);
        }

        // Debit the account with the lamports
        stream.remaining_lamports -= amount_in_lamports;
        // Get the account info for our receiver
        let receiver = &mut ctx.accounts.receiver;

        utils::transfer(
            &stream_info,
            &receiver.to_account_info(),
            amount_in_lamports,
        )?;

        Ok(())
    }

    pub fn cancel(ctx: Context<Cancel>) -> ProgramResult {
        // Obtain the stream
        let stream = &mut ctx.accounts.stream;
        // We check the remaining lamports as the balance would include rent
        if stream.remaining_lamports == 0 {
            return Err(ProgramError::InsufficientFunds);
        }

        let amount_to_return = stream.remaining_lamports;
        stream.remaining_lamports = 0;
        utils::transfer(
            &stream.to_account_info(),
            &ctx.accounts.payer,
            amount_to_return,
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct Create<'info> {
    #[account(init, payer = payer, space = 8 + 32 + 32 + 8 + 8 + 8 + 8)]
    pub stream: Account<'info, Stream>,
    pub payer: AccountInfo<'info>,
    pub receiver: AccountInfo<'info>,
    pub system_program: AccountInfo<'info>,
}

#[derive(Accounts)]
pub struct Withdraw<'info> {
    #[account(mut)]
    pub payer: AccountInfo<'info>,
    pub receiver: AccountInfo<'info>,
    #[account(
        mut,
        constraint =
            stream.payer == *payer.to_account_info().key ||
            stream.receiver == *receiver.to_account_info().key,
    )]
    pub stream: Account<'info, Stream>,
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    #[account(mut)]
    pub payer: AccountInfo<'info>,
    pub receiver: AccountInfo<'info>,
    #[account(
        mut,
        constraint =
            stream.payer == *payer.to_account_info().key ||
            stream.receiver == *receiver.to_account_info().key,
    )]
    pub stream: Account<'info, Stream>,
}

#[account]
pub struct Stream {
    payer: Pubkey,
    receiver: Pubkey,
    amount_in_lamports: u64,
    remaining_lamports: u64,
    start_timestamp_in_seconds: u64,
    time_in_seconds: u64,
}
