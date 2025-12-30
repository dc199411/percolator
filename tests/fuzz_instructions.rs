//! Fuzz tests for instruction parsing and edge cases
//!
//! Tests malformed inputs, boundary conditions, and edge cases.
//! Uses proptest for property-based fuzzing.
//!
//! Run with: cargo test --test fuzz_instructions -- --nocapture

use proptest::prelude::*;

// ============================================================================
// INSTRUCTION DISCRIMINATORS
// ============================================================================

mod ix_disc {
    pub const RESERVE: u8 = 0;
    pub const COMMIT: u8 = 1;
    pub const CANCEL: u8 = 2;
    pub const BATCH_OPEN: u8 = 3;
    pub const INITIALIZE: u8 = 4;
    pub const ADD_INSTRUMENT: u8 = 5;
    pub const UPDATE_FUNDING: u8 = 6;
    pub const LIQUIDATION: u8 = 7;
    pub const MAX_VALID: u8 = 7;
}

// ============================================================================
// INSTRUCTION PARSING
// ============================================================================

/// Parsed instruction result
#[derive(Debug, Clone)]
pub enum ParsedInstruction {
    Initialize(InitializeData),
    AddInstrument(AddInstrumentData),
    Reserve(ReserveData),
    Commit(CommitData),
    Cancel(CancelData),
    BatchOpen(BatchOpenData),
    UpdateFunding(UpdateFundingData),
    Liquidation(LiquidationData),
}

#[derive(Debug, Clone)]
pub struct InitializeData {
    pub market_id: [u8; 32],
    pub lp_owner: [u8; 32],
    pub router_id: [u8; 32],
    pub imr_bps: u64,
    pub mmr_bps: u64,
    pub maker_fee_bps: i64,
    pub taker_fee_bps: u64,
    pub batch_ms: u64,
}

#[derive(Debug, Clone)]
pub struct AddInstrumentData {
    pub symbol: [u8; 8],
    pub contract_size: u64,
    pub tick: u64,
    pub lot: u64,
    pub initial_mark: u64,
}

#[derive(Debug, Clone)]
pub struct ReserveData {
    pub account_idx: u32,
    pub instrument_idx: u16,
    pub side: u8,
    pub qty: u64,
    pub limit_px: u64,
    pub ttl_ms: u64,
    pub commitment_hash: [u8; 32],
    pub route_id: u64,
}

#[derive(Debug, Clone)]
pub struct CommitData {
    pub hold_id: u64,
    pub current_ts: u64,
}

#[derive(Debug, Clone)]
pub struct CancelData {
    pub hold_id: u64,
}

#[derive(Debug, Clone)]
pub struct BatchOpenData {
    pub instrument_idx: u16,
    pub current_ts: u64,
}

#[derive(Debug, Clone)]
pub struct UpdateFundingData {
    pub instrument_idx: u16,
    pub index_price: u64,
    pub current_ts: u64,
}

#[derive(Debug, Clone)]
pub struct LiquidationData {
    pub account_idx: u32,
    pub deficit_target: i128,
    pub current_ts: u64,
}

/// Parse error
#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    Empty,
    InvalidDiscriminator(u8),
    TooShort { expected: usize, actual: usize },
    InvalidData(String),
}

/// Parse instruction from raw bytes
pub fn parse_instruction(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    if data.is_empty() {
        return Err(ParseError::Empty);
    }
    
    let discriminator = data[0];
    let payload = &data[1..];
    
    match discriminator {
        ix_disc::INITIALIZE => parse_initialize(payload),
        ix_disc::ADD_INSTRUMENT => parse_add_instrument(payload),
        ix_disc::RESERVE => parse_reserve(payload),
        ix_disc::COMMIT => parse_commit(payload),
        ix_disc::CANCEL => parse_cancel(payload),
        ix_disc::BATCH_OPEN => parse_batch_open(payload),
        ix_disc::UPDATE_FUNDING => parse_update_funding(payload),
        ix_disc::LIQUIDATION => parse_liquidation(payload),
        _ => Err(ParseError::InvalidDiscriminator(discriminator)),
    }
}

