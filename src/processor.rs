use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};

use crate::{
    error::PaystreamError,
    instruction::PaystreamInstruction,
    state::{StreamAccount, StreamStatus},
};

pub struct Processor;
impl Processor {
    pub fn process(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        instruction_data: &[u8],
    ) -> ProgramResult {
        let instruction = PaystreamInstruction::unpack(instruction_data)?;

        match instruction {
            PaystreamInstruction::Create {
                payee_pubkey,
                payer_pubkey,
                amount,
                duration_in_slots,
            } => Self::create_stream(
                accounts,
                program_id,
                payee_pubkey,
                payer_pubkey,
                amount,
                duration_in_slots,
            ),
            PaystreamInstruction::Withdrawal { amount } => {
                Self::withdraw(accounts, program_id, amount)
            }
            PaystreamInstruction::Cancel {} => Self::cancel(accounts, program_id),
        }
    }

    fn create_stream(
        accounts: &[AccountInfo],
        program_id: &Pubkey,
        payee_pubkey: Pubkey,
        payer_pubkey: Pubkey,
        amount: u64,
        duration_in_slots: u64,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        // The stream account to store state of the stream
        let stream_account = next_account_info(accounts_iter)?;
        if stream_account.owner != program_id {
            msg!("[Paystream] Stream account is not owned by program {} != {}",
                stream_account.owner, program_id);
            return Err(ProgramError::IncorrectProgramId);
        }

        if !stream_account.is_writable {
            msg!("[Paystream] Stream account is not writable");
            return Err(ProgramError::InvalidInstructionData);
        }

        // Check the rent on the stream account
        let solana_rent = &Rent::from_account_info(next_account_info(accounts_iter)?)?;
        if !solana_rent.is_exempt(stream_account.lamports(), stream_account.data_len()) {
            msg!(
                "[Paystream] Not rent exempt. Balance: {}",
                stream_account.lamports(),
            );
            return Err(ProgramError::AccountNotRentExempt);
        }

        let clock_info = next_account_info(accounts_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;

        // Check that the stream has the correct balance
        let minimum_balance = solana_rent.minimum_balance(stream_account.data_len());
        msg!("[Paystream] minimum rent {}", minimum_balance);

        if solana_rent.minimum_balance(stream_account.data_len()) + amount
            > stream_account.lamports()
        {
            msg!(
                "[Paystream] Insufficient amount. Balance: {}",
                stream_account.lamports()
            );
            return Err(ProgramError::from(PaystreamError::InsufficientAmount));
        }

        // Initialise the stream with its initial state
        let mut stream_data = StreamAccount::try_from_slice(*stream_account.data.borrow())?;

        if stream_data.is_initialized() {
            msg!("[Paystream] Stream already initialised");

            return Err(ProgramError::AccountAlreadyInitialized);
        }

        stream_data.status = StreamStatus::Active as u8;
        stream_data.payee_pubkey = payee_pubkey;
        stream_data.payer_pubkey = payer_pubkey;
        stream_data.amount_in_lamports = amount;
        stream_data.remaining_lamports = amount;
        stream_data.start_timestamp_in_slots = clock.slot;
        stream_data.duration_in_slots = duration_in_slots;

        stream_data
            .serialize(&mut *stream_account.data.borrow_mut())?;

        msg!("[Paystream] Created stream account: {:?}", stream_data);

        Ok(())
    }

    fn withdraw(accounts: &[AccountInfo], program_id: &Pubkey, amount: u64) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let stream_account = next_account_info(accounts_iter)?;
        let payee_account = next_account_info(accounts_iter)?;
        let clock_info = next_account_info(accounts_iter)?;
        let clock = &Clock::from_account_info(clock_info)?;

        if stream_account.owner != program_id {
            msg!("[Paystream] Stream account is not owned by program");
            return Err(ProgramError::IncorrectProgramId);
        }

        if !stream_account.is_writable {
            msg!("[Paystream] Stream account is not writable");
            return Err(ProgramError::InvalidInstructionData);
        }

        if !payee_account.is_signer {
            msg!("[Paystream] Payee needs to be signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Initialise the stream with its initial state
        let mut stream_data = StreamAccount::try_from_slice(*stream_account.data.borrow())?;

        if !stream_data.is_initialized() {
            msg!("[Paystream] Stream is not initialised");
            return Err(ProgramError::UninitializedAccount);
        }

        if stream_data.payee_pubkey != *payee_account.key {
            msg!("[Paystream] Signer doesn't match payee");
            return Err(ProgramError::from(PaystreamError::InvalidPayee));
        }

        if stream_data.status != StreamStatus::Active as u8 {
            msg!("[Paystream] Stream is not active");
            return Err(ProgramError::from(PaystreamError::NotActive));
        }

        // Calculate what *can* be withdrawn
        // TODO Check that this won't panic on 0
        let lamport_per_second = stream_data.amount_in_lamports / stream_data.duration_in_slots;
        // How much time has passed
        let time_passed = clock.slot - stream_data.start_timestamp_in_slots;
        msg!("[Paystream] {} seconds have passed", time_passed);
        // TODO Check that this won't overflow
        let maximum_amount = lamport_per_second * time_passed;
        let amount = if amount > maximum_amount {
            maximum_amount
        } else {
            amount
        };

        msg!("[Paystream] Withdrawal of {} requested", amount);
         
        stream_data.remaining_lamports -= amount;
        stream_data.serialize(&mut &mut stream_account.data.borrow_mut()[..])?;

        **stream_account.try_borrow_mut_lamports()? -= amount;
        **payee_account.try_borrow_mut_lamports()? += amount;
        
        // let instruction =
        //     system_instruction::transfer(&stream_account.key, &payee_account.key, amount);
        // invoke(
        //     &instruction,
        //     &[
        //         system_account.clone(),
        //         payee_account.clone(),
        //         stream_account.clone(),
        //     ],
        // )?;
        
        msg!(
            "[Paystream] Withdrawal of {} from stream account: {:?}",
            amount,
            stream_data
        );

        Ok(())
    }

    fn cancel(accounts: &[AccountInfo], program_id: &Pubkey) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let stream_account = next_account_info(accounts_iter)?;
        let payee_account = next_account_info(accounts_iter)?;
        let payer_account = next_account_info(accounts_iter)?;
        
        if stream_account.owner != program_id {
            msg!("[Paystream] Stream account is not owned by program");
            return Err(ProgramError::IncorrectProgramId);
        }

        if !stream_account.is_writable {
            msg!("[Paystream] Stream account is not writable");
            return Err(ProgramError::InvalidInstructionData);
        }

        if !payee_account.is_signer {
            msg!("[Paystream] Payee needs to be signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Initialise the stream with its initial state
        let mut stream_data = StreamAccount::try_from_slice(*stream_account.data.borrow())?;

        if !stream_data.is_initialized() {
            msg!("[Paystream] Stream is not initialised");
            return Err(ProgramError::UninitializedAccount);
        }

        if stream_data.payee_pubkey != *payee_account.key {
            msg!("[Paystream] Signer doesn't match payee");
            return Err(ProgramError::from(PaystreamError::InvalidPayee));
        }

        if stream_data.payer_pubkey != *payer_account.key {
            msg!("[Paystream] Payer doesn't match");
            return Err(ProgramError::from(PaystreamError::InvalidPayer));
        }

        if stream_data.status != StreamStatus::Active as u8 {
            msg!("[Paystream] Stream is not active");
            return Err(ProgramError::from(PaystreamError::NotActive));
        }

        // Credit amount remaining back to payer
        // TODO clean up the rental dust
        msg!("[Paystream] Cancel requested");
        
        **stream_account.try_borrow_mut_lamports()? -= stream_data.remaining_lamports;
        **payer_account.try_borrow_mut_lamports()? += stream_data.remaining_lamports;

        // let instruction =
        //     system_instruction::transfer(&stream_account.key,
        //                                     payer_account.key,
        //                                     stream_data.amount);
        // invoke(
        //     &instruction,
        //     &[
        //         system_account.clone(),
        //         payer_account.clone(),
        //         stream_account.clone(),
        //     ],
        // )?;

        stream_data.remaining_lamports = 0;
        stream_data.status = StreamStatus::Terminated as u8;
        stream_data.serialize(&mut &mut stream_account.data.borrow_mut()[..])?;

        msg!(
            "[Paystream] Cancelled stream account: {:?}",
            stream_data
        );

        Ok(())
    }
}