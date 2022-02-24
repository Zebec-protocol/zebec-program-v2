//! State transition types
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
    program_error::{ProgramError},
    account_info:: AccountInfo,
    borsh::try_from_slice_unchecked,
};

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
/// Initializeing solana stream states
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct Stream{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub withdrawn: u64,
    pub paused_at: u64
}
impl Stream {
    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64
    }
}
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
    pub token_mint: Pubkey,
}
impl TokenEscrow {
    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64 
    }
}
/// Initializeing token stream state
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct StreamToken{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub withdrawn: u64,
    pub paused_at: u64,
}
impl StreamToken {
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
    pub signers: Vec<WhiteList>,
    pub m: u8,
    pub multisig_safe: Pubkey
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

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct EscrowMultisig{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub signed_by: Vec<WhiteList>,
    pub multisig_safe: Pubkey,
    pub can_cancel: bool,
}
/// Initializeing solana stream states
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct StreamMultisig{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub signed_by: Vec<WhiteList>,
    pub multisig_safe: Pubkey,
    pub can_cancel: bool,
    pub withdrawn: u64,
    pub paused_at: u64,
}
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct TokenEscrowMultisig{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub signed_by: Vec<WhiteList>,
    pub multisig_safe: Pubkey,
    pub can_cancel: bool,
}
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct TokenStreamMultisig{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub token_mint: Pubkey,
    pub signed_by: Vec<WhiteList>,
    pub multisig_safe: Pubkey,
    pub can_cancel: bool,
    pub withdrawn: u64,
    pub paused_at: u64,
}
impl TokenEscrowMultisig {
    pub fn from_account(account:&AccountInfo)-> Result<TokenEscrowMultisig, ProgramError> {
        let md: TokenEscrowMultisig =try_from_slice_unchecked(&account.data.borrow_mut())?;
        Ok(md)
    }

    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64 
    }
}
impl StreamMultisig {
    pub fn from_account(account:&AccountInfo)-> Result<StreamMultisig, ProgramError> {
        let md: StreamMultisig =try_from_slice_unchecked(&account.data.borrow_mut())?;
        Ok(md)
    }

    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64 
    }
}
impl TokenStreamMultisig {
    pub fn from_account(account:&AccountInfo)-> Result<TokenStreamMultisig, ProgramError> {
        let md: TokenStreamMultisig =try_from_slice_unchecked(&account.data.borrow_mut())?;
        Ok(md)
    }

    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64 
    }
}
impl EscrowMultisig {
    pub fn from_account(account:&AccountInfo)-> Result<EscrowMultisig, ProgramError> {
        let md: EscrowMultisig =try_from_slice_unchecked(&account.data.borrow_mut())?;
        Ok(md)
    }

    pub fn allowed_amt(&self, now: u64) -> u64 {
        (
        ((now - self.start_time) as f64) / ((self.end_time - self.start_time) as f64) * self.amount as f64
        ) as u64 
    }
}
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct SolTransfer{
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub signed_by: Vec<WhiteList>,
    pub multisig_safe: Pubkey,
    pub amount : u64,
}
impl SolTransfer {
    pub fn from_account(account:&AccountInfo)-> Result<SolTransfer, ProgramError> {
        let md: SolTransfer =try_from_slice_unchecked(&account.data.borrow_mut())?;
        Ok(md)
    }
}
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub struct TokenTransfer{
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub signed_by: Vec<WhiteList>,
    pub multisig_safe: Pubkey,
    pub amount : u64,
    pub token_mint : Pubkey
}
impl TokenTransfer {
    pub fn from_account(account:&AccountInfo)-> Result<TokenTransfer, ProgramError> {
        let md: TokenTransfer =try_from_slice_unchecked(&account.data.borrow_mut())?;
        Ok(md)
    }
}