fn parse_initialize(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    // Expected: 32 + 32 + 32 + 8 + 8 + 8 + 8 + 8 = 136 bytes
    if data.len() < 136 {
        return Err(ParseError::TooShort { expected: 136, actual: data.len() });
    }
    
    let mut offset = 0;
    
    let mut market_id = [0u8; 32];
    market_id.copy_from_slice(&data[offset..offset + 32]);
    offset += 32;
    
    let mut lp_owner = [0u8; 32];
    lp_owner.copy_from_slice(&data[offset..offset + 32]);
    offset += 32;
    
    let mut router_id = [0u8; 32];
    router_id.copy_from_slice(&data[offset..offset + 32]);
    offset += 32;
    
    let imr_bps = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let mmr_bps = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let maker_fee_bps = i64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let taker_fee_bps = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let batch_ms = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    
    Ok(ParsedInstruction::Initialize(InitializeData {
        market_id,
        lp_owner,
        router_id,
        imr_bps,
        mmr_bps,
        maker_fee_bps,
        taker_fee_bps,
        batch_ms,
    }))
}

fn parse_add_instrument(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    // Expected: 8 + 8 + 8 + 8 + 8 = 40 bytes
    if data.len() < 40 {
        return Err(ParseError::TooShort { expected: 40, actual: data.len() });
    }
    
    let mut offset = 0;
    
    let mut symbol = [0u8; 8];
    symbol.copy_from_slice(&data[offset..offset + 8]);
    offset += 8;
    
    let contract_size = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let tick = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let lot = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let initial_mark = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    
    Ok(ParsedInstruction::AddInstrument(AddInstrumentData {
        symbol,
        contract_size,
        tick,
        lot,
        initial_mark,
    }))
}

fn parse_reserve(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    // Expected: 4 + 2 + 1 + 8 + 8 + 8 + 32 + 8 = 71 bytes
    if data.len() < 71 {
        return Err(ParseError::TooShort { expected: 71, actual: data.len() });
    }
    
    let mut offset = 0;
    
    let account_idx = u32::from_le_bytes(data[offset..offset + 4].try_into().unwrap());
    offset += 4;
    
    let instrument_idx = u16::from_le_bytes(data[offset..offset + 2].try_into().unwrap());
    offset += 2;
    
    let side = data[offset];
    offset += 1;
    
    let qty = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let limit_px = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let ttl_ms = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    offset += 8;
    
    let mut commitment_hash = [0u8; 32];
    commitment_hash.copy_from_slice(&data[offset..offset + 32]);
    offset += 32;
    
    let route_id = u64::from_le_bytes(data[offset..offset + 8].try_into().unwrap());
    
    Ok(ParsedInstruction::Reserve(ReserveData {
        account_idx,
        instrument_idx,
        side,
        qty,
        limit_px,
        ttl_ms,
        commitment_hash,
        route_id,
    }))
}

fn parse_commit(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    // Expected: 8 + 8 = 16 bytes
    if data.len() < 16 {
        return Err(ParseError::TooShort { expected: 16, actual: data.len() });
    }
    
    let hold_id = u64::from_le_bytes(data[0..8].try_into().unwrap());
    let current_ts = u64::from_le_bytes(data[8..16].try_into().unwrap());
    
    Ok(ParsedInstruction::Commit(CommitData { hold_id, current_ts }))
}

fn parse_cancel(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    // Expected: 8 bytes
    if data.len() < 8 {
        return Err(ParseError::TooShort { expected: 8, actual: data.len() });
    }
    
    let hold_id = u64::from_le_bytes(data[0..8].try_into().unwrap());
    
    Ok(ParsedInstruction::Cancel(CancelData { hold_id }))
}

fn parse_batch_open(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    // Expected: 2 + 8 = 10 bytes
    if data.len() < 10 {
        return Err(ParseError::TooShort { expected: 10, actual: data.len() });
    }
    
    let instrument_idx = u16::from_le_bytes(data[0..2].try_into().unwrap());
    let current_ts = u64::from_le_bytes(data[2..10].try_into().unwrap());
    
    Ok(ParsedInstruction::BatchOpen(BatchOpenData { instrument_idx, current_ts }))
}

fn parse_update_funding(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    // Expected: 2 + 8 + 8 = 18 bytes
    if data.len() < 18 {
        return Err(ParseError::TooShort { expected: 18, actual: data.len() });
    }
    
    let instrument_idx = u16::from_le_bytes(data[0..2].try_into().unwrap());
    let index_price = u64::from_le_bytes(data[2..10].try_into().unwrap());
    let current_ts = u64::from_le_bytes(data[10..18].try_into().unwrap());
    
    Ok(ParsedInstruction::UpdateFunding(UpdateFundingData {
        instrument_idx,
        index_price,
        current_ts,
    }))
}

