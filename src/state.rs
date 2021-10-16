//! State transition types
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
    account_info::AccountInfo
};

/// Initializeing stream states
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, Clone, Copy, Debug, Default, PartialEq)]
pub struct Escrow{
    pub start_time: u64,
    pub end_time: u64,
    pub paused: u64,
    pub withdraw_limit: u64,
    pub amount: u64,
    pub sender:   Pubkey,
    pub recipient: Pubkey,
    pub escrow: Pubkey,
}

pub struct TokenInitializeAccountParams<'a> {
    pub account: AccountInfo<'a>,
    pub mint: AccountInfo<'a>,
    pub owner: AccountInfo<'a>,
    pub rent: AccountInfo<'a>,
    pub token_program: AccountInfo<'a>,
}

pub struct TokenTransferParams<'a: 'b, 'b> {
    pub source: AccountInfo<'a>,
    pub destination: AccountInfo<'a>,
    pub amount: u64,
    pub authority: AccountInfo<'a>,
    pub authority_signer_seeds: &'b [&'b [u8]],
    pub token_program: AccountInfo<'a>,
}