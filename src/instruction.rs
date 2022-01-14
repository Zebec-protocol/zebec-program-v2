//! Instruction types
use solana_program::{
    program_error::ProgramError,
};
use {borsh::{BorshDeserialize}};

use crate::{
    error::TokenError,
    state::{WhiteList,Multisig,Escrow_multisig,TokenEscrowMultisig}
};
use std::convert::TryInto;

/// Initialize stream data
pub struct ProcessSolStream{
    pub start_time: u64,
    pub end_time: u64,
    pub amount: u64,
}
/// Initialize token stream data
pub struct ProcessTokenStream{
    pub start_time: u64,
    pub end_time: u64,
    pub amount: u64,
}
pub struct ProcessSolWithdrawStream{
    /// Amount of fund
    pub amount: u64,
}
pub struct ProcessTokenWithdrawStream{
    /// Amount of fund
    pub amount: u64,
}
pub struct ProcessDepositSol{
    pub amount: u64,
}
pub struct ProcessDepositToken{
    pub amount: u64,
}
pub struct ProcessFundSol{
    pub end_time: u64,
    pub amount: u64,
}
pub struct ProcessFundToken{
    pub end_time: u64,
    pub amount: u64,
}
pub struct ProcessWithdrawSol{
    pub amount: u64,
}
pub struct ProcessWithdrawToken{
    pub amount: u64,
}
pub struct ProcessSwapSol{
    pub amount: u64,
}
pub struct ProcessSwapToken{
    pub amount: u64,
}
pub struct ProcessSolWithdrawStreamMultisig{
    /// Amount of fund
    pub amount: u64,
}
pub struct ProcessTokenWithdrawStreamMultisig{
    pub amount: u64,
}
pub enum TokenInstruction {
    ProcessSolStream(ProcessSolStream),
    ProcessSolWithdrawStream(ProcessSolWithdrawStream),
    ProcessCancelSolStream ,
    ProcessTokenStream(ProcessTokenStream),
    ProcessPauseSolStream,
    ProcessResumeSolStream,
    ProcessTokenWithdrawStream(ProcessTokenWithdrawStream),
    ProcessDepositSol(ProcessDepositSol),
    ProcessCancelTokenStream,
    ProcessPauseTokenStream,
    ProcessResumeTokenStream,
    ProcessDepositToken(ProcessDepositToken),
    ProcessFundSol(ProcessFundSol),
    ProcessFundToken(ProcessFundToken),
    ProcessWithdrawSol(ProcessWithdrawSol),
    ProcessWithdrawToken(ProcessWithdrawToken),
    CreateWhitelist{
        whitelist_v1:Multisig
    },
    ProcessSwapSol(ProcessSwapSol),
    ProcessSwapToken(ProcessSwapToken),
    SignedBy{
        whitelist_v2:WhiteList
    },
    ProcessSolMultiSigStream{whitelist_v3:Escrow_multisig},
    ProcessSolWithdrawStreamMultisig(ProcessSolWithdrawStreamMultisig),
    ProcessSolCancelStreamMultisig,
    ProcessPauseMultisigStream,
    ProcessResumeMultisigStream,
    ProcessRejectMultisigStream,
    ProcessSolTokenMultiSigStream{whitelist_v4:TokenEscrowMultisig},
    ProcessTokenWithdrawStreamMultisig(ProcessTokenWithdrawStreamMultisig),
    ProcessTokenCancelStreamMultisig,
    ProcessPauseTokenMultisigStream,
    ProcessResumeTokenMultisigStream,
    ProcessRejectTokenMultisigStream,
    SignedByToken{
        whitelist_v4:WhiteList
    }
}
impl TokenInstruction {
    /// Unpacks a byte buffer into a [TokenInstruction](enum.TokenInstruction.html).
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        use TokenError::InvalidInstruction;
        let (&tag, rest) = input.split_first().ok_or(InvalidInstruction)?;
        Ok(match tag {
            // Initialize stream instruction 
            0 => {
                let (start_time, rest) = rest.split_at(8);
                let (end_time, rest) = rest.split_at(8);
                let (amount, _rest) = rest.split_at(8);
                let start_time = start_time.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                let end_time = end_time.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessSolStream (ProcessSolStream{start_time,end_time,amount})
            }
            // Withdraw stream instruction 
            1 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessSolWithdrawStream (ProcessSolWithdrawStream{amount})
            }
            // Cancel stream instruction 
            2 => {
                Self:: ProcessCancelSolStream
            }
             // Initialize Token stream 
             3 => {
                let (start_time, rest) = rest.split_at(8);
                let (end_time, rest) = rest.split_at(8);
                let (amount, _rest) = rest.split_at(8);
                let start_time = start_time.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                let end_time = end_time.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessTokenStream (ProcessTokenStream{start_time,end_time,amount})
            }
            4 =>{
                Self::ProcessPauseSolStream
            }
            5 =>{
                Self::ProcessResumeSolStream
            }
            6 =>{
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessTokenWithdrawStream (ProcessTokenWithdrawStream{amount})
            }
            7 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessDepositSol(ProcessDepositSol{amount})
            }
            8 => {
                Self:: ProcessCancelTokenStream
            }
            9 => {
                Self:: ProcessPauseTokenStream
            }
            10 => {
                Self:: ProcessResumeTokenStream
            }
            11 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessDepositToken(ProcessDepositToken{amount})
            }
            12 => {
                let (end_time, rest) = rest.split_at(8);
                let (amount, _rest) = rest.split_at(8);
                let end_time = end_time.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessFundSol(ProcessFundSol{end_time,amount})
            }
            13 => {
                let (end_time, rest) = rest.split_at(8);
                let (amount, _rest) = rest.split_at(8);
                let end_time = end_time.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessFundToken(ProcessFundToken{end_time,amount})
            }
            14 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessWithdrawSol(ProcessWithdrawSol{amount})
            }
            15 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessWithdrawToken(ProcessWithdrawToken{amount})
            }
            16 => {
                Self::CreateWhitelist{whitelist_v1:Multisig::try_from_slice(rest)?}
            },
            17 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessSwapSol(ProcessSwapSol{amount})
            },
            18 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessSwapToken(ProcessSwapToken{amount})
            },
            19 =>{
                Self::SignedBy{whitelist_v2:WhiteList::try_from_slice(rest)?}
            }
            20 => {
                Self::ProcessSolMultiSigStream{whitelist_v3:Escrow_multisig::try_from_slice(rest)?}
            }
            21 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessSolWithdrawStreamMultisig (ProcessSolWithdrawStreamMultisig{amount})
            }
            22 => {
                Self:: ProcessSolCancelStreamMultisig
            }
            23 => {
                Self:: ProcessPauseMultisigStream
            }
            24 => {
                Self:: ProcessResumeMultisigStream
            }
            25 => {
                Self:: ProcessRejectMultisigStream
            }
            26 => {
                Self::ProcessSolTokenMultiSigStream{whitelist_v4:TokenEscrowMultisig::try_from_slice(rest)?}
            }
            27 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessTokenWithdrawStreamMultisig (ProcessTokenWithdrawStreamMultisig{amount})
            }
            28 => {
                Self:: ProcessTokenCancelStreamMultisig
            }
            29 => {
                Self:: ProcessPauseTokenMultisigStream
            }
            30 => {
                Self:: ProcessResumeTokenMultisigStream
            }
            31 => {
                Self:: ProcessRejectTokenMultisigStream
            }
            32 =>{
                Self::SignedByToken{whitelist_v4:WhiteList::try_from_slice(rest)?}
            }

            _ => return Err(TokenError::InvalidInstruction.into()),
        })
    }
}