//! State transition types
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    pubkey::Pubkey,
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