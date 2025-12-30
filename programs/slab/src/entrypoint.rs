//! Slab program entrypoint

use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    msg,
    pubkey::Pubkey,
    ProgramResult,
};

use crate::instructions::{
    SlabInstruction, 
    process_reserve, 
    process_commit, 
    process_cancel, 
    process_batch_open, 
    process_initialize_slab,
    process_add_instrument,
    process_update_funding,
    process_liquidation,
};
use crate::state::SlabState;
use percolator_common::{PercolatorError, validate_owner, validate_writable, borrow_account_data_mut, InstructionReader};

entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    // Check minimum instruction data length
    if instruction_data.is_empty() {
        msg!("Error: Instruction data is empty");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Parse instruction discriminator
    let discriminator = instruction_data[0];
    let instruction = match discriminator {
        0 => SlabInstruction::Reserve,
        1 => SlabInstruction::Commit,
        2 => SlabInstruction::Cancel,
        3 => SlabInstruction::BatchOpen,
        4 => SlabInstruction::Initialize,
        5 => SlabInstruction::AddInstrument,
        6 => SlabInstruction::UpdateFunding,
        7 => SlabInstruction::Liquidation,
        _ => {
            msg!("Error: Unknown instruction");
            return Err(PercolatorError::InvalidInstruction.into());
        }
    };

    // Dispatch to instruction handler
    match instruction {
        SlabInstruction::Reserve => {
            msg!("Instruction: Reserve");
            process_reserve_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Commit => {
            msg!("Instruction: Commit");
            process_commit_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Cancel => {
            msg!("Instruction: Cancel");
            process_cancel_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::BatchOpen => {
            msg!("Instruction: BatchOpen");
            process_batch_open_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Initialize => {
            msg!("Instruction: Initialize");
            process_initialize_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::AddInstrument => {
            msg!("Instruction: AddInstrument");
            process_add_instrument_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::UpdateFunding => {
            msg!("Instruction: UpdateFunding");
            process_update_funding_inner(program_id, accounts, &instruction_data[1..])
        }
        SlabInstruction::Liquidation => {
            msg!("Instruction: Liquidation");
            process_liquidation_inner(program_id, accounts, &instruction_data[1..])
        }
    }
}

// Instruction processors with account validation

/// Process reserve instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account
/// 2. `[]` Router program (for CPI validation)
///
/// Expected data layout (78 bytes):
/// - account_idx: u32 (4 bytes)
/// - instrument_idx: u16 (2 bytes)
/// - side: u8 (1 byte)
/// - qty: u64 (8 bytes)
/// - limit_px: u64 (8 bytes)
/// - ttl_ms: u64 (8 bytes)
/// - commitment_hash: [u8; 32] (32 bytes)
/// - route_id: u64 (8 bytes)
fn process_reserve_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    // Validate account count
    if accounts.len() < 1 {
        msg!("Error: Reserve instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    // Account 0: Slab state (must be writable and owned by this program)
    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    // Deserialize slab state
    // SAFETY: We've validated ownership and the account should contain SlabState
    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let account_idx = reader.read_u32()?;
    let instrument_idx = reader.read_u16()?;
    let side = reader.read_side()?;
    let qty = reader.read_u64()?;
    let limit_px = reader.read_u64()?;
    let ttl_ms = reader.read_u64()?;
    let commitment_hash = reader.read_bytes::<32>()?;
    let route_id = reader.read_u64()?;

    // Call the instruction handler
    let _result = process_reserve(
        slab,
        account_idx,
        instrument_idx,
        side,
        qty,
        limit_px,
        ttl_ms,
        commitment_hash,
        route_id,
    )?;

    msg!("Reserve processed successfully");
    Ok(())
}

/// Process commit instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account
///
/// Expected data layout (16 bytes):
/// - hold_id: u64 (8 bytes)
/// - current_ts: u64 (8 bytes)
fn process_commit_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Commit instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let hold_id = reader.read_u64()?;
    let current_ts = reader.read_u64()?;

    // Call the instruction handler
    let _result = process_commit(slab, hold_id, current_ts)?;

    msg!("Commit processed successfully");
    Ok(())
}

/// Process cancel instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` User account
///
/// Expected data layout (8 bytes):
/// - hold_id: u64 (8 bytes)
fn process_cancel_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Cancel instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let hold_id = reader.read_u64()?;

    // Call the instruction handler
    process_cancel(slab, hold_id)?;

    msg!("Cancel processed successfully");
    Ok(())
}

/// Process batch open instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Authority account (for permissioned batch opening)
///
/// Expected data layout (10 bytes):
/// - instrument_idx: u16 (2 bytes)
/// - current_ts: u64 (8 bytes)
fn process_batch_open_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: BatchOpen instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let instrument_idx = reader.read_u16()?;
    let current_ts = reader.read_u64()?;

    // Call the instruction handler
    process_batch_open(slab, instrument_idx, current_ts)?;

    msg!("BatchOpen processed successfully");
    Ok(())
}

/// Process initialize instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account (PDA, uninitialized)
/// 1. `[signer]` Payer/authority
///
/// Expected data layout (168 bytes):
/// - market_id: [u8; 32] (32 bytes)
/// - lp_owner: Pubkey (32 bytes)
/// - router_id: Pubkey (32 bytes)
/// - imr: u64 (8 bytes)
/// - mmr: u64 (8 bytes)
/// - maker_fee: i64 (8 bytes)
/// - taker_fee: u64 (8 bytes)
/// - batch_ms: u64 (8 bytes)
fn process_initialize_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Initialize instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let market_id = reader.read_bytes::<32>()?;
    let lp_owner_bytes = reader.read_bytes::<32>()?;
    let router_id_bytes = reader.read_bytes::<32>()?;
    let imr = reader.read_u64()?;
    let mmr = reader.read_u64()?;
    let maker_fee = reader.read_i64()?;
    let taker_fee = reader.read_u64()?;
    let batch_ms = reader.read_u64()?;

    let lp_owner = Pubkey::from(lp_owner_bytes);
    let router_id = Pubkey::from(router_id_bytes);

    // Call the initialization logic
    process_initialize_slab(
        program_id,
        slab_account,
        market_id,
        lp_owner,
        router_id,
        imr,
        mmr,
        maker_fee,
        taker_fee,
        batch_ms,
    )?;

    msg!("Slab initialized successfully");
    Ok(())
}

/// Process add instrument instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Authority (LP owner)
///
/// Expected data layout (32 bytes):
/// - symbol: [u8; 8] (8 bytes)
/// - contract_size: u64 (8 bytes)
/// - tick: u64 (8 bytes)
/// - lot: u64 (8 bytes)
/// - initial_mark: u64 (8 bytes)
fn process_add_instrument_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: AddInstrument instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let symbol = reader.read_bytes::<8>()?;
    let contract_size = reader.read_u64()?;
    let tick = reader.read_u64()?;
    let lot = reader.read_u64()?;
    let initial_mark = reader.read_u64()?;

    // Call the instruction handler
    let idx = process_add_instrument(slab, symbol, contract_size, tick, lot, initial_mark)?;

    msg!("AddInstrument processed successfully");
    Ok(())
}

/// Process update funding instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Oracle/Authority
///
/// Expected data layout (18 bytes):
/// - instrument_idx: u16 (2 bytes)
/// - index_price: u64 (8 bytes)
/// - current_ts: u64 (8 bytes)
fn process_update_funding_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: UpdateFunding instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let instrument_idx = reader.read_u16()?;
    let index_price = reader.read_u64()?;
    let current_ts = reader.read_u64()?;

    // Call the instruction handler
    process_update_funding(slab, instrument_idx, index_price, current_ts)?;

    msg!("UpdateFunding processed successfully");
    Ok(())
}

/// Process liquidation instruction
///
/// Expected accounts:
/// 0. `[writable]` Slab state account
/// 1. `[signer]` Router authority
///
/// Expected data layout (20 bytes):
/// - account_idx: u32 (4 bytes)
/// - deficit_target: i128 (16 bytes) - stored as bytes
/// - current_ts: u64 (8 bytes)
fn process_liquidation_inner(program_id: &Pubkey, accounts: &[AccountInfo], data: &[u8]) -> ProgramResult {
    if accounts.len() < 1 {
        msg!("Error: Liquidation instruction requires at least 1 account");
        return Err(PercolatorError::InvalidInstruction.into());
    }

    let slab_account = &accounts[0];
    validate_owner(slab_account, program_id)?;
    validate_writable(slab_account)?;

    let slab = unsafe { borrow_account_data_mut::<SlabState>(slab_account)? };

    // Parse instruction data
    let mut reader = InstructionReader::new(data);
    let account_idx = reader.read_u32()?;
    
    // Read deficit_target as i128 (16 bytes)
    let deficit_bytes = reader.read_bytes::<16>()?;
    let deficit_target = i128::from_le_bytes(deficit_bytes);
    
    let current_ts = reader.read_u64()?;

    // Call the instruction handler
    let result = process_liquidation(slab, account_idx, deficit_target, current_ts)?;

    msg!("Liquidation processed successfully");
    Ok(())
}
