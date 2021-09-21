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
    pub withdrawn: u64,
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
// impl Escrow {
//     /// Checks if account is frozen
//     pub fn is_frozen(&self) -> bool {
//         self.state == AccountState::Frozen
//     }
// }
// impl IsInitialized for Escrow {
//     fn is_initialized(&self) -> bool {
//         self.state != AccountState::Uninitialized
//     }
// }
// impl Sealed for Escrow {}
// impl Pack for Escrow {
//     const LEN: usize = 100;
//     fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
//         msg!("{:?}",src);
//         let src = array_ref![src, 0, 100];
//         let (mint, owner,state) =
//             array_refs![src, 32,32,36];
//         Ok(Escrow {
//             sender: Pubkey::new_from_array(*mint),
//             recipient: Pubkey::new_from_array(*owner),
//             state: AccountState::try_from_primitive(state[0])
//                 .or(Err(ProgramError::InvalidAccountData))?,
//         })    }
//     fn pack_into_slice(&self, dst: &mut [u8]) {
//         let dst = array_mut_ref![dst, 0, 100];
//         let (mint_dst,owner_dst,state_dst) = mut_array_refs![dst, 32, 32,36];
//         let &Escrow {ref sender, ref recipient,state} = self;
//         mint_dst.copy_from_slice(sender.as_ref());
//         owner_dst.copy_from_slice(recipient.as_ref());
//         state_dst[0] = state as u8;
//     }
// }

// /// Account state.
// #[repr(u8)]
// #[derive(Clone, Copy, Debug, PartialEq, TryFromPrimitive)]
// pub enum AccountState {
//     /// Account is not yet initialized
//     Uninitialized,
//     /// Account is initialized; the account owner and/or delegate may perform permitted operations
//     /// on this account
//     Initialized,
//     /// Account has been frozen by the mint freeze authority. Neither the account owner nor
//     /// the delegate are able to perform operations on this account.
//     Frozen,
// }

// impl Default for AccountState {
//     fn default() -> Self {
//         AccountState::Uninitialized
//     }
// }