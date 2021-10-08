use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    program_pack::{IsInitialized, Sealed},
    pubkey::Pubkey,
};

/// Rent Share Account state stored in the Agreement Account
#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct StreamAccount {
    pub status: u8,
    pub payee_pubkey: Pubkey,
    pub payer_pubkey: Pubkey,
    pub amount_in_lamports: u64,
    pub remaining_lamports: u64,
    pub duration_in_slots: u64,
    pub start_timestamp_in_slots: u64,
}

impl Sealed for StreamAccount {}

impl IsInitialized for StreamAccount {
    fn is_initialized(&self) -> bool {
        self.status != StreamStatus::Uninitialized as u8
    }
}

impl StreamAccount {
    pub fn is_complete(&self) -> bool {
        self.status == StreamStatus::Completed as u8
    }

    pub fn is_terminated(&self) -> bool {
        self.status == StreamStatus::Terminated as u8
    }
}

#[derive(Copy, Clone)]
pub enum StreamStatus {
    Uninitialized = 0,
    Active,
    Completed,
    Terminated,
}
