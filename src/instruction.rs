use solana_program::{program_error::ProgramError, pubkey::Pubkey, instruction::Instruction};
use std::convert::TryInto;
use solana_program::instruction::AccountMeta;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::sysvar;

#[derive(Debug, BorshSerialize, BorshDeserialize, )]
pub enum PaystreamInstruction {
    /// Initialize the stream payment
    ///
    /// Accounts expected:
    /// 0. `[writable]` The stream account created to manage state across 2 parties; owned by program id.
    /// 1. `[]` Sysvar Rent Account to validate rent exemption (SYSVAR_RENT_PUBKEY)
    Create {
        payee_pubkey: Pubkey,
        payer_pubkey: Pubkey,
        amount: u64,
        duration_in_seconds: u64,
    },

    /// Withdraw amount from stream
    ///
    /// Accounts expected:
    /// 0. `[writable]` The stream account created to manage state across 2 parties; owned by program id.
    /// 1. `[signer]` Payee account (keypair)
    /// 2. `[]` System program account
    Withdrawal { amount: u64 },

    /// Cancel stream payment
    /// Accounts expected:
    /// 0. `[writable]` The stream account created to manage state across 2 parties; owned by program id.
    /// 1. `[signer]` Payee account (keypair)
    /// 2. `[]` Payer (Owner) account (public key)
    /// 3. `[]` System program account
    Cancel {},
}

impl PaystreamInstruction {
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let (tag, rest) = input
            .split_first()
            .ok_or(ProgramError::InvalidInstructionData)?;

        Ok(match tag {
            0 => {
                let payee_pubkey: Pubkey = Pubkey::new(&rest[..32]);
                let payer_pubkey: Pubkey = Pubkey::new(&rest[32..64]);
                let amount: u64 = Self::unpack_u64(&rest, 64)?;
                let duration_in_seconds: u64 = Self::unpack_u64(&rest, 72)?;
                
                Self::Create {
                    payee_pubkey,
                    payer_pubkey,
                    amount,
                    duration_in_seconds,
                }
            }
            1 => {
                let amount: u64 = Self::unpack_u64(&rest, 0)?;
                Self::Withdrawal { amount }
            }
            2 => Self::Cancel {},
            _ => return Err(ProgramError::InvalidInstructionData),
        })
    }

    fn unpack_u64(input: &[u8], start: usize) -> Result<u64, ProgramError> {
        let value = input
            .get(start..8 + start)
            .and_then(|slice| slice.try_into().ok())
            .map(u64::from_le_bytes)
            .ok_or(ProgramError::InvalidInstructionData)?;
        Ok(value)
    }
}

/// Create instruction
pub fn create(
    program_id: Pubkey,
    instruction_data: PaystreamInstruction,
    stream_account_key: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = instruction_data.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(stream_account_key, true),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    Ok(Instruction {
        program_id,
        accounts,
        data,
    })
}

pub fn withdrawal(
    program_id: Pubkey,
    instruction_data: PaystreamInstruction,
    stream_account_key: Pubkey,
    payee_account_key: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = instruction_data.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(stream_account_key, true),
        AccountMeta::new(payee_account_key, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction{
        program_id,
        accounts,
        data,
    })
}

pub fn cancel(
    program_id: Pubkey,
    instruction_data: PaystreamInstruction,
    stream_account_key: Pubkey,
    payee_account_key: Pubkey,
    payer_account_key: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = instruction_data.try_to_vec().unwrap();
    let accounts = vec![
        AccountMeta::new(stream_account_key, true),
        AccountMeta::new(payee_account_key, true),
        AccountMeta::new(payer_account_key, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    Ok(Instruction{
        program_id,
        accounts,
        data,
    })
}