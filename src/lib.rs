pub mod state;
pub mod processor;
pub mod instruction;
pub mod error;
pub mod utils;
use crate::{
    processor::Processor,
    error::TokenError
};
use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult,
    pubkey::Pubkey,
    instruction::{AccountMeta, Instruction},
    program_error::PrintProgramError,
};
pub const PREFIX: &str = "withdraw_sol";
pub const PREFIX_TOKEN: &str = "withdraw_token";
pub const PREFIXMULTISIG: &str = "withdraw_multisig_sol";
pub const PREFIXMULTISIGSAFE: &str = "multisig_safe";

/// Minimum number of multisignature signers (min N)
pub const MIN_SIGNERS: usize = 1;
/// Maximum number of multisignature signers (max N)
pub const MAX_SIGNERS: usize = 11;

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    if let Err(error) = Processor::process(program_id, accounts, input) {
        // catch the error so we can print it
        error.print::<TokenError>();
        return Err(error);
    }
    Ok(())
}