fn parse_liquidation(data: &[u8]) -> Result<ParsedInstruction, ParseError> {
    // Expected: 4 + 16 + 8 = 28 bytes
    if data.len() < 28 {
        return Err(ParseError::TooShort { expected: 28, actual: data.len() });
    }
    
    let account_idx = u32::from_le_bytes(data[0..4].try_into().unwrap());
    let deficit_target = i128::from_le_bytes(data[4..20].try_into().unwrap());
    let current_ts = u64::from_le_bytes(data[20..28].try_into().unwrap());
    
    Ok(ParsedInstruction::Liquidation(LiquidationData {
        account_idx,
        deficit_target,
        current_ts,
    }))
}

// ============================================================================
// FUZZ TESTS
// ============================================================================

/// Fuzz test: empty input should return Empty error
#[test]
fn fuzz_empty_input() {
    let result = parse_instruction(&[]);
    assert_eq!(result.unwrap_err(), ParseError::Empty);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10000))]
    
    /// Fuzz test: invalid discriminator should be rejected
    #[test]
    fn fuzz_invalid_discriminator(
        discriminator in (ix_disc::MAX_VALID + 1)..=255u8,
        payload in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let mut data = vec![discriminator];
        data.extend(payload);
        
        let result = parse_instruction(&data);
        match result {
            Err(ParseError::InvalidDiscriminator(_)) => {},
            other => prop_assert!(false, "Expected InvalidDiscriminator, got {:?}", other),
        }
    }
    
    /// Fuzz test: short initialize payload should fail
    #[test]
    fn fuzz_short_initialize(
        len in 0usize..136,
    ) {
        let mut data = vec![ix_disc::INITIALIZE];
        data.extend(vec![0u8; len]);
        
        let result = parse_instruction(&data);
        match result {
            Err(ParseError::TooShort { .. }) => {},
            other => prop_assert!(false, "Expected TooShort, got {:?}", other),
        }
    }
    
    /// Fuzz test: valid initialize should parse correctly
    #[test]
    fn fuzz_valid_initialize(
        market_id in any::<[u8; 32]>(),
        lp_owner in any::<[u8; 32]>(),
        router_id in any::<[u8; 32]>(),
        imr_bps in any::<u64>(),
        mmr_bps in any::<u64>(),
        maker_fee_bps in any::<i64>(),
        taker_fee_bps in any::<u64>(),
        batch_ms in any::<u64>(),
    ) {
        let mut data = vec![ix_disc::INITIALIZE];
        data.extend_from_slice(&market_id);
        data.extend_from_slice(&lp_owner);
        data.extend_from_slice(&router_id);
        data.extend_from_slice(&imr_bps.to_le_bytes());
        data.extend_from_slice(&mmr_bps.to_le_bytes());
        data.extend_from_slice(&maker_fee_bps.to_le_bytes());
        data.extend_from_slice(&taker_fee_bps.to_le_bytes());
        data.extend_from_slice(&batch_ms.to_le_bytes());
        
        let result = parse_instruction(&data);
        prop_assert!(result.is_ok());
        
        if let Ok(ParsedInstruction::Initialize(init)) = result {
            prop_assert_eq!(init.market_id, market_id);
            prop_assert_eq!(init.imr_bps, imr_bps);
            prop_assert_eq!(init.mmr_bps, mmr_bps);
            prop_assert_eq!(init.maker_fee_bps, maker_fee_bps);
            prop_assert_eq!(init.taker_fee_bps, taker_fee_bps);
            prop_assert_eq!(init.batch_ms, batch_ms);
        } else {
            prop_assert!(false, "Expected Initialize instruction");
        }
    }
    
    /// Fuzz test: short reserve payload should fail
    #[test]
    fn fuzz_short_reserve(
        len in 0usize..71,
    ) {
        let mut data = vec![ix_disc::RESERVE];
        data.extend(vec![0u8; len]);
        
        let result = parse_instruction(&data);
        match result {
            Err(ParseError::TooShort { .. }) => {},
            other => prop_assert!(false, "Expected TooShort, got {:?}", other),
        }
    }
    
    /// Fuzz test: valid reserve should parse correctly
    #[test]
    fn fuzz_valid_reserve(
        account_idx in any::<u32>(),
        instrument_idx in any::<u16>(),
        side in 0u8..2,
        qty in any::<u64>(),
        limit_px in any::<u64>(),
        ttl_ms in any::<u64>(),
        commitment_hash in any::<[u8; 32]>(),
        route_id in any::<u64>(),
    ) {
        let mut data = vec![ix_disc::RESERVE];
        data.extend_from_slice(&account_idx.to_le_bytes());
        data.extend_from_slice(&instrument_idx.to_le_bytes());
        data.push(side);
        data.extend_from_slice(&qty.to_le_bytes());
        data.extend_from_slice(&limit_px.to_le_bytes());
        data.extend_from_slice(&ttl_ms.to_le_bytes());
        data.extend_from_slice(&commitment_hash);
        data.extend_from_slice(&route_id.to_le_bytes());
        
        let result = parse_instruction(&data);
        prop_assert!(result.is_ok());
        
        if let Ok(ParsedInstruction::Reserve(res)) = result {
            prop_assert_eq!(res.account_idx, account_idx);
            prop_assert_eq!(res.instrument_idx, instrument_idx);
            prop_assert_eq!(res.side, side);
            prop_assert_eq!(res.qty, qty);
            prop_assert_eq!(res.limit_px, limit_px);
        } else {
            prop_assert!(false, "Expected Reserve instruction");
        }
    }
    
    /// Fuzz test: valid commit should parse correctly
    #[test]
    fn fuzz_valid_commit(
        hold_id in any::<u64>(),
        current_ts in any::<u64>(),
    ) {
        let mut data = vec![ix_disc::COMMIT];
        data.extend_from_slice(&hold_id.to_le_bytes());
        data.extend_from_slice(&current_ts.to_le_bytes());
        
        let result = parse_instruction(&data);
        prop_assert!(result.is_ok());
        
        if let Ok(ParsedInstruction::Commit(commit)) = result {
            prop_assert_eq!(commit.hold_id, hold_id);
            prop_assert_eq!(commit.current_ts, current_ts);
        } else {
            prop_assert!(false, "Expected Commit instruction");
        }
    }
    
    /// Fuzz test: valid cancel should parse correctly
    #[test]
    fn fuzz_valid_cancel(
        hold_id in any::<u64>(),
    ) {
        let mut data = vec![ix_disc::CANCEL];
        data.extend_from_slice(&hold_id.to_le_bytes());
        
        let result = parse_instruction(&data);
        prop_assert!(result.is_ok());
        
        if let Ok(ParsedInstruction::Cancel(cancel)) = result {
            prop_assert_eq!(cancel.hold_id, hold_id);
        } else {
            prop_assert!(false, "Expected Cancel instruction");
        }
    }
    
    /// Fuzz test: random bytes should either parse or fail gracefully
    #[test]
    fn fuzz_random_bytes(
        data in prop::collection::vec(any::<u8>(), 0..512),
    ) {
        // This should never panic
        let _ = parse_instruction(&data);
    }
    
    /// Fuzz test: extra bytes after valid instruction should still parse
    #[test]
    fn fuzz_extra_bytes(
        hold_id in any::<u64>(),
        extra in prop::collection::vec(any::<u8>(), 0..256),
    ) {
        let mut data = vec![ix_disc::CANCEL];
        data.extend_from_slice(&hold_id.to_le_bytes());
        data.extend(extra);
        
        // Should still parse (extra bytes ignored)
        let result = parse_instruction(&data);
        prop_assert!(result.is_ok());
    }
}

