use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum PaystreamError {
    /// Insufficient amount for stream
    #[error("Insufficient amount for stream")]
    InsufficientAmount,
    #[error("Stream not active")]
    NotActive,
    #[error("Invalid payee requested")]
    InvalidPayee,
    #[error("Invalid payer")]
    InvalidPayer,
}

impl From<PaystreamError> for ProgramError {
    fn from(e: PaystreamError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
