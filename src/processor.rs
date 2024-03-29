//! Program state processor
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{AccountInfo,next_account_info},
    program_error::{PrintProgramError,ProgramError},
    decode_error::DecodeError,
    entrypoint::ProgramResult,
    program::{invoke,invoke_signed},
    pubkey::Pubkey,
    sysvar::{rent::Rent,clock::Clock,Sysvar},
    msg,
    system_program,
};
use num_traits::FromPrimitive;
use crate::{
    instruction::{
        TokenInstruction,
        ProcessSolStream,
        ProcessSolWithdrawStream,
        ProcessTokenStream,
        ProcessTokenWithdrawStream,
        ProcessDepositSol,
        ProcessDepositToken,
        ProcessFundSol,
        ProcessFundToken,
        ProcessWithdrawToken,
        ProcessWithdrawSol,
        ProcessSwapSol,
        ProcessSwapToken,
        ProcessSolWithdrawStreamMultisig,
        ProcessTokenWithdrawStreamMultisig,
    },
    state::{Stream,StreamToken,StreamMultisig,TokenStreamMultisig,Escrow,TokenEscrow,Withdraw,TokenWithdraw,Multisig,WhiteList,TokenEscrowMultisig,EscrowMultisig,SolTransfer,TokenTransfer},
    error::{TokenError},

    utils::{
        assert_keys_equal,
        create_pda_account,
        get_master_address_and_bump_seed,
        create_transfer,
        get_withdraw_data_and_bump_seed,
        create_pda_account_signed,
        get_multisig_data_and_bump_seed,
        get_token_withdraw_data_and_bump_seed,
        get_token_balance
    },
    PREFIX,
    PREFIXMULTISIG,
    PREFIX_TOKEN,
    PREFIXMULTISIGSAFE,
};
use spl_associated_token_account::get_associated_token_address;
use std::str::FromStr;

