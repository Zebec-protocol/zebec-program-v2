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
    #[error("Time has already passed")]
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
    // Already Paused
    #[error("Transaction is already paused")]
    AlreadyPaused,
    // Already Resumed or not paused
    #[error("Transaction is not paused")]
    AlreadyResumed,
    // Already Resumed or not paused
    #[error("Stream Already Created")]
    StreamAlreadyCreated,
    // Already Resumed or not paused
    #[error("Stream has not been started")]
    StreamNotStarted,
    // Withdraw more than streamed money
    #[error("Cannot withdraw streaming amount")]
    StreamedAmt,
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