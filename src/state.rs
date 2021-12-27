//! State transition types
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
    program_error::{ProgramError},
    account_info:: AccountInfo,
    borsh::try_from_slice_unchecked,
    msg
};
use crate::{
    MAX_SIGNERS
};
use std::mem::size_of;

/// Initializeing solana stream states
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Escrow{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
}
impl Escrow {
    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64 
    }
}
/// Initializeing token stream state
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct TokenEscrow{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey
}
impl TokenEscrow {
    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64 
    }
}
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Withdraw{
    pub amount: u64,
}

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct TokenWithdraw{
    pub amount: u64,
}

/// Multisignature data.
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Multisig {
    /// Number of signers required
    /// Number of valid signers
    // pub n: u8,
    /// Is `true` if this structure has been initialized
    // pub is_initialized: bool,
    /// Signer public keys
    pub signers: Vec<WhiteList>,
    pub m: u8,
    // PDA save
    // pub pda: Pubkey
}
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct WhiteList{
    pub address: Pubkey,
    pub counter:u8,
}
/// Check is a token state is initialized
pub trait IsInitialized {
    /// Is initialized
    fn is_initialized(&self) -> bool;
}
impl Multisig {
    pub fn from_account(account:&AccountInfo)-> Result<Multisig, ProgramError> {
            let md: Multisig =try_from_slice_unchecked(&account.data.borrow_mut())?;
            Ok(md)
    }
}

/// Initializeing solana stream states
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Escrow_multisig{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub signed_by: Vec<WhiteList>,
}
impl Escrow_multisig {
    pub fn from_account(account:&AccountInfo)-> Result<Escrow_multisig, ProgramError> {
        let md: Escrow_multisig =try_from_slice_unchecked(&account.data.borrow_mut())?;
        Ok(md)
    }

    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64 
    }
}