/// Program state handler.
pub struct Processor {}
impl Processor {
    /// Function to initialize a solana
    pub fn process_sol_stream(program_id: &Pubkey, accounts: &[AccountInfo], start_time: u64, end_time: u64, amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda_data = next_account_info(account_info_iter)?; // pda data storage
        let withdraw_data = next_account_info(account_info_iter)?; // pda data storage
        let system_program = next_account_info(account_info_iter)?; // system program
        // Get the rent sysvar via syscall
        let rent = Rent::get()?; //
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        // current time in unix time
        let now = Clock::get()?.unix_timestamp as u64; 
        if now >= end_time{
            return Err(TokenError::TimeEnd.into());
        }
        if start_time >= end_time {
            return Err(TokenError::InvalidInstruction.into());
        }
        assert_keys_equal(system_program::id(), *system_program.key)?;
        if !pda_data.data_is_empty(){
            return Err(TokenError::StreamAlreadyCreated.into());
        }
        let (account_address, bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIX,
            source_account_info.key,
            program_id,
        );
        let withdraw_data_signer_seeds: &[&[_]] = &[
            PREFIX.as_bytes(),
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        assert_keys_equal(*withdraw_data.key,account_address )?;
        if withdraw_data.data_is_empty(){
            let transfer_amount =  rent.minimum_balance(std::mem::size_of::<Withdraw>());
            create_pda_account_signed(
                source_account_info,
                transfer_amount,
                std::mem::size_of::<Withdraw>(),
                program_id,
                system_program,
                withdraw_data,
                withdraw_data_signer_seeds
            )?;
        }
        let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount += amount;
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;

        let transfer_amount =  rent.minimum_balance(std::mem::size_of::<Stream>());
        // Sending transaction fee to recipient. So, he can withdraw the streamed fund
        create_pda_account( 
            source_account_info,
            transfer_amount,
            std::mem::size_of::<Stream>(),
            program_id,
            system_program,
            pda_data
        )?;
        let mut escrow = Stream::try_from_slice(&pda_data.data.borrow())?;
        escrow.start_time = start_time;
        escrow.end_time = end_time;
        escrow.paused = 0;
        escrow.withdraw_limit = 0;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = amount;
        escrow.withdrawn = 0 ;
        escrow.paused_at = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    // This function will be removed in future
    pub fn process_sol_withdraw_stream_deprecated(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // stream initiator address
        let dest_account_info = next_account_info(account_info_iter)?; // stream receiver
        let pda = next_account_info(account_info_iter)?; // locked fund
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let withdraw_data = next_account_info(account_info_iter)?; // withdraw data 
        let system_program = next_account_info(account_info_iter)?; // system program id 
        let fee_account =  next_account_info(account_info_iter)?; // 0.25 fee account
        
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let (account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIX,
            source_account_info.key,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        msg!("{:?}",pda_data.data_len());
        let mut escrow = Escrow::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        if now <= escrow.start_time {
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            allowed_amt = escrow.amount;
        }
        msg!("You can withdraw {}",allowed_amt);
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        // Checking if amount is greater than allowed amount
        if amount>allowed_amt {
            return Err(ProgramError::InsufficientFunds);
        }
        // Checking if paused stream is greater than withdraw limit
        if escrow.paused == 1 && amount > escrow.withdraw_limit {
            return Err(ProgramError::InsufficientFunds);
        }
        let (_account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let comission: u64 = 25*amount/10000; 
        let receiver_amount:u64=amount-comission;
        
        create_transfer(
            pda,
            fee_account,
            system_program,
            comission,
            pda_signer_seeds
        )?;
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            receiver_amount,
            pda_signer_seeds
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit -= amount;
        }
        escrow.amount -= amount;
        // Closing account to send rent to sender
        if escrow.amount == 0 { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount -= amount;
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to withdraw solana
    fn process_sol_withdraw_stream(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // stream initiator address
        let dest_account_info = next_account_info(account_info_iter)?; // stream receiver
        let pda = next_account_info(account_info_iter)?; // locked fund
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let withdraw_data = next_account_info(account_info_iter)?; // withdraw data 
        let system_program = next_account_info(account_info_iter)?; // system program id 
        let fee_account =  next_account_info(account_info_iter)?; // 0.25 fee account
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let (account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIX,
            source_account_info.key,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = Stream::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        if now <= escrow.start_time {
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = escrow.allowed_amt(now);
        msg!("{}",allowed_amt);
        if now >= escrow.end_time {
            allowed_amt = escrow.amount;
        }
        allowed_amt -=  escrow.withdrawn;
        msg!("You can withdraw {}",allowed_amt);
        // if !dest_account_info.is_signer {
        //     return Err(ProgramError::MissingRequiredSignature); 
        // }
        msg!("{:?}",escrow);
        if *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        // Checking if amount is greater than allowed amount
        if amount>allowed_amt {
            return Err(ProgramError::InsufficientFunds);
        }
        // Checking if paused stream is greater than withdraw limit
        if escrow.paused == 1 && amount > escrow.withdraw_limit {
            return Err(ProgramError::InsufficientFunds);
        }
        let (_account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let comission: u64 = 25*amount/10000; 
        let receiver_amount:u64=amount-comission;
        
        create_transfer(
            pda,
            fee_account,
            system_program,
            comission,
            pda_signer_seeds
        )?;
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            receiver_amount,
            pda_signer_seeds
        )?;
        if escrow.paused == 1{
            escrow.withdraw_limit -= amount;
        }
        escrow.withdrawn += amount;
        // escrow.amount -= amount;
        // Closing account to send rent to sender
        if escrow.withdrawn == escrow.amount { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount = withdraw_state.amount.checked_sub(amount).unwrap();
        msg!("{:?}",withdraw_state);
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
     /// Function to cancel solana streaming
     fn process_cancel_sol_stream(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?; // locked fund
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let withdraw_data = next_account_info(account_info_iter)?; // withdraw data 
        let system_program = next_account_info(account_info_iter)?; // system program id 
        let fee_account = next_account_info(account_info_iter)?;

        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let (account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIX,
            source_account_info.key,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = Stream::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        // Amount that recipient should receive.  
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            msg!("Stream already completed");
            return Err(TokenError::StreamNotStarted.into());
        }
        if now < escrow.start_time {
            allowed_amt = 0;
        }
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        let (_account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        allowed_amt = allowed_amt.checked_sub(escrow.withdrawn).unwrap();
        let comission: u64 = 25*allowed_amt/10000; 
        let receiver_amount:u64=allowed_amt-comission;
        create_transfer(
            pda,
            fee_account,
            system_program,
            comission,
            pda_signer_seeds
        )?;
        // Sending streamed payment to receiver 
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            receiver_amount,
            pda_signer_seeds
        )?;
        let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount -= escrow.amount.checked_sub(escrow.withdrawn).unwrap();
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        // We don't need to send remaining funds to sender, its already in sender master pda account which he can withdraw with withdraw function
        // Closing account to send rent to sender
        let dest_starting_lamports = source_account_info.lamports();
        **source_account_info.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(pda_data.lamports())
            .ok_or(TokenError::Overflow)?;

        **pda_data.lamports.borrow_mut() = 0;
        escrow.amount = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    //Function to pause solana stream
    fn process_pause_sol_stream(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id {
            return Err(ProgramError::InvalidArgument);
        }
        let mut escrow = Stream::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        let allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            return Err(TokenError::TimeEnd.into());
        }
        if now <= escrow.start_time{
            return Err(TokenError::StreamNotStarted.into());
        }
        if escrow.start_time >= escrow.end_time {
            return Err(TokenError::InvalidInstruction.into());
        }
        // Both sender and receiver can pause / resume stream
        if !source_account_info.is_signer && !dest_account_info.is_signer{ 
            return Err(ProgramError::MissingRequiredSignature); 
        }

        if *source_account_info.key != escrow.sender || *dest_account_info.key != escrow.recipient { 
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==1{
            return Err(TokenError::AlreadyPaused.into());
        }
        escrow.paused = 1;
        escrow.withdraw_limit = allowed_amt;
        escrow.paused_at = now;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    //Function to resume solana stream
    fn process_resume_sol_stream(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;
        if *pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let now = Clock::get()?.unix_timestamp as u64;
        let mut escrow = Stream::try_from_slice(&pda_data.data.borrow())?;
        // Both sender and receiver can pause / resume stream
        if !source_account_info.is_signer && !dest_account_info.is_signer{ 
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *source_account_info.key != escrow.sender || *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==0{
            return Err(TokenError::AlreadyResumed.into());
        }
        let time_spent = now - escrow.paused_at;
        let paused_start_time = escrow.start_time + time_spent;
        let paused_amount = escrow.allowed_amt(paused_start_time);
        let current_amount = escrow.allowed_amt(now);
        let total_amount_to_sent = current_amount - paused_amount;
        msg!("total_amount_to_sent: {},  paused_amount:{}, current_amount:{}",total_amount_to_sent,paused_amount,current_amount);
        escrow.withdrawn = escrow.withdrawn + total_amount_to_sent;
        escrow.paused = 0;
        escrow.paused_at = 0;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    // Function to initialize token streaming 
    fn process_token_stream(program_id: &Pubkey, accounts: &[AccountInfo], start_time: u64, end_time: u64, amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let withdraw_data = next_account_info(account_info_iter)?; // Program pda to store withdraw data
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let system_program = next_account_info(account_info_iter)?; // system address
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 

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
        if now >= end_time{
            return Err(TokenError::TimeEnd.into());
        }
        if start_time >= end_time {
            return Err(TokenError::InvalidInstruction.into());
        }
        let space_size = std::mem::size_of::<StreamToken>();

        let (account_address, bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIX_TOKEN,
            source_account_info.key,
            token_mint_info.key,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        let withdraw_data_signer_seeds: &[&[_]] = &[
            PREFIX_TOKEN.as_bytes(),
            &source_account_info.key.to_bytes(),
            &token_mint_info.key.to_bytes(),
            &[bump_seed],
        ];
        if withdraw_data.data_is_empty(){
            let transfer_amount =  rent.minimum_balance(std::mem::size_of::<TokenWithdraw>());
            create_pda_account_signed(
                source_account_info,
                transfer_amount,
                std::mem::size_of::<TokenWithdraw>(),
                program_id,
                system_program,
                withdraw_data,
                withdraw_data_signer_seeds
            )?;
        }
        let mut withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount += amount;
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;

        if !pda_data.data_is_empty(){
            return Err(TokenError::StreamAlreadyCreated.into());
        }
        let transfer_amount =  rent.minimum_balance(std::mem::size_of::<StreamToken>());

        create_pda_account( 
            source_account_info,
            transfer_amount,
            space_size,
            program_id,
            system_program,
            pda_data
        )?;
        let mut escrow = StreamToken::try_from_slice(&pda_data.data.borrow())?;
        escrow.start_time = start_time;
        escrow.end_time = end_time;
        escrow.paused = 0;
        escrow.withdraw_limit = 0;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = amount;
        escrow.token_mint = *token_mint_info.key;
        escrow.withdrawn = 0;
        escrow.paused_at = 0;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        msg!("{}",pda_data.data_len());
        Ok(())
    }
    // Function to withdraw from  token streaming 
    fn process_token_withdraw_stream_deprecated(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // master pda
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let withdraw_data = next_account_info(account_info_iter)?; // Program pda to store withdraw data
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?;
        let fee_account = next_account_info(account_info_iter)?;
        let associated_fee_account = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id {
            return Err(ProgramError::InvalidArgument);
        }
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if token_program_info.key != &spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }    
        // Since we are performing system_instruction source account must be signer
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = TokenEscrow::try_from_slice(&pda_data.data.borrow())?;
        assert_keys_equal(escrow.token_mint, *token_mint_info.key)?;
        let now = Clock::get()?.unix_timestamp as u64;
        if now <= escrow.start_time {
            msg!("Stream has not been started");
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            msg!("Stream has been successfully completed");
            allowed_amt = escrow.amount;
        }
        // let rent = &Rent::from_account_info(dest_account_info)?;
        msg!("{} allowed_amt",allowed_amt);
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

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address,&escrow.token_mint);
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        if receiver_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    dest_account_info.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    receiver_associated_info.clone(),
                    dest_account_info.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let fee_account_associated_token = spl_associated_token_account::get_associated_token_address(&fee_account.key,&escrow.token_mint);
        assert_keys_equal(*associated_fee_account.key, fee_account_associated_token)?;
        if associated_fee_account.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    fee_account.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    associated_fee_account.clone(),
                    fee_account.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let comission: u64 = 25*amount/10000; 
        let receiver_amount:u64=amount-comission;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                associated_fee_account.key,
                pda.key,
                &[pda.key],
                comission
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                associated_fee_account.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                receiver_associated_info.key,
                pda.key,
                &[pda.key],
                receiver_amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                receiver_associated_info.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit -= amount;
        }
        msg!("{:?}",escrow);

        escrow.amount  -= amount;
        // Closing account to send rent to sender 
        if escrow.amount == 0 { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIX_TOKEN,
            source_account_info.key,
            &escrow.token_mint,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let mut withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount -= amount;
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
    // Function to withdraw from  token streaming 
    fn process_token_withdraw_stream(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // master pda
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let withdraw_data = next_account_info(account_info_iter)?; // Program pda to store withdraw data
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?;
        let fee_account = next_account_info(account_info_iter)?;
        let associated_fee_account = next_account_info(account_info_iter)?;

       
        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id {
            return Err(ProgramError::InvalidArgument);
        }
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if token_program_info.key != &spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }    
        // Since we are performing system_instruction source account must be signer
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = StreamToken::try_from_slice(&pda_data.data.borrow())?;
        assert_keys_equal(escrow.token_mint, *token_mint_info.key)?;
        let now = Clock::get()?.unix_timestamp as u64;
        msg!("current time: {:?}",now);
        if now <= escrow.start_time {
            msg!("Stream has not been started");
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            allowed_amt = escrow.amount;
        }
        allowed_amt -=  escrow.withdrawn;
        // let rent = &Rent::from_account_info(dest_account_info)?;
        msg!("{} allowed_amt",allowed_amt);
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

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address,&escrow.token_mint);
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        if receiver_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    dest_account_info.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    receiver_associated_info.clone(),
                    dest_account_info.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let fee_account_associated_token = spl_associated_token_account::get_associated_token_address(&fee_account.key,&escrow.token_mint);
        assert_keys_equal(*associated_fee_account.key, fee_account_associated_token)?;
        if associated_fee_account.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    fee_account.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    associated_fee_account.clone(),
                    fee_account.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let comission: u64 = 25*amount/10000; 
        let receiver_amount:u64=amount-comission;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                associated_fee_account.key,
                pda.key,
                &[pda.key],
                comission
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                associated_fee_account.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                receiver_associated_info.key,
                pda.key,
                &[pda.key],
                receiver_amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                receiver_associated_info.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit -= amount;
        }
        msg!("{:?}",escrow);
        escrow.withdrawn += amount;
        // Closing account to send rent to sender
        if escrow.withdrawn == escrow.amount { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIX_TOKEN,
            source_account_info.key,
            &escrow.token_mint,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let mut withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
        msg!("{:?}",withdraw_state);
        withdraw_state.amount = withdraw_state.amount.checked_sub(amount).unwrap();
        msg!("{:?}",withdraw_state);
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to cancel token streaming
    fn process_token_cancel_stream(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // master pda
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let withdraw_data = next_account_info(account_info_iter)?; // Program pda to store withdraw data
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let pda_associated_info = next_account_info(account_info_iter)?; // pda associated token info 
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?; // system program id
        let fee_account = next_account_info(account_info_iter)?;
        let associated_fee_account = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id {
            return Err(ProgramError::InvalidArgument);
        }
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = StreamToken::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;

        // Amount that recipient should receive.  
        let mut allowed_amt = escrow.allowed_amt(now);

        if now < escrow.start_time {
            allowed_amt = 0;
        }
        if now >= escrow.end_time {
            msg!("Stream already completed");
            return Err(TokenError::TimeEnd.into());
        }
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        assert_keys_equal(*token_mint_info.key, escrow.token_mint)?;

        let receiver_associated_account_check = get_associated_token_address(dest_account_info.key,&escrow.token_mint);

        assert_keys_equal(receiver_associated_account_check, *receiver_associated_info.key)?;

        // Sending pending streaming payment to sender 
        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];

        let pda_associated_token = get_associated_token_address(&account_address,&escrow.token_mint);
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;

        if receiver_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    source_account_info.key,
                    dest_account_info.key,
                    token_mint_info.key,
                ),&[
                    source_account_info.clone(),
                    receiver_associated_info.clone(),
                    dest_account_info.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let fee_account_associated_token = spl_associated_token_account::get_associated_token_address(&fee_account.key,&escrow.token_mint);
        assert_keys_equal(*associated_fee_account.key, fee_account_associated_token)?;
        if associated_fee_account.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    fee_account.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    associated_fee_account.clone(),
                    fee_account.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        allowed_amt = allowed_amt.checked_sub(escrow.withdrawn).unwrap();
        msg!("{:?}",allowed_amt);
        let comission: u64 = 25*allowed_amt/10000; 
        let receiver_amount:u64=allowed_amt-comission;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                associated_fee_account.key,
                pda.key,
                &[pda.key],
                comission
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                associated_fee_account.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                receiver_associated_info.key,
                pda.key,
                &[pda.key],
                receiver_amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                receiver_associated_info.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIX_TOKEN,
            source_account_info.key,
            &escrow.token_mint,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        let mut withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount -= escrow.amount.checked_sub(escrow.withdrawn).unwrap();
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        // We don't need to send tokens to sender wallet since tokens are already stored in master pda associated token account
        // Sending pda rent to sender account
        let dest_starting_lamports = source_account_info.lamports();
        **source_account_info.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(pda_data.lamports())
            .ok_or(TokenError::Overflow)?;

        **pda_data.lamports.borrow_mut() = 0;

        escrow.amount = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to pause token streaming
    fn process_pause_token_stream(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;
        if *pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = StreamToken::try_from_slice(&pda_data.data.borrow())?;
        let now = Clock::get()?.unix_timestamp as u64;
        let allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            return Err(TokenError::TimeEnd.into());
        }
        if now < escrow.start_time{
            return Err(TokenError::StreamNotStarted.into());
        }
        if !source_account_info.is_signer && !dest_account_info.is_signer{ // Both sender and receiver can pause / resume stream
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if escrow.start_time >= escrow.end_time {
            return Err(TokenError::InvalidInstruction.into());
        }
        if *source_account_info.key != escrow.sender || *dest_account_info.key != escrow.recipient { //Sender and Recipient both can pause or resume any transaction
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==1{
            return Err(TokenError::AlreadyPaused.into());
        }
        escrow.paused = 1;
        escrow.withdraw_limit = allowed_amt;
        escrow.paused_at = now;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to resume token streaming
    fn process_resume_token_stream(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let now = Clock::get()?.unix_timestamp as u64;
        let mut escrow = StreamToken::try_from_slice(&pda_data.data.borrow())?;
        if !source_account_info.is_signer && !dest_account_info.is_signer{ // Both sender and receiver can pause / resume stream
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *source_account_info.key != escrow.sender || *dest_account_info.key != escrow.recipient { //Sender and Recipient both can pause or resume any transaction
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==0{
            return Err(TokenError::AlreadyResumed.into());
        }
        let time_spent = now - escrow.paused_at;
        escrow.paused = 0;
        escrow.start_time += time_spent;
        escrow.end_time += time_spent;
        escrow.paused_at = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to deposit solana
    fn process_deposit_sol(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?;
        let system_program = next_account_info(account_info_iter)?;

        let (account_address, _bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        assert_keys_equal(account_address, *pda.key)?;
        assert_keys_equal(system_program::id(), *system_program.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        invoke(
            &solana_program::system_instruction::transfer(
                source_account_info.key,
                pda.key,
                amount
            ),
            &[
                source_account_info.clone(),
                pda.clone(),
                system_program.clone()
            ],
        )?;
        Ok(())
    }
    /// Function to deposit token
    fn process_deposit_token(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?; // pda
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let token_mint_info = next_account_info(account_info_iter)?; // token mint
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let associated_token_address = next_account_info(account_info_iter)?; // sender associated token address of token you are initializing 
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let system_program = next_account_info(account_info_iter)?;
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let (account_address, _bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );

        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address,token_mint_info.key);
        assert_keys_equal(spl_token::id(), *token_program_info.key)?;
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    source_account_info.key,
                    pda.key,
                    token_mint_info.key,
                ),&[
                    source_account_info.clone(),
                    pda_associated_info.clone(),
                    pda.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        msg!("1");
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
                system_program.clone()
            ],
        )?;
        msg!("2");
        Ok(())
    }
    /// Function to fund ongoing solana streaming
    fn process_fund_sol(program_id: &Pubkey,accounts: &[AccountInfo],end_time: u64, amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let  source_account_info = next_account_info(account_info_iter)?;  //sender
        let pda_data = next_account_info(account_info_iter)?;  //pda
        let withdraw_data = next_account_info(account_info_iter)?;  //withdraw data

        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        if pda_data.owner != program_id {
            return Err(ProgramError::InvalidArgument);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = Escrow::try_from_slice(&pda_data.data.borrow())?;
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        escrow.end_time = end_time;
        escrow.amount = escrow.amount.checked_add(amount).unwrap();
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let (account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIX,
            source_account_info.key,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount = escrow.amount.checked_add(amount).unwrap();
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to fund ongoing token streaming
    fn process_fund_token(program_id: &Pubkey,accounts: &[AccountInfo],end_time: u64, amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let  source_account_info = next_account_info(account_info_iter)?;  //sender
        let pda_data = next_account_info(account_info_iter)?;  //sender
        let withdraw_data = next_account_info(account_info_iter)?;  //withdraw data

        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = TokenEscrow::try_from_slice(&pda_data.data.borrow())?;
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        escrow.end_time = end_time;
        escrow.amount = amount;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIX_TOKEN,
            source_account_info.key,
            &escrow.token_mint,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        let mut withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount = escrow.amount.checked_add(amount).unwrap();
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to deposit solana
    fn process_withdraw_sol(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?;
        let withdraw_data = next_account_info(account_info_iter)?;  //withdraw data
        let system_program = next_account_info(account_info_iter)?;

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        assert_keys_equal(account_address, *pda.key)?;
        assert_keys_equal(system_program::id(), *system_program.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let (withdraw_account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIX,
            source_account_info.key,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,withdraw_account_address )?;
        if withdraw_data.data_is_empty(){
            invoke_signed(
                &solana_program::system_instruction::transfer(
                    pda.key,
                    source_account_info.key,
                    amount
                ),
                &[
                    pda.clone(),
                    source_account_info.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
        }
        else{
            if *withdraw_data.owner != *program_id {
                return Err(ProgramError::InvalidArgument);
            }
            let allowed_amt = pda.lamports() - amount;
            let withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
            msg!("Your streaming amount is: {}",withdraw_state.amount);
            if allowed_amt < withdraw_state.amount {
                return Err(TokenError::StreamedAmt.into()); 
            }
            invoke_signed(
                &solana_program::system_instruction::transfer(
                    pda.key,
                    source_account_info.key,
                    amount
                ),
                &[
                    pda.clone(),
                    source_account_info.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
            if withdraw_state.amount == 0 { 
                let dest_starting_lamports = source_account_info.lamports();
                **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                    .checked_add(withdraw_data.lamports())
                    .ok_or(TokenError::Overflow)?;
                **withdraw_data.lamports.borrow_mut() = 0;
            }
            withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        }
        Ok(())
    }
    /// Function to deposit token
    fn process_withdraw_token(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let token_mint_info = next_account_info(account_info_iter)?; // token mint
        let associated_token_address = next_account_info(account_info_iter)?; // sender associated token address
        let pda = next_account_info(account_info_iter)?; // pda
        let withdraw_data = next_account_info(account_info_iter)?;  //withdraw data
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let system_program = next_account_info(account_info_iter)?; // system program 

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address,token_mint_info.key);
        let source_associated_token = spl_associated_token_account::get_associated_token_address(source_account_info.key,token_mint_info.key);
        assert_keys_equal(source_associated_token, *associated_token_address.key)?;
        assert_keys_equal(spl_token::id(), *token_program_info.key)?;
        assert_keys_equal(account_address, *pda.key)?;
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIX_TOKEN,
            source_account_info.key,
            token_mint_info.key,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        if withdraw_data.data_is_empty(){
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_info.key,
                    pda_associated_info.key,
                    associated_token_address.key,
                    pda.key,
                    &[pda.key],
                    amount
                )?,
                &[
                    token_program_info.clone(),
                    pda_associated_info.clone(),
                    associated_token_address.clone(),
                    pda.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
        }
        else{
            if *withdraw_data.owner != *program_id {
                return Err(ProgramError::InvalidArgument);
            }
            let token_balance = get_token_balance(pda_associated_info);
            let allowed_amt = token_balance.unwrap() - amount;
            let withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
            if allowed_amt < withdraw_state.amount {
                return Err(TokenError::StreamedAmt.into()); 
            }
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_info.key,
                    pda_associated_info.key,
                    associated_token_address.key,
                    pda.key,
                    &[pda.key],
                    amount
                )?,
                &[
                    token_program_info.clone(),
                    pda_associated_info.clone(),
                    associated_token_address.clone(),
                    pda.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
            if withdraw_state.amount == 0 { 
                let dest_starting_lamports = source_account_info.lamports();
                **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                    .checked_add(withdraw_data.lamports())
                    .ok_or(TokenError::Overflow)?;
                **withdraw_data.lamports.borrow_mut() = 0;
            }
            withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        }
        Ok(())
    }
    fn process_create_multisig(program_id: &Pubkey,accounts: &[AccountInfo],signers: Multisig) -> ProgramResult{
        let account_info= & mut accounts.iter();
        let source_account_info = next_account_info(account_info)?;
        let pda_data = next_account_info(account_info)?;
        let system_program = next_account_info(account_info)?;
        let withdraw_data = next_account_info(account_info)?; // pda data storage
        
        let rent = Rent::get()?; 
        let transfer_amount = rent.minimum_balance(std::mem::size_of::<Multisig>()+355+3*std::mem::size_of::<u64>());
        let (multisig_safe, _bump_seed_multisig) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            pda_data.key,
            program_id,
        );
        let (account_address, bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &multisig_safe,
            program_id,
        );
        let withdraw_data_signer_seeds: &[&[_]] = &[
            PREFIXMULTISIG.as_bytes(),
            &multisig_safe.to_bytes(),
            &[bump_seed],
        ];
        assert_keys_equal(*withdraw_data.key,account_address )?;
        create_pda_account_signed(
            source_account_info,
            rent.minimum_balance(std::mem::size_of::<Withdraw>()),
            std::mem::size_of::<Withdraw>(),
            program_id,
            system_program,
            withdraw_data,
            withdraw_data_signer_seeds
        )?;
        create_pda_account(
            source_account_info,
            transfer_amount,
            std::mem::size_of::<Multisig>()+355,
            program_id,
            system_program,
            pda_data,
        )?;
        msg!("Creating Escrow for multisig");
        msg!("Escrow Created - {}",multisig_safe);
        let mut save_owners = Multisig::from_account(pda_data)?;
        save_owners.signers = signers.signers;
        save_owners.m = signers.m;
        save_owners.multisig_safe = multisig_safe;
        save_owners.serialize(&mut *pda_data.data.borrow_mut())?;
        Ok(())
    }
    /// Function to initialize a solana
    pub fn process_sol_stream_multisig(program_id: &Pubkey, accounts: &[AccountInfo], data: EscrowMultisig) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda_data_multisig = next_account_info(account_info_iter)?; // pda multisig data storage
        let pda_data = next_account_info(account_info_iter)?; // pda data storage
        let system_program = next_account_info(account_info_iter)?; // system program

        // Get the rent sysvar via syscall
        let rent = Rent::get()?; //
        // Since we are performing system_instruction source account must be signer.
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        // current time in unix time
        let now = Clock::get()?.unix_timestamp as u64; 
        if now >= data.end_time{
            return Err(TokenError::TimeEnd.into());
        }
        if data.start_time >= data.end_time {
            return Err(TokenError::InvalidInstruction.into());
        }
        assert_keys_equal(system_program::id(), *system_program.key)?;
        if !pda_data.data_is_empty(){
            return Err(TokenError::StreamAlreadyCreated.into());
        }
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let transfer_amount =  rent.minimum_balance(std::mem::size_of::<StreamMultisig>()+355);
        create_pda_account( 
            source_account_info,
            transfer_amount,
            std::mem::size_of::<StreamMultisig>()+355,
            program_id,
            system_program,
            pda_data
        )?;
        let mut escrow = StreamMultisig::from_account(pda_data)?;
        escrow.start_time = data.start_time;
        escrow.end_time = data.end_time;
        escrow.paused = 1;
        escrow.withdraw_limit = 0;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = data.amount;
        escrow.signed_by = data.signed_by;
        escrow.multisig_safe = multisig_check.multisig_safe;
        escrow.can_cancel = data.can_cancel;
        escrow.paused_at = 0;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        multisig_check.serialize(&mut *pda_data_multisig.data.borrow_mut())?;
        msg!("{}",pda_data.data_len());
        Ok(())
    }
    fn process_sign_stream(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult{
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let pda_data = next_account_info(account_info_iter)?; // pda data storage
        let pda_data_multisig = next_account_info(account_info_iter)?; // pda multisig data storage
        let withdraw_data = next_account_info(account_info_iter)?; // pda data storage
        let system_program = next_account_info(account_info_iter)?; 

        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *pda_data.owner != *program_id && *pda_data_multisig.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut n = 0; 
        let mut escrow = StreamMultisig::from_account(pda_data)?;
        let now = Clock::get()?.unix_timestamp as u64;
        if now > escrow.start_time {
            return Err(TokenError::TimeEnd.into());
        }
        let signed_by = WhiteList {
            address: *source_account_info.key,
            counter:0
        };
        for i in 0..escrow.signed_by.len(){
            if escrow.signed_by[i].address == signed_by.address {
                n += 1;
            }
        }
        if n > 0{
            return Err(TokenError::PublicKeyMismatch.into()); 
        }
        msg!("{:?}",signed_by);
        escrow.signed_by.push(signed_by);
        let rent = Rent::get()?; 
        let (account_address, bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &multisig_check.multisig_safe,
            program_id,
        );
        let withdraw_data_signer_seeds: &[&[_]] = &[
            PREFIXMULTISIG.as_bytes(),
            &multisig_check.multisig_safe.to_bytes(),
            &[bump_seed],
        ];
        assert_keys_equal(*withdraw_data.key,account_address )?;
        
        if  escrow.signed_by.len() >= multisig_check.m.into() {
            escrow.paused = 0;
        }
        if withdraw_data.data_is_empty(){
            create_pda_account_signed(
                source_account_info,
                rent.minimum_balance(std::mem::size_of::<TokenWithdraw>()),
                std::mem::size_of::<TokenWithdraw>(),
                program_id,
                system_program,
                withdraw_data,
                withdraw_data_signer_seeds
            )?;
        }
        else{
            if *withdraw_data.owner != *program_id {
                return Err(ProgramError::InvalidArgument);
            }
            let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
            withdraw_state.amount += escrow.amount;
            withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        }
        msg!("{:?}",escrow);
        multisig_check.serialize(&mut *pda_data_multisig.data.borrow_mut())?;
        escrow.serialize(&mut *pda_data.data.borrow_mut())?;
        Ok(())
    }
    fn process_reject_sol_stream_multisig(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // sender
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let pda_data_multisig = next_account_info(account_info_iter)?; // pda multisig data storage

        let escrow = StreamMultisig::from_account(pda_data)?;
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        let now = Clock::get()?.unix_timestamp as u64;
        if now > escrow.start_time {
            return Err(TokenError::TimeEnd.into());
        }
        if multisig_check.multisig_safe !=escrow.multisig_safe{
            return Err(TokenError::PublicKeyMismatch.into());
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        multisig_check.serialize(&mut &mut pda_data_multisig.data.borrow_mut()[..])?;
        let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        Ok(())
    }
    fn process_swap_sol(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?; // master pda 
        let multi_sig_pda = next_account_info(account_info_iter)?; // multisig vault
        let multi_sig_pda_data = next_account_info(account_info_iter)?;
        let withdraw_data = next_account_info(account_info_iter)?;  //withdraw data
        let system_program = next_account_info(account_info_iter)?;

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let (account_address_multisig, _bump_seed_multisig) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multi_sig_pda_data.key,
            program_id,
        );
        assert_keys_equal(account_address_multisig, *multi_sig_pda.key)?;
        assert_keys_equal(account_address, *pda.key)?;
        assert_keys_equal(system_program::id(), *system_program.key)?;
        let (account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIX,
            source_account_info.key,
            program_id,
        );
        assert_keys_equal(account_address, *withdraw_data.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if withdraw_data.data_is_empty(){
            invoke_signed(
                &solana_program::system_instruction::transfer(
                    pda.key,
                    multi_sig_pda.key,
                    amount
                ),
                &[
                    pda.clone(),
                    multi_sig_pda.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
        }
        else{
            let allowed_amt = pda.lamports() - amount;
            let withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
            msg!("Your streaming amount is: {}",withdraw_state.amount);
            if allowed_amt < withdraw_state.amount {
                return Err(TokenError::StreamedAmt.into()); 
            }
            invoke_signed(
                &solana_program::system_instruction::transfer(
                    pda.key,
                    multi_sig_pda.key,
                    amount
                ),
                &[
                    pda.clone(),
                    multi_sig_pda.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
            if withdraw_state.amount == 0 { 
                let dest_starting_lamports = source_account_info.lamports();
                **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                    .checked_add(withdraw_data.lamports())
                    .ok_or(TokenError::Overflow)?;
                **withdraw_data.lamports.borrow_mut() = 0;
            }
            withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        }
        Ok(())
    }
    fn process_swap_token(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let multi_sig_pda = next_account_info(account_info_iter)?;
        let multi_sig_pda_data = next_account_info(account_info_iter)?;
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let token_mint_info = next_account_info(account_info_iter)?; // token mint
        let associated_token_address = next_account_info(account_info_iter)?; // sender associated token address
        let pda = next_account_info(account_info_iter)?; // pda
        let withdraw_data = next_account_info(account_info_iter)?;  //withdraw data
        let multisig_pda_associated_info = next_account_info(account_info_iter)?; // Associated token of multisig pda
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of multisig pda
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?; // system program 

        let (account_address, bump_seed) = get_master_address_and_bump_seed(
            source_account_info.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &source_account_info.key.to_bytes(),
            &[bump_seed],
        ];
        let (account_address_multisig, _bump_seed_multisig) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multi_sig_pda_data.key,
            program_id,
        );
        assert_keys_equal(account_address_multisig, *multi_sig_pda.key)?;
        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address_multisig,token_mint_info.key);
        let source_associated_token = spl_associated_token_account::get_associated_token_address(source_account_info.key,token_mint_info.key);
        assert_keys_equal(source_associated_token, *associated_token_address.key)?;
        assert_keys_equal(spl_token::id(), *token_program_info.key)?;
        assert_keys_equal(account_address, *pda.key)?;
        assert_keys_equal(pda_associated_token, *multisig_pda_associated_info.key)?;
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIX_TOKEN,
            source_account_info.key,
            token_mint_info.key,
            program_id,
        );
        assert_keys_equal(account_address, *withdraw_data.key)?;
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if multisig_pda_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    source_account_info.key,
                    multi_sig_pda.key,
                    token_mint_info.key,
                ),&[
                    source_account_info.clone(),
                    multisig_pda_associated_info.clone(),
                    multi_sig_pda.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        if withdraw_data.data_is_empty(){
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_info.key,
                    pda_associated_info.key,
                    multisig_pda_associated_info.key,
                    pda.key,
                    &[pda.key],
                    amount
                )?,
                &[
                    token_program_info.clone(),
                    pda_associated_info.clone(),
                    multisig_pda_associated_info.clone(),
                    pda.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
        }
        else{
            let allowed_amt = pda.lamports() - amount;
            let withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
            msg!("Your streaming amount is: {}",withdraw_state.amount);
            if allowed_amt < withdraw_state.amount {
                return Err(TokenError::StreamedAmt.into()); 
            }
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_info.key,
                    pda_associated_info.key,
                    multisig_pda_associated_info.key,
                    pda.key,
                    &[pda.key],
                    amount
                )?,
                &[
                    token_program_info.clone(),
                    pda_associated_info.clone(),
                    multisig_pda_associated_info.clone(),
                    pda.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
            if withdraw_state.amount == 0 { 
                let dest_starting_lamports = source_account_info.lamports();
                **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                    .checked_add(withdraw_data.lamports())
                    .ok_or(TokenError::Overflow)?;
                **withdraw_data.lamports.borrow_mut() = 0;
            }
            withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        }
        Ok(())
    }
    /// Function to deposit solana
    fn process_transfer_sol_multisig(program_id: &Pubkey,accounts: &[AccountInfo],data: SolTransfer,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // sender
        let dest_account_info = next_account_info(account_info_iter)?; // reciver
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda data
        let pda_data = next_account_info(account_info_iter)?; // new sol.keypair
        let system_program = next_account_info(account_info_iter)?; 

        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let (account_address_multisig, _bump_seed_multisig) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multisig_pda_data.key,
            program_id,
        );
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        assert_keys_equal(account_address_multisig, multisig_check.multisig_safe)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let rent = Rent::get()?; //
        let transfer_amount =  rent.minimum_balance(std::mem::size_of::<SolTransfer>()+355);
        create_pda_account( 
            source_account_info,
            transfer_amount,
            std::mem::size_of::<SolTransfer>()+355,
            program_id,
            system_program,
            pda_data
        )?;
        let mut escrow = SolTransfer::from_account(pda_data)?;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = data.amount;
        escrow.signed_by = data.signed_by;
        escrow.multisig_safe = multisig_check.multisig_safe;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        multisig_check.serialize(&mut &mut multisig_pda_data.data.borrow_mut()[..])?;

        Ok(())
    }
    fn process_transfer_sol_sign_multisig(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult{
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let dest_account_info = next_account_info(account_info_iter)?;  //receiver
        let multisig_vault = next_account_info(account_info_iter)?;  //multig vault 
        let pda_data_multisig = next_account_info(account_info_iter)?;  //multisig pda 
        let pda_data = next_account_info(account_info_iter)?; // pda data storage transfer sol multisig
        let system_program = next_account_info(account_info_iter)?; 

        if *pda_data.owner != *program_id && *pda_data_multisig.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = SolTransfer::from_account(pda_data)?;
        if escrow.multisig_safe != multisig_check.multisig_safe{
            return Err(ProgramError::MissingRequiredSignature); 
        }  
        let mut n = 0; 
        let signed_by = WhiteList {
            address: *source_account_info.key,
            counter:0
        };
        for i in 0..escrow.signed_by.len(){
            if escrow.signed_by[i].address == signed_by.address {
                n += 1;
            }
        }
        if n > 0{
            return Err(TokenError::PublicKeyMismatch.into()); 
        }
        escrow.signed_by.push(signed_by);
        if  escrow.signed_by.len() >= multisig_check.m.into() {
            let (account_address_multisig, bump_seed_multisig) = get_multisig_data_and_bump_seed(
                PREFIXMULTISIGSAFE,
                pda_data_multisig.key,
                program_id,
            );
            let pda_signer_seeds: &[&[_]] = &[
                &PREFIXMULTISIGSAFE.as_bytes(),
                &pda_data_multisig.key.to_bytes(),
                &[bump_seed_multisig],
            ];
            assert_keys_equal(*multisig_vault.key,account_address_multisig)?;
            create_transfer(
                multisig_vault,
                dest_account_info,
                system_program,
                escrow.amount,
                pda_signer_seeds
            )?;
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        msg!("{:?}",escrow);
        escrow.serialize(&mut *pda_data.data.borrow_mut())?;
        multisig_check.serialize(&mut &mut pda_data_multisig.data.borrow_mut()[..])?;
        Ok(())
    }
    fn process_transfer_sol_reject_multisig(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult{
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let pda_data_multisig = next_account_info(account_info_iter)?;  //multisig pda 
        let pda_data = next_account_info(account_info_iter)?; // pda data storage transfer sol multisig

        if *pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let escrow = SolTransfer::from_account(pda_data)?;   
        if escrow.multisig_safe != multisig_check.multisig_safe{
            return Err(ProgramError::MissingRequiredSignature); 
        }  
        escrow.serialize(&mut *pda_data.data.borrow_mut())?;
        let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        multisig_check.serialize(&mut &mut pda_data_multisig.data.borrow_mut()[..])?;
        Ok(())
    }
    fn process_sol_withdraw_stream_multisig_deprecated(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // stream initiator address
        let dest_account_info = next_account_info(account_info_iter)?; // stream receiver
        let pda = next_account_info(account_info_iter)?; // multisig vault pda
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda
        let withdraw_data = next_account_info(account_info_iter)?; // withdraw data 
        let system_program = next_account_info(account_info_iter)?; // system program id 
        let fee_account = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id  && *multisig_pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        let (account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &multisig_check.multisig_safe,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        multisig_check.serialize(&mut &mut multisig_pda_data.data.borrow_mut()[..])?;
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = EscrowMultisig::from_account(pda_data)?;
        if escrow.multisig_safe != *pda.key{
            return Err(TokenError::EscrowMismatch.into());
        }
        let now = Clock::get()?.unix_timestamp as u64;
        if now <= escrow.start_time {
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            allowed_amt = escrow.amount;
        }
        msg!("You can withdraw {}",allowed_amt);
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        // Checking if amount is greater than allowed amount
        if amount>allowed_amt {
            return Err(ProgramError::InsufficientFunds);
        }
        // Checking if paused stream is greater than withdraw limit
        if escrow.paused == 1 && amount > escrow.withdraw_limit {
            return Err(ProgramError::InsufficientFunds);
        }
        let (_account_address_multisig, bump_seed_multisig) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multisig_pda_data.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &PREFIXMULTISIGSAFE.as_bytes(),
            &multisig_pda_data.key.to_bytes(),
            &[bump_seed_multisig],
        ];
        let comission: u64 = 25*amount/10000; 
        let receiver_amount:u64=amount-comission;
        create_transfer(
            pda,
            fee_account,
            system_program,
            comission,
            pda_signer_seeds
        )?;
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            receiver_amount,
            pda_signer_seeds
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit -= amount;
        }
        escrow.amount -=amount;
        // Closing account to send rent to sender
        if escrow.amount == 0 { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount -= amount;
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
    fn process_sol_withdraw_stream_multisig(program_id: &Pubkey,accounts: &[AccountInfo],amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // stream initiator address
        let dest_account_info = next_account_info(account_info_iter)?; // stream receiver
        let pda = next_account_info(account_info_iter)?; // multisig vault pda
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda
        let withdraw_data = next_account_info(account_info_iter)?; // withdraw data 
        let system_program = next_account_info(account_info_iter)?; // system program id 
        let fee_account = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id  && *multisig_pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        let (account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &multisig_check.multisig_safe,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        multisig_check.serialize(&mut &mut multisig_pda_data.data.borrow_mut()[..])?;
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = StreamMultisig::from_account(pda_data)?;
        if escrow.multisig_safe != *pda.key{
            return Err(TokenError::EscrowMismatch.into());
        }
        let now = Clock::get()?.unix_timestamp as u64;
        if now <= escrow.start_time {
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            allowed_amt = escrow.amount;
        }
        allowed_amt -=  escrow.withdrawn;
        msg!("You can withdraw {}",allowed_amt);
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if *dest_account_info.key != escrow.recipient {
            return Err(TokenError::EscrowMismatch.into());
        }
        // Checking if amount is greater than allowed amount
        if amount>allowed_amt {
            return Err(ProgramError::InsufficientFunds);
        }
        // Checking if paused stream is greater than withdraw limit
        if escrow.paused == 1 && amount > escrow.withdraw_limit {
            return Err(ProgramError::InsufficientFunds);
        }
        let (_account_address_multisig, bump_seed_multisig) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multisig_pda_data.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &PREFIXMULTISIGSAFE.as_bytes(),
            &multisig_pda_data.key.to_bytes(),
            &[bump_seed_multisig],
        ];
        let comission: u64 = 25*amount/10000; 
        let receiver_amount:u64=amount-comission;
        create_transfer(
            pda,
            fee_account,
            system_program,
            comission,
            pda_signer_seeds
        )?;
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            receiver_amount,
            pda_signer_seeds
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit -= amount;
        }
        escrow.withdrawn  = escrow.withdrawn.checked_add(amount).unwrap();
        // Closing account to send rent to sender
        if escrow.withdrawn == escrow.amount { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount = withdraw_state.amount.checked_sub(amount).unwrap();
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
     /// Function to cancel solana streaming
     fn process_cancel_sol_stream_multisig(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda = next_account_info(account_info_iter)?; // locked fund
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda
        let withdraw_data = next_account_info(account_info_iter)?; // withdraw data 
        let system_program = next_account_info(account_info_iter)?; // system program id 
        let fee_account = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id  && *multisig_pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = StreamMultisig::from_account(pda_data)?;
        if escrow.can_cancel == false {
            return Err(TokenError::CancelNotAllowed.into());
        }
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        let (account_address, _bump_seed) = get_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &multisig_check.multisig_safe,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let now = Clock::get()?.unix_timestamp as u64;
        msg!("Current time: {}", now);
        // Amount that recipient should receive.  
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            msg!("Stream already completed");
            return Err(TokenError::StreamNotStarted.into());
        }
        if now < escrow.start_time {
            allowed_amt = 0;
        }
        if *source_account_info.key != escrow.sender {
            return Err(TokenError::OwnerMismatch.into());
        }
        msg!("escrow.multisig_safe: {} *pda.key: {}",escrow.multisig_safe,*pda.key);

        if escrow.multisig_safe != *pda.key {
            return Err(TokenError::OwnerMismatch.into());
        }
        let (_account_address_multisig, bump_seed_multisig) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multisig_pda_data.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &PREFIXMULTISIGSAFE.as_bytes(),
            &multisig_pda_data.key.to_bytes(),
            &[bump_seed_multisig],
        ];
        allowed_amt = allowed_amt.checked_sub(escrow.withdrawn).unwrap();
        let comission: u64 = 25*allowed_amt/10000; 
        let receiver_amount:u64=allowed_amt-comission;
        create_transfer(
            pda,
            fee_account,
            system_program,
            comission,
            pda_signer_seeds
        )?;
        // Sending streamed payment to receiver 
        create_transfer(
            pda,
            dest_account_info,
            system_program,
            receiver_amount,
            pda_signer_seeds
        )?;
        let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount -= escrow.amount.checked_sub(escrow.withdrawn).unwrap();
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        // We don't need to send remaining funds to sender, its already in sender master pda account which he can withdraw with withdraw function
        // Closing account to send rent to sender
        let dest_starting_lamports = source_account_info.lamports();
        **source_account_info.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(pda_data.lamports())
            .ok_or(TokenError::Overflow)?;

        **pda_data.lamports.borrow_mut() = 0;
        escrow.amount = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    //Function to pause solana stream
    fn process_pause_sol_stream_multisig(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda
        
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = StreamMultisig::from_account(pda_data)?;
        if multisig_check.multisig_safe != escrow.multisig_safe{
            return Err(TokenError::OwnerMismatch.into());
        }
        let now = Clock::get()?.unix_timestamp as u64;
        let allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            return Err(TokenError::TimeEnd.into());
        }
        if now < escrow.start_time{
            return Err(TokenError::StreamNotStarted.into());
        }
        // Both sender and receiver can pause / resume stream
        if !source_account_info.is_signer && !dest_account_info.is_signer{ 
            return Err(ProgramError::MissingRequiredSignature); 
        }

        if escrow.paused ==1{
            return Err(TokenError::AlreadyPaused.into());
        }
        escrow.paused = 1;
        escrow.withdraw_limit = allowed_amt;
        escrow.paused_at = now;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    //Function to resume solana stream
    fn process_resume_sol_stream_multisig(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda

        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let now = Clock::get()?.unix_timestamp as u64;
        let mut escrow = StreamMultisig::from_account(pda_data)?;
        // Both sender and receiver can pause / resume stream
        if !source_account_info.is_signer && !dest_account_info.is_signer{ 
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if multisig_check.multisig_safe != escrow.multisig_safe{
            return Err(TokenError::OwnerMismatch.into());
        }
        if escrow.paused ==0{
            return Err(TokenError::AlreadyResumed.into());
        }
        let time_spent = now - escrow.paused_at;
        escrow.paused = 0;
        escrow.start_time += time_spent;
        escrow.end_time += time_spent;
        escrow.paused_at = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    // Function to initialize token streaming 
    fn process_token_multisig_stream(program_id: &Pubkey, accounts: &[AccountInfo],data: TokenEscrowMultisig) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let pda_data_multisig = next_account_info(account_info_iter)?; // Program pda to store data
        let token_program_info = next_account_info(account_info_iter)?; // TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA
        let system_program = next_account_info(account_info_iter)?; // system address
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
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
        if now >= data.end_time{
            return Err(TokenError::TimeEnd.into());
        }
        if data.start_time >= data.end_time {
            return Err(TokenError::InvalidInstruction.into());
        }
        if !pda_data.data_is_empty(){
            return Err(TokenError::StreamAlreadyCreated.into());
        }
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let transfer_amount =  rent.minimum_balance(std::mem::size_of::<TokenStreamMultisig>());
        create_pda_account( 
            source_account_info,
            transfer_amount+transfer_amount+transfer_amount+transfer_amount,
            std::mem::size_of::<TokenStreamMultisig>()+355,
            program_id,
            system_program,
            pda_data
        )?;

        let mut escrow = TokenStreamMultisig::from_account(pda_data)?;
        escrow.start_time = data.start_time;
        escrow.end_time = data.end_time;
        escrow.paused = 1;
        escrow.withdraw_limit = 0;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = data.amount;
        escrow.token_mint = *token_mint_info.key;
        escrow.signed_by = data.signed_by;
        escrow.multisig_safe = multisig_check.multisig_safe;
        escrow.can_cancel = data.can_cancel;
        escrow.withdrawn = 0;
        escrow.paused_at = 0;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        multisig_check.serialize(&mut &mut pda_data_multisig.data.borrow_mut()[..])?;
        Ok(())
    }
    fn process_sign_token_stream(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult{
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let pda_data = next_account_info(account_info_iter)?; // pda data storage
        let pda_data_multisig = next_account_info(account_info_iter)?; // pda multisig data storage
        let withdraw_data = next_account_info(account_info_iter)?; // Program pda to store withdraw data
        let system_program = next_account_info(account_info_iter)?; 

        if *pda_data.owner != *program_id && *pda_data_multisig.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key{
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut n = 0; 
        let mut escrow = TokenStreamMultisig::from_account(pda_data)?;
        let now = Clock::get()?.unix_timestamp as u64;
        if now > escrow.start_time {
            return Err(TokenError::TimeEnd.into());
        }
        let signed_by = WhiteList {
            address: *source_account_info.key,
            counter:0
        };
        for i in 0..escrow.signed_by.len(){
            if escrow.signed_by[i].address == signed_by.address {
                n += 1;
            }
        }
        if n > 0{
            return Err(TokenError::PublicKeyMismatch.into()); 
        }
        escrow.signed_by.push(signed_by);
        if escrow.paused == 0{
            return Err(TokenError::TimeEnd.into()); 
        }
        let (account_address, bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &multisig_check.multisig_safe,
            &escrow.token_mint,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        let withdraw_data_signer_seeds: &[&[_]] = &[
            PREFIXMULTISIG.as_bytes(),
            &multisig_check.multisig_safe.to_bytes(),
            &escrow.token_mint.to_bytes(),
            &[bump_seed],
        ];
        let rent = Rent::get()?; 
       
        if  escrow.signed_by.len() >= multisig_check.m.into() {
            escrow.paused = 0;
        }
        if withdraw_data.data_is_empty(){
            create_pda_account_signed(
                source_account_info,
                rent.minimum_balance(std::mem::size_of::<TokenWithdraw>()),
                std::mem::size_of::<TokenWithdraw>(),
                program_id,
                system_program,
                withdraw_data,
                withdraw_data_signer_seeds
            )?;
        }
        else{
            if *withdraw_data.owner != *program_id {
                return Err(ProgramError::InvalidArgument);
            }
            let mut withdraw_state = Withdraw::try_from_slice(&withdraw_data.data.borrow())?;
            withdraw_state.amount += escrow.amount;
            withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        }
        msg!("{:?}",escrow);
        multisig_check.serialize(&mut *pda_data_multisig.data.borrow_mut())?;
        escrow.serialize(&mut *pda_data.data.borrow_mut())?;
        Ok(())
    }
    fn process_reject_token_stream_multisig(accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // stream initiator address
        let initiator_account_info = next_account_info(account_info_iter)?; // stream initiator address
        let pda_data = next_account_info(account_info_iter)?; // stored data 
        let pda_data_multisig = next_account_info(account_info_iter)?; // pda multisig data storage

        let escrow = TokenStreamMultisig::from_account(pda_data)?;
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        msg!("multisig: {} escrow:{}",multisig_check.multisig_safe,escrow.multisig_safe);
        let now = Clock::get()?.unix_timestamp as u64;
        if now > escrow.start_time {
            return Err(TokenError::TimeEnd.into());
        }
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if multisig_check.multisig_safe !=escrow.multisig_safe{
            return Err(TokenError::PublicKeyMismatch.into());
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        multisig_check.serialize(&mut &mut pda_data_multisig.data.borrow_mut()[..])?;
        let dest_starting_lamports = initiator_account_info.lamports();
            **initiator_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        Ok(())
    }
    fn process_token_withdraw_multisig_stream_deprecated(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // multisig vault
        let multisig_pda_data = next_account_info(account_info_iter)?; // Multisig pda stored data
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let withdraw_data = next_account_info(account_info_iter)?; // Program pda to store withdraw data
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?;
        let fee_account = next_account_info(account_info_iter)?;
        let associated_fee_account = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id  && *multisig_pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if token_program_info.key != &spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }    
        // Since we are performing system_instruction source account must be signer
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = TokenEscrowMultisig::from_account(pda_data)?;
        assert_keys_equal(escrow.token_mint, *token_mint_info.key)?;
        let now = Clock::get()?.unix_timestamp as u64;
        if now <= escrow.start_time {
            msg!("Stream has not been started");
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            msg!("Stream has been successfully completed");
            allowed_amt = escrow.amount;
        }
        // let rent = &Rent::from_account_info(dest_account_info)?;
        msg!("{} allowed_amt",allowed_amt);
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

        let (account_address_multisig, bump_seed) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multisig_pda_data.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &PREFIXMULTISIGSAFE.as_bytes(),
            &multisig_pda_data.key.to_bytes(),
            &[bump_seed],
        ];

        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address_multisig,&escrow.token_mint);
        msg!("pda_associated_token frontend:{}",pda_associated_info.key);
        msg!("pda_associated_token frontend:{}",pda_associated_token);
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &escrow.multisig_safe,
            &escrow.token_mint,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        if receiver_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    dest_account_info.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    receiver_associated_info.clone(),
                    dest_account_info.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let fee_account_associated_token = spl_associated_token_account::get_associated_token_address(&fee_account.key,&escrow.token_mint);
        assert_keys_equal(*associated_fee_account.key, fee_account_associated_token)?;
        if associated_fee_account.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    fee_account.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    associated_fee_account.clone(),
                    fee_account.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let comission: u64 = 25*amount/10000; 
        let receiver_amount:u64=amount-comission;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                associated_fee_account.key,
                pda.key,
                &[pda.key],
                comission
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                associated_fee_account.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                receiver_associated_info.key,
                pda.key,
                &[pda.key],
                receiver_amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                receiver_associated_info.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit -= amount;
        }
        escrow.amount  -= amount;
        // Closing account to send rent to sender 
        if escrow.amount == 0 { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let mut withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount -= amount;
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
     // Function to withdraw from  token streaming 
    fn process_token_withdraw_multisig_stream(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // multisig vault
        let multisig_pda_data = next_account_info(account_info_iter)?; // Multisig pda stored data
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let withdraw_data = next_account_info(account_info_iter)?; // Program pda to store withdraw data
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of pda
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?;
        let fee_account = next_account_info(account_info_iter)?;
        let associated_fee_account = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id  && *multisig_pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        if token_program_info.key != &spl_token::id() {
            return Err(ProgramError::IncorrectProgramId);
        }    
        // Since we are performing system_instruction source account must be signer
        if !dest_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = TokenStreamMultisig::from_account(pda_data)?;
        assert_keys_equal(escrow.token_mint, *token_mint_info.key)?;
        let now = Clock::get()?.unix_timestamp as u64;
        msg!("current time: {}",now);
        if now <= escrow.start_time {
            msg!("Stream has not been started");
            return Err(TokenError::StreamNotStarted.into());
        }
        // Recipient can only withdraw the money that is already streamed. 
        let mut allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            allowed_amt = escrow.amount;
        }
        allowed_amt -=  escrow.withdrawn;
        // let rent = &Rent::from_account_info(dest_account_info)?;
        msg!("{} allowed_amt",allowed_amt);
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

        let (account_address_multisig, bump_seed) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multisig_pda_data.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &PREFIXMULTISIGSAFE.as_bytes(),
            &multisig_pda_data.key.to_bytes(),
            &[bump_seed],
        ];

        let pda_associated_token = spl_associated_token_account::get_associated_token_address(&account_address_multisig,&escrow.token_mint);
        msg!("pda_associated_token frontend:{}",pda_associated_info.key);
        msg!("pda_associated_token frontend:{}",pda_associated_token);
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &escrow.multisig_safe,
            &escrow.token_mint,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        if receiver_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    dest_account_info.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    receiver_associated_info.clone(),
                    dest_account_info.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let fee_account_associated_token = spl_associated_token_account::get_associated_token_address(&fee_account.key,&escrow.token_mint);
        assert_keys_equal(*associated_fee_account.key, fee_account_associated_token)?;
        if associated_fee_account.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    fee_account.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    associated_fee_account.clone(),
                    fee_account.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let comission: u64 = 25*amount/10000; 
        let receiver_amount:u64=amount-comission;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                associated_fee_account.key,
                pda.key,
                &[pda.key],
                comission
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                associated_fee_account.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                receiver_associated_info.key,
                pda.key,
                &[pda.key],
                receiver_amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                receiver_associated_info.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        if escrow.paused == 1{
            msg!("{}{}",escrow.withdraw_limit,amount);
            escrow.withdraw_limit -= amount;
        }
        msg!("amount: {}",amount);
        escrow.withdrawn = escrow.withdrawn.checked_add(amount).unwrap();
        msg!("amount: {}",escrow.withdrawn);

        // Closing account to send rent to sender
        if escrow.withdrawn == escrow.amount { 
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        let mut withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount = withdraw_state.amount.checked_sub(amount).unwrap();
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to cancel token streaming
    fn process_token_cancel_multisig_stream(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  // sender 
        let dest_account_info = next_account_info(account_info_iter)?; // recipient
        let pda = next_account_info(account_info_iter)?; // multisig vault
        let pda_data = next_account_info(account_info_iter)?; // Program pda to store data
        let multisig_pda_data = next_account_info(account_info_iter)?;
        let withdraw_data = next_account_info(account_info_iter)?; // Program pda to store withdraw data
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let pda_associated_info = next_account_info(account_info_iter)?; // pda associated token info 
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?; // system program id
        let fee_account = next_account_info(account_info_iter)?;
        let associated_fee_account = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *withdraw_data.owner != *program_id  && *multisig_pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let fee_receiver= &Pubkey::from_str("EsDV3m3xUZ7g8QKa1kFdbZT18nNz8ddGJRcTK84WDQ7k").unwrap();
        if fee_account.key != fee_receiver {
            return Err(TokenError::OwnerMismatch.into());
        }
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = TokenStreamMultisig::from_account(pda_data)?;
        let now = Clock::get()?.unix_timestamp as u64;
        msg!("current time: {}",now);

        if escrow.can_cancel == false {
            return Err(TokenError::CancelNotAllowed.into());
        }
        if escrow.multisig_safe != *pda.key {
            return Err(TokenError::OwnerMismatch.into());
        }
        // Amount that recipient should receive.  
        let mut allowed_amt = escrow.allowed_amt(now);
        msg!("allowed_amt: {}",allowed_amt);
        if now < escrow.start_time {
            allowed_amt = 0;
        }
        if now >= escrow.end_time {
            msg!("Stream already completed");
            return Err(TokenError::TimeEnd.into());
        }
        assert_keys_equal(*token_mint_info.key, escrow.token_mint)?;

        let receiver_associated_account_check = get_associated_token_address(dest_account_info.key,&escrow.token_mint);

        assert_keys_equal(receiver_associated_account_check, *receiver_associated_info.key)?;

        // Sending pending streaming payment to sender 
        let (_account_address, bump_seed) = get_multisig_data_and_bump_seed(
            &PREFIXMULTISIGSAFE,
            multisig_pda_data.key,
            program_id,
        );
        let pda_signer_seeds: &[&[_]] = &[
            &PREFIXMULTISIGSAFE.as_bytes(),
            &multisig_pda_data.key.to_bytes(),
            &[bump_seed],
        ];

        let pda_associated_token = get_associated_token_address(&multisig_check.multisig_safe,&escrow.token_mint);
        assert_keys_equal(pda_associated_token, *pda_associated_info.key)?;

        if receiver_associated_info.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    source_account_info.key,
                    dest_account_info.key,
                    token_mint_info.key,
                ),&[
                    source_account_info.clone(),
                    receiver_associated_info.clone(),
                    dest_account_info.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        let fee_account_associated_token = spl_associated_token_account::get_associated_token_address(&fee_account.key,&escrow.token_mint);
        assert_keys_equal(*associated_fee_account.key, fee_account_associated_token)?;
        if associated_fee_account.data_is_empty(){
            invoke(            
                &spl_associated_token_account::create_associated_token_account(
                    dest_account_info.key,
                    fee_account.key,
                    token_mint_info.key,
                ),&[
                    dest_account_info.clone(),
                    associated_fee_account.clone(),
                    fee_account.clone(),
                    token_mint_info.clone(),
                    token_program_info.clone(),
                    rent_info.clone(),
                    associated_token_info.clone(),
                    system_program.clone()
                ]
            )?
        }
        allowed_amt = allowed_amt.checked_sub(escrow.withdrawn).unwrap();
        msg!("withdrawn: {}",escrow.withdrawn);
        msg!("subtracting withdrawn: {}",allowed_amt);
        let comission: u64 = 25*allowed_amt/10000; 
        msg!("comission: {}",comission);
        let receiver_amount:u64=allowed_amt-comission;
        msg!("receiver_amount: {}",receiver_amount);
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                associated_fee_account.key,
                pda.key,
                &[pda.key],
                comission
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                associated_fee_account.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        invoke_signed(
            &spl_token::instruction::transfer(
                token_program_info.key,
                pda_associated_info.key,
                receiver_associated_info.key,
                pda.key,
                &[pda.key],
                receiver_amount
            )?,
            &[
                token_program_info.clone(),
                pda_associated_info.clone(),
                receiver_associated_info.clone(),
                pda.clone(),
                system_program.clone()
            ],&[&pda_signer_seeds],
        )?;
        let (account_address, _bump_seed) = get_token_withdraw_data_and_bump_seed(
            PREFIXMULTISIG,
            &escrow.multisig_safe,
            &escrow.token_mint,
            program_id,
        );
        assert_keys_equal(*withdraw_data.key,account_address )?;
        let mut withdraw_state = TokenWithdraw::try_from_slice(&withdraw_data.data.borrow())?;
        withdraw_state.amount -= escrow.amount.checked_sub(escrow.withdrawn).unwrap();
        withdraw_state.serialize(&mut &mut withdraw_data.data.borrow_mut()[..])?;
        // We don't need to send tkens to sender wallet since tokens are already stored in master pda associated token account
        // Sending pda rent to sender account
        let dest_starting_lamports = source_account_info.lamports();
        **source_account_info.lamports.borrow_mut() = dest_starting_lamports
            .checked_add(pda_data.lamports())
            .ok_or(TokenError::Overflow)?;

        **pda_data.lamports.borrow_mut() = 0;

        escrow.amount = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to pause token streaming
    fn process_pause_token_multisig_stream(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda
        if *pda_data.owner != *program_id && *multisig_pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let mut escrow = TokenStreamMultisig::from_account(pda_data)?;
        if multisig_check.multisig_safe != escrow.multisig_safe{
            return Err(TokenError::OwnerMismatch.into());
        }
        let now = Clock::get()?.unix_timestamp as u64;
        let allowed_amt = escrow.allowed_amt(now);
        if now >= escrow.end_time {
            return Err(TokenError::TimeEnd.into());
        }
        if now < escrow.start_time{
            return Err(TokenError::StreamNotStarted.into());
        }
        if !source_account_info.is_signer && !dest_account_info.is_signer{ // Both sender and receiver can pause / resume stream
            return Err(ProgramError::MissingRequiredSignature); 
        }

        if k == multisig_check.signers.len() || *dest_account_info.key != escrow.recipient { //Sender and Recipient both can pause or resume any transaction
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==1{
            return Err(TokenError::AlreadyPaused.into());
        }
        escrow.paused = 1;
        escrow.withdraw_limit = allowed_amt;
        escrow.paused_at = now;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to resume token streaming
    fn process_resume_token_multisig_stream(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;
        let dest_account_info = next_account_info(account_info_iter)?;
        let pda_data = next_account_info(account_info_iter)?;
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda

        if *pda_data.owner != *program_id && *multisig_pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if pda_data.data_is_empty(){
            return Err(ProgramError::UninitializedAccount);
        }
        let now = Clock::get()?.unix_timestamp as u64;
        let mut escrow = TokenStreamMultisig::from_account(pda_data)?;
        if multisig_check.multisig_safe != escrow.multisig_safe{
            return Err(TokenError::OwnerMismatch.into());
        }
        if !source_account_info.is_signer && !dest_account_info.is_signer{ // Both sender and receiver can pause / resume stream
            return Err(ProgramError::MissingRequiredSignature); 
        }
        if k == multisig_check.signers.len() || *dest_account_info.key != escrow.recipient { //Sender and Recipient both can pause or resume any transaction
            return Err(TokenError::EscrowMismatch.into());
        }
        if escrow.paused ==0{
            return Err(TokenError::AlreadyResumed.into());
        }
        let time_spent = now - escrow.paused_at;
        escrow.paused = 0;
        escrow.start_time += time_spent;
        escrow.end_time += time_spent;
        escrow.paused_at = 0;
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    /// Function to deposit solana
    fn process_transfer_token_multisig(program_id: &Pubkey,accounts: &[AccountInfo],data: TokenTransfer,) -> ProgramResult {
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?; // sender
        let dest_account_info = next_account_info(account_info_iter)?; // reciver
        let multisig_pda_data = next_account_info(account_info_iter)?; // multisig pda data
        let pda_data = next_account_info(account_info_iter)?; // new sol.keypair
        let token_mint = next_account_info(account_info_iter)?; // token mint address
        let system_program = next_account_info(account_info_iter)?; 
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let (_account_address_multisig, _bump_seed_multisig) = get_multisig_data_and_bump_seed(
            PREFIXMULTISIGSAFE,
            multisig_pda_data.key,
            program_id,
        );
        let multisig_check = Multisig::from_account(multisig_pda_data)?;
        assert_keys_equal(_account_address_multisig, multisig_check.multisig_safe)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let rent = Rent::get()?; //
        let transfer_amount =  rent.minimum_balance(std::mem::size_of::<TokenTransfer>()+355);
        create_pda_account( 
            source_account_info,
            transfer_amount,
            std::mem::size_of::<TokenTransfer>()+355,
            program_id,
            system_program,
            pda_data
        )?;
        let mut escrow = TokenTransfer::from_account(pda_data)?;
        escrow.sender = *source_account_info.key;
        escrow.recipient = *dest_account_info.key;
        escrow.amount = data.amount;
        escrow.signed_by = data.signed_by;
        escrow.multisig_safe = multisig_check.multisig_safe;
        escrow.token_mint = *token_mint.key;
        msg!("{:?}",escrow);
        escrow.serialize(&mut &mut pda_data.data.borrow_mut()[..])?;
        Ok(())
    }
    fn process_transfer_token_sign_multisig(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult{
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let dest_account_info = next_account_info(account_info_iter)?;  //receiver
        let multisig_vault = next_account_info(account_info_iter)?;  //multig vault 
        let pda_data_multisig = next_account_info(account_info_iter)?;  //multisig pda 
        let pda_data = next_account_info(account_info_iter)?; // pda data storage transfer sol multisig
        let token_program_info = next_account_info(account_info_iter)?; // {TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA}
        let token_mint_info = next_account_info(account_info_iter)?; // token you would like to initilaize 
        let rent_info = next_account_info(account_info_iter)?; // rent address
        let pda_associated_info = next_account_info(account_info_iter)?; // Associated token of multisig vault
        let receiver_associated_info = next_account_info(account_info_iter)?; // Associated token of receiver
        let associated_token_info = next_account_info(account_info_iter)?; // Associated token master {ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL}
        let system_program = next_account_info(account_info_iter)?;

        if *pda_data.owner != *program_id && *pda_data_multisig.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let mut escrow = TokenTransfer::from_account(pda_data)?;
        let mut n = 0; 
        let signed_by = WhiteList {
            address: *source_account_info.key,
            counter:0
        };
        for i in 0..escrow.signed_by.len(){
            if escrow.signed_by[i].address == signed_by.address {
                n += 1;
            }
        }
        if n > 0{
            return Err(TokenError::PublicKeyMismatch.into()); 
        }
        if escrow.multisig_safe != multisig_check.multisig_safe{
            return Err(ProgramError::MissingRequiredSignature); 
        }  
        escrow.signed_by.push(signed_by);
        if  escrow.signed_by.len() >= multisig_check.m.into() {
            if *dest_account_info.key != escrow.recipient {
                return Err(TokenError::EscrowMismatch.into());
            }
            assert_keys_equal(escrow.token_mint, *token_mint_info.key)?;
            let (account_address_multisig, bump_seed_multisig) = get_multisig_data_and_bump_seed(
                PREFIXMULTISIGSAFE,
                pda_data_multisig.key,
                program_id,
            );
            let pda_signer_seeds: &[&[_]] = &[
                &PREFIXMULTISIGSAFE.as_bytes(),
                &pda_data_multisig.key.to_bytes(),
                &[bump_seed_multisig],
            ];
            assert_keys_equal(*multisig_vault.key,account_address_multisig)?;
            if receiver_associated_info.data_is_empty(){
                invoke(            
                    &spl_associated_token_account::create_associated_token_account(
                        source_account_info.key,
                        dest_account_info.key,
                        token_mint_info.key,
                    ),&[
                        source_account_info.clone(),
                        receiver_associated_info.clone(),
                        dest_account_info.clone(),
                        token_mint_info.clone(),
                        token_program_info.clone(),
                        rent_info.clone(),
                        associated_token_info.clone(),
                        system_program.clone()
                    ]
                )?
            }
            invoke_signed(
                &spl_token::instruction::transfer(
                    token_program_info.key,
                    pda_associated_info.key,
                    receiver_associated_info.key,
                    multisig_vault.key,
                    &[multisig_vault.key],
                    escrow.amount
                )?,
                &[
                    token_program_info.clone(),
                    pda_associated_info.clone(),
                    receiver_associated_info.clone(),
                    multisig_vault.clone(),
                    system_program.clone()
                ],&[&pda_signer_seeds],
            )?;
            let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        }
        msg!("{:?}",escrow);
        escrow.serialize(&mut *pda_data.data.borrow_mut())?;
        Ok(())
    }
    fn process_transfer_token_reject_multisig(program_id: &Pubkey,accounts: &[AccountInfo]) -> ProgramResult{
        let account_info_iter = &mut accounts.iter();
        let source_account_info = next_account_info(account_info_iter)?;  //sender
        let pda_data_multisig = next_account_info(account_info_iter)?;  //multisig pda 
        let pda_data = next_account_info(account_info_iter)?; // pda data storage transfer sol multisig

        if *pda_data.owner != *program_id{
            return Err(ProgramError::InvalidArgument);
        }
        if !source_account_info.is_signer {
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let multisig_check = Multisig::from_account(pda_data_multisig)?;
        let mut k = 0; 
        for i in 0..multisig_check.signers.len(){
            if multisig_check.signers[i].address != *source_account_info.key {
                k += 1;
            }
        }
        if k == multisig_check.signers.len(){
            return Err(ProgramError::MissingRequiredSignature); 
        }
        let escrow = TokenTransfer::from_account(pda_data)?;
        if escrow.multisig_safe != multisig_check.multisig_safe{
            return Err(ProgramError::MissingRequiredSignature); 
        }  
        escrow.serialize(&mut *pda_data.data.borrow_mut())?;
        let dest_starting_lamports = source_account_info.lamports();
            **source_account_info.lamports.borrow_mut() = dest_starting_lamports
                .checked_add(pda_data.lamports())
                .ok_or(TokenError::Overflow)?;
            **pda_data.lamports.borrow_mut() = 0;
        multisig_check.serialize(&mut &mut pda_data_multisig.data.borrow_mut()[..])?;
        Ok(())
    }
    
    /// Processes an [Instruction](enum.Instruction.html).
    pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
        let instruction = TokenInstruction::unpack(input)?;
        match instruction {
            TokenInstruction::ProcessSolStream (ProcessSolStream{
                start_time,
                end_time,
                amount,
            }) => {
                msg!("Instruction: Sol Stream");
                Self::process_sol_stream(program_id,accounts,start_time, end_time, amount)
            }
            TokenInstruction::ProcessSolWithdrawStream(ProcessSolWithdrawStream {
                amount,
            }) => {
                msg!("Instruction: Sol Withdraw");
                let pda_data = &accounts[3];// Program pda to store data
                if pda_data.data_len() != 120 {
                    Self::process_sol_withdraw_stream_deprecated(program_id,accounts,amount)
                }
                else{
                    Self::process_sol_withdraw_stream(program_id,accounts, amount)
                }
            }
            TokenInstruction::ProcessCancelSolStream => {
                msg!("Instruction: Sol Cancel");
                Self::process_cancel_sol_stream(program_id,accounts)
            }
            TokenInstruction::ProcessTokenStream(ProcessTokenStream {
                start_time,
                end_time,
                amount,
            }) => {
                msg!("Instruction: Token Stream");
                Self::process_token_stream(program_id,accounts,start_time, end_time, amount)
            }
            TokenInstruction::ProcessPauseSolStream => {
                msg!("Instruction: Stream pause");
                Self::process_pause_sol_stream(program_id,accounts)
            }
            TokenInstruction::ProcessResumeSolStream=> {
                msg!("Instruction: Stream Resume ");
                Self::process_resume_sol_stream(program_id,accounts)
            }
            TokenInstruction::ProcessTokenWithdrawStream(ProcessTokenWithdrawStream {
                amount,
            }) => {
                msg!("Instruction: Token Withdraw");
                let pda_data = &accounts[3];// Program pda to store data
                msg!("{}",pda_data.data_len());
                if pda_data.data_len() != 152 {
                    Self::process_token_withdraw_stream_deprecated(program_id,accounts,amount)
                }
                else{
                    Self::process_token_withdraw_stream(program_id,accounts, amount)
                }
            }
            TokenInstruction::ProcessDepositSol(ProcessDepositSol {
                amount,
            }) => {
                msg!("Instruction: Deposit Sol");
                Self::process_deposit_sol(program_id,accounts, amount)
            }
            TokenInstruction::ProcessCancelTokenStream => {
                msg!("Instruction: Cancel Token Stream");
                Self::process_token_cancel_stream(program_id,accounts)
            }
            TokenInstruction::ProcessPauseTokenStream => {
                msg!("Instruction: Token Stream Pause");
                Self::process_pause_token_stream(program_id,accounts)
            }
            TokenInstruction::ProcessResumeTokenStream => {
                msg!("Instruction:  Token Stream Resume");
                Self::process_resume_token_stream(program_id,accounts)
            }
            TokenInstruction::ProcessDepositToken(ProcessDepositToken {
                amount,
            }) => {
                msg!("Instruction: Deposit token");
                Self::process_deposit_token(program_id,accounts,amount) 
            }
            TokenInstruction::ProcessFundSol(ProcessFundSol {
                end_time,
                amount,
            }) => {
                msg!("Instruction: Fund Solana");
                Self::process_fund_sol(program_id,accounts,end_time,amount) 
            }
            TokenInstruction::ProcessFundToken(ProcessFundToken {
                end_time,
                amount,
            }) => {
                msg!("Instruction: Fund token");
                Self::process_fund_token(program_id,accounts,end_time,amount) 
            }
            TokenInstruction::ProcessWithdrawSol(ProcessWithdrawSol {
                amount,
            }) => {
                msg!("Instruction: Withdraw Sol");
                Self::process_withdraw_sol(program_id,accounts,amount) 
            }
            TokenInstruction::ProcessWithdrawToken(ProcessWithdrawToken {
                amount,
            }) => {
                msg!("Instruction: Withdraw token");
                Self::process_withdraw_token(program_id,accounts,amount) 
            }
            TokenInstruction::CreateWhitelist{whitelist_v1} => {
                msg!("Instruction: Creating MultiSig");
                Self::process_create_multisig(program_id,accounts,whitelist_v1) 
            }
            TokenInstruction::ProcessSwapSol(ProcessSwapSol{
                amount
            }) =>{
                msg!("Instruction: Swapping Solana");
                Self::process_swap_sol(program_id,accounts,amount) 
            }
            TokenInstruction::ProcessSwapToken(ProcessSwapToken{
                amount
            }) =>{
                msg!("Instruction: Swapping token");
                Self::process_swap_token(program_id,accounts,amount) 
            }
            TokenInstruction::SignedBy => {
                msg!("Instruction: Signing multisig");
                Self::process_sign_stream(program_id,accounts) 
            }
            TokenInstruction::ProcessSolMultiSigStream{whitelist_v3} => {
                msg!("Instruction: Streaming MultiSig");
                Self::process_sol_stream_multisig(program_id,accounts,whitelist_v3) 
            }
            TokenInstruction::ProcessSolWithdrawStreamMultisig (ProcessSolWithdrawStreamMultisig{
                amount}) =>{
                    let pda_data = &accounts[3];// Program pda to store data
                    msg!("pda_data: {}",pda_data.data_len());
                    if pda_data.data_len() != 539 {
                        Self::process_sol_withdraw_stream_multisig_deprecated(program_id,accounts,amount)
                    }
                    else {
                        Self::process_sol_withdraw_stream_multisig(program_id,accounts,amount) 
                    }
                }
            TokenInstruction::ProcessSolCancelStreamMultisig => {
                msg!("Instruction: Multisig Sol Cancel");
                Self::process_cancel_sol_stream_multisig(program_id,accounts)
            }
            TokenInstruction::ProcessPauseMultisigStream => {
                msg!("Instruction: Stream pause");
                Self::process_pause_sol_stream_multisig(accounts)
            }
            TokenInstruction::ProcessResumeMultisigStream=> {
                msg!("Instruction: Stream Resume ");
                Self::process_resume_sol_stream_multisig(accounts)
            }
            TokenInstruction::ProcessRejectMultisigStream=> {
                msg!("Instruction: Rejecting stream ");
                Self::process_reject_sol_stream_multisig(accounts)
            }
            TokenInstruction::ProcessSolTokenMultiSigStream{whitelist_v4}=>{
                msg!("Instruction: Streaming Token MultiSig");
                Self::process_token_multisig_stream(program_id,accounts,whitelist_v4) 
            }
            TokenInstruction::ProcessTokenWithdrawStreamMultisig (ProcessTokenWithdrawStreamMultisig{
                amount}) =>{
                    msg!("Instruction: Withdraw Token MultiSig");
                    let pda_data = &accounts[4];// Program pda to store data
                    msg!("pda_data: {}",pda_data.data_len());
                    if pda_data.data_len() != 571 {
                        Self::process_token_withdraw_multisig_stream_deprecated(program_id,accounts,amount)
                    }
                    else {
                        Self::process_token_withdraw_multisig_stream(program_id,accounts,amount) 
                    }
                }
            TokenInstruction::ProcessTokenCancelStreamMultisig => {
                msg!("Instruction: Multisig Token Cancel");
                Self::process_token_cancel_multisig_stream(program_id,accounts)
            }
            TokenInstruction::ProcessPauseTokenMultisigStream => {
                msg!("Instruction: Stream pause");
                Self::process_pause_token_multisig_stream(program_id,accounts)
            }
            TokenInstruction::ProcessResumeTokenMultisigStream=> {
                msg!("Instruction: Stream Resume ");
                Self::process_resume_token_multisig_stream(program_id,accounts)
            }
            TokenInstruction::ProcessRejectTokenMultisigStream=> {
                msg!("Instruction: Rejecting token stream ");
                Self::process_reject_token_stream_multisig(accounts)
            }
            TokenInstruction::SignedByToken => {
                msg!("Instruction: Signing token multisig");
                Self::process_sign_token_stream(program_id,accounts) 
            }
            TokenInstruction::ProcessSolTransfer{whitelist_v3} => {
                msg!("Instruction: Initiating Sol transfer multisig");
                Self::process_transfer_sol_multisig(program_id,accounts,whitelist_v3) 
            }
            TokenInstruction::SignedByTransferSol => {
                msg!("Instruction: Signing Sol transfer multisig");
                Self::process_transfer_sol_sign_multisig(program_id,accounts) 
            }
            TokenInstruction::ProcessTokenTransfer{whitelist_v3} => {
                msg!("Instruction: Initiating token transfer multisig");
                Self::process_transfer_token_multisig(program_id,accounts,whitelist_v3) 
            }
            TokenInstruction::SignedByTransferToken => {
                msg!("Instruction: Signing token transfer multisig");
                Self::process_transfer_token_sign_multisig(program_id,accounts) 
            }
            TokenInstruction::ProcessRejectTransferSol => {
                msg!("Instruction: Rejecting token transfer multisig");
                Self::process_transfer_sol_reject_multisig(program_id,accounts) 
            }
            TokenInstruction::ProcessRejectTransferToken => {
                msg!("Instruction: Rejecting token transfer multisig");
                Self::process_transfer_token_reject_multisig(program_id,accounts) 
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
            TokenError::TimeEnd => msg!("Error: Time has already passed"),
            TokenError::OwnerMismatch => msg!("Error: Owner does not match"),
            TokenError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            TokenError::EscrowMismatch => msg!("Error: Account not associated with this Escrow"),
            TokenError::InvalidInstruction => msg!("Error: Invalid instruction"),
            TokenError::AlreadyCancel => msg!("Error: Invalid instruction"),
            TokenError::AlreadyWithdrawn => msg!("Error: Paused stream, streamed amount already withdrawn"),
            TokenError::Overflow => msg!("Error: Operation overflowed"),
            TokenError::PublicKeyMismatch => msg!("Error: Public key mismatched"),
            TokenError::AlreadyPaused=> msg!("Error: Transaction is already paused"),
            TokenError::AlreadyResumed=>msg!("Error: Transaction is not paused"),
            TokenError::StreamAlreadyCreated=>msg!("Stream Already Created"),
            TokenError::StreamNotStarted=>msg!("Stream has not been started"),
            TokenError::StreamedAmt=>msg!("Cannot withdraw streaming amount"),
            TokenError::CancelNotAllowed=>msg!("cannot cancel this transaction")
        }
    }
}