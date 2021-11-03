//! Instruction types
use solana_program::{
    program_error::ProgramError,
};
use crate::{
    error::TokenError,

};
use std::convert::TryInto;
/// Initialize stream data
pub struct ProcessInitializeStream{
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
pub struct Processwithdrawstream{
    /// Amount of fund
    pub amount: u64,
}
pub struct ProcessTokenWithdrawStream{
    /// Amount of fund
    pub amount: u64,
}
pub struct ProcessFundStream{
    pub amount: u64,
}
pub enum TokenInstruction {
    ProcessInitializeStream(ProcessInitializeStream),
    Processwithdrawstream(Processwithdrawstream),
    ProcessCancelStream ,
    ProcessTokenStream(ProcessTokenStream),
    ProcessPauseStream,
    ProcessResumeStream,
    ProcessTokenWithdrawStream(ProcessTokenWithdrawStream),
    ProcessFundStream(ProcessFundStream),
    ProcessCancelToken,
    ProcessPauseToken,
    ProcessResumeToken
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
                Self::ProcessInitializeStream (ProcessInitializeStream{start_time,end_time,amount})
            }
            // Withdraw stream instruction 
            1 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::Processwithdrawstream (Processwithdrawstream{amount})
            }
            // Cancel stream instruction 
            2 => {
                Self:: ProcessCancelStream
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
                Self::ProcessPauseStream
            }
            5 =>{
                Self::ProcessResumeStream
            }
            6 =>{
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessTokenWithdrawStream (ProcessTokenWithdrawStream{amount})
            }
            7 => {
                let (amount, _rest) = rest.split_at(8);
                let amount = amount.try_into().map(u64::from_le_bytes).or(Err(InvalidInstruction))?;
                Self::ProcessFundStream (ProcessFundStream{amount})
            }
            8 => {
                Self:: ProcessCancelToken
            }
            9 => {
                Self:: ProcessPauseToken
            }
            10 => {
                Self:: ProcessResumeToken
            }
            _ => return Err(TokenError::InvalidInstruction.into()),
        })
    }
}