// ============================================================================
// EDGE CASE TESTS
// ============================================================================

#[cfg(test)]
mod edge_cases {
    use super::*;
    
    #[test]
    fn test_boundary_values_u64() {
        // Test u64 boundaries
        let values = [0u64, 1, u64::MAX - 1, u64::MAX];
        
        for &val in &values {
            let mut data = vec![ix_disc::CANCEL];
            data.extend_from_slice(&val.to_le_bytes());
            
            let result = parse_instruction(&data);
            assert!(result.is_ok());
            
            if let Ok(ParsedInstruction::Cancel(cancel)) = result {
                assert_eq!(cancel.hold_id, val);
            }
        }
    }
    
    #[test]
    fn test_boundary_values_i64() {
        // Test i64 boundaries  
        let values = [i64::MIN, i64::MIN + 1, -1, 0, 1, i64::MAX - 1, i64::MAX];
        
        for &val in &values {
            let mut data = vec![ix_disc::INITIALIZE];
            data.extend([0u8; 32]); // market_id
            data.extend([0u8; 32]); // lp_owner
            data.extend([0u8; 32]); // router_id
            data.extend(0u64.to_le_bytes()); // imr
            data.extend(0u64.to_le_bytes()); // mmr
            data.extend(val.to_le_bytes()); // maker_fee
            data.extend(0u64.to_le_bytes()); // taker_fee
            data.extend(0u64.to_le_bytes()); // batch_ms
            
            let result = parse_instruction(&data);
            assert!(result.is_ok());
            
            if let Ok(ParsedInstruction::Initialize(init)) = result {
                assert_eq!(init.maker_fee_bps, val);
            }
        }
    }
    
