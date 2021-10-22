//! Program state processor
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo,next_account_info},
    program_error::{PrintProgramError,ProgramError},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    system_instruction::create_account,
    program::{invoke,invoke_signed},
    pubkey::Pubkey,
    sysvar::{rent::Rent,fees::Fees,clock::Clock,Sysvar},
    program_pack::{IsInitialized},
    msg,
    system_program
};
use num_traits::FromPrimitive;
use crate::{
    instruction::{TokenInstruction,ProcessInitializeStream,Processwithdrawstream,ProcessTokenStream,ProcessTokenWithdrawStream,ProcessFundStream},
    state::{Escrow},
    error::TokenError
};
use crate::{spl_utils::{get_seeds,initialize_token_account,assert_keys_equal}};

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Function to initilize a stream
    pub fn process_initialize_stream(program_id: &Pubkey, accounts: &[AccountInfo], start_time: u64, end_time: u64, amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let lock_account_info = next_account_info(account_info_iter)?; // program derived address
        let system_program = next_account_info(account_info_iter)?; // system program
        let space_size = std::mem::size_of::<Escrow>() as u64;
        // Get the rent sysvar via syscall
        let rent = Rent::get()?; //
        // Since we are performing system_instruction source account must be signer.
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        // current time in unix time
        let now = Clock::get()?.unix_timestamp as u64; 
        msg!("End_time:{} start_time:{},Amount:{}",end_time,start_time,amount);
        if now > end_time{
            msg!("End time is already passed Now:{} End_time:{}",now,end_time);
            return Err(TokenError::TimeEnd.into());
        }
        assert_keys_equal(system_program::id(), *system_program.key)?;
        let create_account_instruction = create_account(
            source_account_info.key,
            lock_account_info.key,
            amount + rent.minimum_balance(std::mem::size_of::<Escrow>()),
            space_size,
            program_id,
        );
        invoke(
            &create_account_instruction,
            &[
                source_account_info.clone(),
                lock_account_info.clone(),
                system_program.clone(),
            ])?;
        // Sending transaction fee to destination account, to call withdraw instruction. 
        let fees = Fees::get()?;
        **lock_account_info.lamports.borrow_mut() = lock_account_info
        .lamports()
        .checked_sub(fees.fee_calculator.lamports_per_signature * 2)
        .unwrap();
        **dest_account_info.lamports.borrow_mut() = dest_account_info
        .lamports()
        .checked_add(fees.fee_calculator.lamports_per_signature * 2)
        .unwrap();

        let mut escrow = Escrow::try_from_slice(&lock_account_info.data.borrow())?;
        escrow.start_time = start_time;
        escrow.end_time = end_time;
        escrow.paused = 0;
        escrow.withdraw_limit = 0;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = amount;
        escrow.escrow = *lock_account_info.key;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut lock_account_info.data.borrow_mut()[..])?;
        Ok(())
    }
    //Function to stream tokens
    fn process_token_stream(program_id: &Pubkey, accounts: &[AccountInfo], start_time: u64, end_time: u64, amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let lock_account_info = next_account_info(account_info_iter)?; // Program pda
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let system_program = next_account_info(account_info_iter)?; // system address
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let associated_token_address = next_account_info(account_info_iter)?; // sender associated token address of token you are initializing 
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        // Get the rent sysvar via syscall
        let rent = Rent::get()?; //
        if token_program_info.key != &spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }    
        // Since we are performing system_instruction source account must be signer
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        // current time in unix time
        let now = Clock::get()?.unix_timestamp as u64; 
        if now > end_time{
            msg!("End time is already passed Now:{} End_time:{}",now,end_time);
            return Err(TokenError::TimeEnd.into());
        }
        let space_size = std::mem::size_of::<Escrow>() as u64;

        let program_pda = &get_seeds(source_account_info.key) as &[&[u8]];
        let (_account_address, bump_seed) =
        Pubkey::find_program_address(&[&source_account_info.key.to_bytes()], program_id); //program_pda
        let mut signers_seeds = program_pda.to_vec();
        let bump = &[bump_seed];
        signers_seeds.push(bump);
        msg!("{:?}",signers_seeds);
        if lock_account_info.data_is_empty() {
            // Creating pda to make associated token owner
            let create_account_instruction = create_account(
                source_account_info.key,
                lock_account_info.key,
                amount + rent.minimum_balance(std::mem::size_of::<Escrow>()),
                space_size,
                program_id,
            );
            invoke_signed(
                &create_account_instruction,
                &[
                    source_account_info.clone(),
                    lock_account_info.clone(),
                    system_program.clone(),
                ],&[&signers_seeds[..]])?;
        }
        initialize_token_account(
            token_program_info,
            token_mint_info,
            source_account_info,
            pda_associated_info,
            rent.minimum_balance(165)
            ,rent_info,system_program,
            lock_account_info,
            &signers_seeds[..]
        )?;
        invoke(
            &spl_token::instruction::transfer(
                token_program_info.key,
                associated_token_address.key,
                pda_associated_info.key,
                source_account_info.key,
                &[source_account_info.key],
                amount
            )?,
            &[
                token_program_info.clone(),
                associated_token_address.clone(),
                pda_associated_info.clone(),
                source_account_info.clone(),
            ],
        )?;
        let mut escrow = Escrow::try_from_slice(&lock_account_info.data.borrow())?;
        escrow.start_time = start_time;
        escrow.end_time = end_time;
        escrow.paused = 0;
        escrow.withdraw_limit = 0;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = amount;
        escrow.escrow = *lock_account_info.key;
        escrow.serialize(&mut &mut lock_account_info.data.borrow_mut()[..])?;
        Ok(())
    }
    //OnGoing Development 
    fn process_token_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let lock_account_info = next_account_info(account_info_iter)?; // assocaited token address for our program id 
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let associated_token_address = next_account_info(account_info_iter)?; // sender associated token address of token you are initializing 
        let receiver_associated_info = next_account_info(account_info_iter)?; // Pda to store data
        let token_sender_info = next_account_info(account_info_iter)?;
        if token_program_info.key != &spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }    
        // Since we are performing system_instruction source account must be signer
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = Escrow::try_from_slice(&lock_account_info.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        msg!("{}",amount);
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if now >= escrow.end_time {
            msg!("Stream has been successfully completed");
            allowed_amt = escrow.amount;
        }
        // let rent = &Rent::from_account_info(dest_account_info)?;
        msg!("{} allowed_amt",allowed_amt);
        if *source_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        if amount>allowed_amt {
            msg!("{} is not yet streamlined.",amount);
            return Err(ProgramError::InsufficientFunds);
        }
        msg!("{}",amount);
        if escrow.paused == 1 && amount > escrow.withdraw_limit {
            msg!("{} is your withdraw limit",escrow.withdraw_limit);
            return Err(ProgramError::InsufficientFunds);
        }
        let i = &get_seeds(token_sender_info.key) as &[&[u8]];
        let (account_address, bump_seed) =
        Pubkey::find_program_address(i, program_id);
        let mut signers_seeds = i.to_vec();
        let bump = &[bump_seed];
        signers_seeds.push(bump);
        msg!("Signer/Owner: {}", account_address);
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                associated_token_address.key,
                receiver_associated_info.key,
                lock_account_info.key,
                &[lock_account_info.key],
                amount
            )?,
            &[
                token_program_info.clone(),
                associated_token_address.clone(),
                receiver_associated_info.clone(),
                lock_account_info.clone(),
            ],&[&signers_seeds[..]],
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit = escrow.withdraw_limit-amount
        }
        escrow.amount = escrow.amount-amount;
        escrow.serialize(&mut &mut lock_account_info.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to withdraw from a stream
    fn process_withdraw_stream(accounts: &[AccountInfo],amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let dest_account_info = next_account_info(account_info_iter)?;
        let locked_fund = next_account_info(account_info_iter)?;
        // let data = locked_fund.try_borrow_mut_data()?;
        let mut escrow = Escrow::try_from_slice(&locked_fund.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        msg!("{}",amount);

        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if now >= escrow.end_time {
            msg!("Stream has been successfully completed");
            allowed_amt = escrow.amount;
        }
        // let rent = &Rent::from_account_info(dest_account_info)?;
        msg!("{} allowed_amt",allowed_amt);
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        if amount>allowed_amt {
            msg!("{} is not yet streamlined.",amount);
            return Err(ProgramError::InsufficientFunds);
        }
        msg!("{}",amount);
        if escrow.paused == 1 && amount > escrow.withdraw_limit {
            msg!("{} is your withdraw limit",escrow.withdraw_limit);
            return Err(ProgramError::InsufficientFunds);
        }
        **locked_fund.try_borrow_mut_lamports()? = locked_fund
        .lamports()
        .checked_sub(amount)
        .unwrap();
        
        **dest_account_info.try_borrow_mut_lamports()? = dest_account_info
        .lamports()
        .checked_add(amount)
        .unwrap();
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit = escrow.withdraw_limit-amount
        }
        escrow.amount = escrow.amount-amount;
        escrow.serialize(&mut &mut locked_fund.data.borrow_mut()[..])?;
        msg!("{:?}",escrow);
        Ok(())
    }
     /// Function to cancel a stream
    fn process_cancel_stream(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let locked_fund = next_account_info(account_info_iter)?;
        let mut escrow = Escrow::try_from_slice(&locked_fund.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        // Amount that recipient should receive.  
        let allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if now >= escrow.end_time {
            msg!("Stream already completed");
            return Err(TokenError::TimeEnd.into());
        }
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        let dest_account_amount = escrow.amount-allowed_amt;
        let source_account_amount = escrow.amount-dest_account_amount;

        **locked_fund.try_borrow_mut_lamports()? = locked_fund
        .lamports()
        .checked_sub(escrow.amount)
        .unwrap();
        
        // Send unstreamed fund to the sender. 
        **source_account_info.try_borrow_mut_lamports()? = source_account_info
        .lamports()
        .checked_add(source_account_amount)
        .unwrap();

        // Send streamed fund to the recipient. 
        **dest_account_info.try_borrow_mut_lamports()? = dest_account_info
        .lamports()
        .checked_add(dest_account_amount)
        .unwrap();
        escrow.amount = 0;
        escrow.serialize(&mut &mut locked_fund.data.borrow_mut()[..])?;
        Ok(())
    }
    //Function to pause a stream
    fn process_pause(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let locked_fund = next_account_info(account_info_iter)?;
        let mut escrow = Escrow::try_from_slice(&locked_fund.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        let allowed_amt = (((now - escrow.start_time) as f64) / ((escrow.end_time - escrow.start_time) as f64) * escrow.amount as f64) as u64;
        if now >= escrow.end_time {
            msg!("End time is already passed");
            return Err(TokenError::TimeEnd.into());
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *source_account_info.key != escrow.sender || *source_account_info.key != escrow.recipient { //Sender and Recipient both can pause or resume any transaction
            return Err(TokenError::EscrowMismatch.into());
        }
        escrow.paused = 1;
        escrow.withdraw_limit = allowed_amt;
        escrow.serialize(&mut &mut locked_fund.data.borrow_mut()[..])?;
        Ok(())
    }
    fn process_resume(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let locked_fund = next_account_info(account_info_iter)?;
        let now = Clock::get()?.unix_timestamp as u64;
        let mut escrow = Escrow::try_from_slice(&locked_fund.data.borrow())?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *source_account_info.key != escrow.sender || *source_account_info.key != escrow.recipient { //Sender and Recipient both can pause or resume any transaction
            return Err(TokenError::EscrowMismatch.into());
        }
        escrow.paused = 0;
        escrow.start_time =  now;
        escrow.serialize(&mut &mut locked_fund.data.borrow_mut()[..])?;
        Ok(())
    }
    //OnGoing Development
    fn process_fund_stream(accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let locked_fund = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;
        let mut escrow = Escrow::try_from_slice(&locked_fund.data.borrow())?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        // if *source_account_info.key != escrow.sender {
        //     return Err(TokenError::EscrowMismatch.into());
        // }

        invoke(
            &solana_program::system_instruction::transfer(
                source_account_info.key,
                locked_fund.key,
                amount
            ),
            &[
                source_account_info.clone(),
                locked_fund.clone(),
                system_program.clone()
            ],
        )?;
        // **source_account_info.try_borrow_mut_lamports()? = source_account_info
        // .lamports()
        // .checked_sub(amount)
        // .unwrap();
        
        // **locked_fund.try_borrow_mut_lamports()? = locked_fund
        // .lamports()
        // .checked_add(amount)
        // .unwrap();
        escrow.amount = escrow.amount+amount;
        escrow.serialize(&mut &mut locked_fund.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = TokenInstruction::unpack(input)?;
        match instruction {
            TokenInstruction::ProcessInitializeStream (ProcessInitializeStream{
                start_time,
                end_time,
                amount,
            }) => {
                msg!("Instruction: Processing Stream V1.0");
                Self::process_initialize_stream(program_id,accounts,start_time, end_time, amount)
            }
            TokenInstruction::Processwithdrawstream(Processwithdrawstream {
                amount,
            }) => {
                msg!("Instruction: Processing Withdraw V1.0");
                Self::process_withdraw_stream(accounts, amount)
            }
            TokenInstruction::Processcancelstream => {
                msg!("Instruction: Processing cancel V1.0");
                Self::process_cancel_stream(accounts)
            }
            TokenInstruction::ProcessTokenStream(ProcessTokenStream {
                start_time,
                end_time,
                amount,
            }) => {
                msg!("Instruction: Initializing USDC stream V1.0");
                Self::process_token_stream(program_id,accounts,start_time, end_time, amount)
            }
            TokenInstruction::ProcessPauseStream => {
                msg!("Instruction: Pausing stream");
                Self::process_pause(accounts)
            }
            TokenInstruction::ProcessResumeStream=> {
                msg!("Instruction: Resuming stream");
                Self::process_resume(accounts)
            }
            TokenInstruction::ProcessTokenWithdrawStream(ProcessTokenWithdrawStream {
                amount,
            }) => {
                msg!("Instruction: Processing Token Withdraw V1.0");
                Self::process_token_withdraw(program_id,accounts, amount)
            }
            TokenInstruction::ProcessFundStream(ProcessFundStream {
                amount,
            }) => {
                msg!("Instruction: Processing Token Withdraw V1.0");
                Self::process_fund_stream(accounts, amount)
            }
        }
    }
}
impl PrintProgramError for TokenError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            TokenError::TimeEnd => msg!("Error: Stream already completed"),
            TokenError::OwnerMismatch => msg!("Error: Owner does not match"),
            TokenError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            TokenError::EscrowMismatch => msg!("Error: Account not associated with this Escrow"),
            TokenError::InvalidInstruction => msg!("Error: Invalid instruction"),
            TokenError::AlreadyCancel => msg!("Error: Invalid instruction"),
            TokenError::AlreadyWithdrawn => msg!("Error: Paused stream, streamed amount already withdrawn"),
            TokenError::Overflow => msg!("Error: Operation overflowed"),
            TokenError::PublicKeyMismatch => msg!("Error: Public key mismatched"),
        }
    }
}