//! Error types

use num_derive::FromPrimitive;
use solana_program::{decode_error::DecodeError, program_error::ProgramError};
use thiserror::Error;

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TokenError {
    // 0
    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    /// Account not associated with this Escrow.
    #[error("Account not associated with this Escrow")]
    EscrowMismatch,
    /// Owner does not match.
    #[error("Owner does not match")]
    OwnerMismatch,
    #[error("Invalid instruction")]
    InvalidInstruction,
    /// Streams whose time is already passed
    #[error("Stream already completed")]
    TimeEnd,
    #[error("Stream already cancelled")]
    AlreadyCancel,
    #[error("Paused stream, streamed amount already withdrawn")]
    AlreadyWithdrawn,
    /// Operation overflowed
    #[error("Operation overflowed")]
    Overflow,
    // Publck Key Check error
    #[error("Public key mismatched")]
    PublicKeyMismatch,
}
impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for TokenError {
    fn type_of() -> &'static str {
        "TokenError"
    }
}