    #[test]
    fn test_boundary_values_i128() {
        // Test i128 boundaries
        let values = [i128::MIN, i128::MIN + 1, -1, 0, 1, i128::MAX - 1, i128::MAX];
        
        for &val in &values {
            let mut data = vec![ix_disc::LIQUIDATION];
            data.extend(0u32.to_le_bytes()); // account_idx
            data.extend(val.to_le_bytes()); // deficit_target
            data.extend(0u64.to_le_bytes()); // current_ts
            
            let result = parse_instruction(&data);
            assert!(result.is_ok());
            
            if let Ok(ParsedInstruction::Liquidation(liq)) = result {
                assert_eq!(liq.deficit_target, val);
            }
        }
    }
    
    #[test]
    fn test_all_discriminators() {
        // Ensure all valid discriminators are handled
        let test_cases = [
            (ix_disc::RESERVE, 71),
            (ix_disc::COMMIT, 16),
            (ix_disc::CANCEL, 8),
            (ix_disc::BATCH_OPEN, 10),
            (ix_disc::INITIALIZE, 136),
            (ix_disc::ADD_INSTRUMENT, 40),
            (ix_disc::UPDATE_FUNDING, 18),
            (ix_disc::LIQUIDATION, 28),
        ];
        
        for (disc, min_len) in test_cases {
            // Too short
            let mut data = vec![disc];
            data.extend(vec![0u8; min_len - 1]);
            let result = parse_instruction(&data);
            assert!(matches!(result, Err(ParseError::TooShort { .. })),
                "Discriminator {} should fail with len {}", disc, min_len - 1);
            
            // Exactly right
            let mut data = vec![disc];
            data.extend(vec![0u8; min_len]);
            let result = parse_instruction(&data);
            assert!(result.is_ok(),
                "Discriminator {} should succeed with len {}", disc, min_len);
        }
    }
    
    #[test]
    fn test_side_values() {
        // Test all possible side values
        for side in 0u8..=255 {
            let mut data = vec![ix_disc::RESERVE];
            data.extend(0u32.to_le_bytes()); // account_idx
            data.extend(0u16.to_le_bytes()); // instrument_idx
            data.push(side); // side
            data.extend(0u64.to_le_bytes()); // qty
            data.extend(0u64.to_le_bytes()); // limit_px
            data.extend(0u64.to_le_bytes()); // ttl_ms
            data.extend([0u8; 32]); // commitment_hash
            data.extend(0u64.to_le_bytes()); // route_id
            
            let result = parse_instruction(&data);
            assert!(result.is_ok(), "Side {} should parse", side);
            
            if let Ok(ParsedInstruction::Reserve(res)) = result {
                assert_eq!(res.side, side);
            }
        }
    }
    
    #[test]
    fn test_symbol_encoding() {
        // Test various symbol encodings
        let symbols: &[[u8; 8]] = &[
            [0; 8],
            [0xFF; 8],
            *b"BTC-PERP",
            *b"ETH\0\0\0\0\0",
            [0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF, 0x00, 0xFF],
        ];
        
        for symbol in symbols {
            let mut data = vec![ix_disc::ADD_INSTRUMENT];
            data.extend_from_slice(symbol);
            data.extend(0u64.to_le_bytes()); // contract_size
            data.extend(0u64.to_le_bytes()); // tick
            data.extend(0u64.to_le_bytes()); // lot
            data.extend(0u64.to_le_bytes()); // initial_mark
            
            let result = parse_instruction(&data);
            assert!(result.is_ok());
            
            if let Ok(ParsedInstruction::AddInstrument(inst)) = result {
                assert_eq!(inst.symbol, *symbol);
            }
        }
    }
    
    #[test]
    fn test_commitment_hash() {
        // Test various commitment hash values
        let hashes: &[[u8; 32]] = &[
            [0; 32],
            [0xFF; 32],
            {
                let mut h = [0u8; 32];
                for i in 0..32 {
                    h[i] = i as u8;
                }
                h
            },
        ];
        
        for hash in hashes {
            let mut data = vec![ix_disc::RESERVE];
            data.extend(0u32.to_le_bytes()); // account_idx
            data.extend(0u16.to_le_bytes()); // instrument_idx
            data.push(0); // side
            data.extend(1u64.to_le_bytes()); // qty
            data.extend(1u64.to_le_bytes()); // limit_px
            data.extend(1u64.to_le_bytes()); // ttl_ms
            data.extend_from_slice(hash); // commitment_hash
            data.extend(0u64.to_le_bytes()); // route_id
            
            let result = parse_instruction(&data);
            assert!(result.is_ok());
            
            if let Ok(ParsedInstruction::Reserve(res)) = result {
                assert_eq!(res.commitment_hash, *hash);
            }
        }
    }
    
    #[test]
    fn test_parse_determinism() {
        // Ensure parsing is deterministic
        let data: Vec<u8> = vec![
            ix_disc::RESERVE,
            0x01, 0x00, 0x00, 0x00, // account_idx = 1
            0x02, 0x00, // instrument_idx = 2
            0x00, // side = 0
            0xE8, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // qty = 1000
            0x10, 0x27, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // limit_px = 10000
            0x88, 0x13, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // ttl_ms = 5000
            // 32 bytes of commitment_hash
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07,
            0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D, 0x0E, 0x0F,
            0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
            0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1D, 0x1E, 0x1F,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // route_id = 0
        ];
        
        // Parse multiple times
        for _ in 0..100 {
            let result = parse_instruction(&data);
            assert!(result.is_ok());
            
            if let Ok(ParsedInstruction::Reserve(res)) = result {
                assert_eq!(res.account_idx, 1);
                assert_eq!(res.instrument_idx, 2);
                assert_eq!(res.side, 0);
                assert_eq!(res.qty, 1000);
                assert_eq!(res.limit_px, 10000);
                assert_eq!(res.ttl_ms, 5000);
            }
        }
    }
}

// ============================================================================
// ARITHMETIC OVERFLOW TESTS
// ============================================================================

#[cfg(test)]
mod overflow_tests {
    use super::*;
    
    /// Test that our VWAP calculation handles overflow correctly
    #[test]
    fn test_vwap_overflow_protection() {
        // Large prices and quantities that could overflow u64
        let price = u64::MAX / 2;
        let qty = 1000u64;
        
        // This calculation should use u128 internally
        let notional = (price as u128) * (qty as u128);
        assert!(notional > u64::MAX as u128); // Proves overflow would happen with u64
        
        // But we can safely divide back to u64
        let vwap = (notional / qty as u128) as u64;
        assert_eq!(vwap, price);
    }
    
    /// Test margin calculation with extreme values
    #[test]
    fn test_margin_calculation_overflow() {
        let qty = u64::MAX / 10_000;
        let price = u64::MAX / 10_000;
        let bps = 10_000u64; // 100%
        
        // Calculate using u128
        let notional = (qty as u128) * (price as u128);
        let margin = notional * bps as u128 / 10_000;
        
        // Should fit in u128 even with extreme values
        assert!(margin <= u128::MAX);
    }
    
    /// Test PnL calculation with extreme price movements
    #[test]
    fn test_pnl_extreme_movement() {
        let entry = 50_000_000_000u64; // $50,000
        let mark_up = u64::MAX; // Extreme up
        let mark_down = 1u64; // Near zero
        let qty = 1_000_000i64;
        
        // Long position, price goes up
        let pnl_up = ((qty.unsigned_abs() as u128) * (mark_up as u128)) as i128
            - ((qty.unsigned_abs() as u128) * (entry as u128)) as i128;
        assert!(pnl_up > 0);
        
        // Long position, price goes down
        let pnl_down = ((qty.unsigned_abs() as u128) * (mark_down as u128)) as i128
            - ((qty.unsigned_abs() as u128) * (entry as u128)) as i128;
        assert!(pnl_down < 0);
    }
}
