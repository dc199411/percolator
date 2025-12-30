//! Percolator Protocol Client

use solana_client::rpc_client::RpcClient;
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::constants::*;
use crate::error::{PercolatorSdkError, Result};
use crate::instructions::*;
use crate::pda::*;
use crate::types::*;

/// Percolator client configuration
pub struct PercolatorClientConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// USDC mint address
    pub usdc_mint: Pubkey,
    /// Commitment level
    pub commitment: CommitmentConfig,
}

impl Default for PercolatorClientConfig {
    fn default() -> Self {
        Self {
            rpc_url: "https://api.devnet.solana.com".to_string(),
            usdc_mint: Pubkey::default(),
            commitment: CommitmentConfig::confirmed(),
        }
    }
}

/// Main client for interacting with Percolator Protocol
pub struct PercolatorClient {
    rpc: Arc<RpcClient>,
    config: PercolatorClientConfig,
}

impl PercolatorClient {
    /// Create a new client with the given RPC URL
    pub fn new(rpc_url: &str) -> Result<Self> {
        let config = PercolatorClientConfig {
            rpc_url: rpc_url.to_string(),
            ..Default::default()
        };
        Self::with_config(config)
    }

    /// Create a new client with full configuration
    pub fn with_config(config: PercolatorClientConfig) -> Result<Self> {
        let rpc = RpcClient::new_with_commitment(config.rpc_url.clone(), config.commitment);
        Ok(Self {
            rpc: Arc::new(rpc),
            config,
        })
    }

    // ==========================================================================
    // PDA GETTERS
    // ==========================================================================

    /// Get router registry PDA
    pub fn registry_pda(&self) -> Pubkey {
        let (pda, _) = derive_registry_pda();
        pda
    }

    /// Get router vault PDA
    pub fn vault_pda(&self) -> Pubkey {
        let (pda, _) = derive_vault_pda();
        pda
    }

    /// Get portfolio PDA for user
    pub fn portfolio_pda(&self, owner: &Pubkey) -> Pubkey {
        let (pda, _) = derive_portfolio_pda(owner);
        pda
    }

    /// Get insurance pool PDA for slab
    pub fn insurance_pda(&self, slab_state: &Pubkey) -> Pubkey {
        let (pda, _) = derive_insurance_pda(slab_state);
        pda
    }

    // ==========================================================================
    // ACCOUNT FETCHING
    // ==========================================================================

    /// Fetch user portfolio
    pub fn get_portfolio(&self, owner: &Pubkey) -> Result<Option<UserPortfolio>> {
        let portfolio_pda = self.portfolio_pda(owner);
        let account = self
            .rpc
            .get_account_with_commitment(&portfolio_pda, self.config.commitment)
            .map_err(|e| PercolatorSdkError::RpcError(e.to_string()))?;

        if let Some(account) = account.value {
            let portfolio = borsh::from_slice::<UserPortfolio>(&account.data)
                .map_err(|e| PercolatorSdkError::DeserializationError(e.to_string()))?;
            Ok(Some(portfolio))
        } else {
            Ok(None)
        }
    }

    /// Check if portfolio exists
    pub fn portfolio_exists(&self, owner: &Pubkey) -> Result<bool> {
        let portfolio_pda = self.portfolio_pda(owner);
        let balance = self
            .rpc
            .get_balance(&portfolio_pda)
            .map_err(|e| PercolatorSdkError::RpcError(e.to_string()))?;
        Ok(balance > 0)
    }

    // ==========================================================================
    // TRANSACTION BUILDERS
    // ==========================================================================

    /// Build initialize portfolio instruction
    pub fn build_initialize_portfolio(&self, owner: &Pubkey) -> Instruction {
        create_initialize_portfolio_instruction(owner)
    }

    /// Build deposit instruction
    pub fn build_deposit(&self, owner: &Pubkey, token_account: &Pubkey, amount: u64) -> Instruction {
        let params = DepositParams { amount };
        create_deposit_instruction(owner, token_account, &params)
    }

    /// Build withdraw instruction
    pub fn build_withdraw(&self, owner: &Pubkey, token_account: &Pubkey, amount: u64) -> Instruction {
        let params = WithdrawParams { amount };
        create_withdraw_instruction(owner, token_account, &params)
    }

    /// Build multi-slab reserve instruction
    pub fn build_multi_slab_reserve(
        &self,
        owner: &Pubkey,
        slab_accounts: &[Pubkey],
        splits: Vec<SlabSplit>,
        request_id: Option<u64>,
    ) -> Instruction {
        let total_qty = splits.iter().map(|s| s.qty).sum();
        let id = request_id.unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        });
        let expiry_ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs()
            + 60;

        let params = MultiSlabReserveParams {
            splits,
            total_qty,
            request_id: id,
            expiry_ts,
        };

        create_multi_slab_reserve_instruction(owner, slab_accounts, &params)
    }

    /// Build multi-slab commit instruction
    pub fn build_multi_slab_commit(
        &self,
        owner: &Pubkey,
        slab_accounts: &[Pubkey],
        request_id: u64,
        hold_ids: &[u64],
    ) -> Instruction {
        create_multi_slab_commit_instruction(owner, slab_accounts, request_id, hold_ids)
    }

    /// Build liquidation instruction
    pub fn build_liquidate(
        &self,
        liquidator: &Pubkey,
        target_owner: &Pubkey,
        slab_accounts: &[Pubkey],
    ) -> Instruction {
        let target_portfolio = self.portfolio_pda(target_owner);
        create_global_liquidation_instruction(liquidator, &target_portfolio, slab_accounts)
    }

    // ==========================================================================
    // INSURANCE OPERATIONS
    // ==========================================================================

    /// Build initialize insurance instruction
    pub fn build_initialize_insurance(
        &self,
        slab_state: &Pubkey,
        lp_owner: &Pubkey,
        contribution_rate_bps: u64,
        adl_threshold_bps: u64,
        withdrawal_timelock_secs: u64,
    ) -> Instruction {
        let params = InitializeInsuranceParams {
            contribution_rate_bps,
            adl_threshold_bps,
            withdrawal_timelock_secs,
        };
        create_initialize_insurance_instruction(slab_state, lp_owner, &params)
    }

    /// Build contribute insurance instruction
    pub fn build_contribute_insurance(
        &self,
        slab_state: &Pubkey,
        lp_owner: &Pubkey,
        lp_token_account: &Pubkey,
        insurance_vault: &Pubkey,
        amount: u64,
    ) -> Instruction {
        let params = ContributeInsuranceParams { amount };
        create_contribute_insurance_instruction(
            slab_state,
            lp_owner,
            lp_token_account,
            insurance_vault,
            &params,
        )
    }

    /// Build initiate insurance withdrawal instruction
    pub fn build_initiate_insurance_withdrawal(
        &self,
        slab_state: &Pubkey,
        lp_owner: &Pubkey,
        amount: u64,
    ) -> Instruction {
        let params = InitiateWithdrawalParams { amount };
        create_initiate_insurance_withdrawal_instruction(slab_state, lp_owner, &params)
    }

    // ==========================================================================
    // TRANSACTION SENDING
    // ==========================================================================

    /// Send and confirm a transaction
    pub fn send_transaction(
        &self,
        instructions: &[Instruction],
        signers: &[&Keypair],
        payer: &Pubkey,
    ) -> Result<Signature> {
        let recent_blockhash = self
            .rpc
            .get_latest_blockhash()
            .map_err(|e| PercolatorSdkError::RpcError(e.to_string()))?;

        let mut transaction = Transaction::new_with_payer(instructions, Some(payer));
        transaction.sign(signers, recent_blockhash);

        let signature = self
            .rpc
            .send_and_confirm_transaction(&transaction)
            .map_err(|e| PercolatorSdkError::TransactionFailed(e.to_string()))?;

        Ok(signature)
    }

    /// Send transaction without waiting for confirmation
    pub fn send_transaction_no_wait(
        &self,
        instructions: &[Instruction],
        signers: &[&Keypair],
        payer: &Pubkey,
    ) -> Result<Signature> {
        let recent_blockhash = self
            .rpc
            .get_latest_blockhash()
            .map_err(|e| PercolatorSdkError::RpcError(e.to_string()))?;

        let mut transaction = Transaction::new_with_payer(instructions, Some(payer));
        transaction.sign(signers, recent_blockhash);

        let signature = self
            .rpc
            .send_transaction(&transaction)
            .map_err(|e| PercolatorSdkError::TransactionFailed(e.to_string()))?;

        Ok(signature)
    }

    // ==========================================================================
    // MARGIN CALCULATIONS
    // ==========================================================================

    /// Calculate portfolio margin requirements
    pub fn calculate_portfolio_margin(
        &self,
        collateral: u64,
        unrealized_pnl: i64,
        positions: &[PositionInfo],
    ) -> PortfolioMarginResult {
        let mut gross_im: u64 = 0;
        let mut gross_mm: u64 = 0;

        // Calculate gross margins
        for pos in positions {
            if pos.qty == 0 {
                continue;
            }

            let notional = (pos.qty.unsigned_abs() as u128 * pos.last_mark_price as u128
                / PRICE_SCALE as u128) as u64;
            let im = notional * DEFAULT_IMR_BPS / BPS_SCALE;
            let mm = notional * DEFAULT_MMR_BPS / BPS_SCALE;

            gross_im = gross_im.saturating_add(im);
            gross_mm = gross_mm.saturating_add(mm);
        }

        // Calculate net margins (simplified - real impl would group by instrument)
        let net_im = gross_im;
        let net_mm = gross_mm;
        let netting_benefit = gross_im.saturating_sub(net_im);

        let equity = collateral as i64 + unrealized_pnl;
        let available_margin = equity - net_im as i64;

        let margin_ratio_bps = if net_im > 0 {
            (equity as u64 * BPS_SCALE) / net_im
        } else {
            BPS_SCALE * 10 // 1000% if no margin required
        };

        PortfolioMarginResult {
            gross_im,
            net_im,
            gross_mm,
            net_mm,
            netting_benefit,
            available_margin,
            margin_ratio_bps,
        }
    }

    /// Check if portfolio is liquidatable
    pub fn is_liquidatable(&self, collateral: u64, unrealized_pnl: i64, maintenance_margin: u64) -> bool {
        let equity = collateral as i64 + unrealized_pnl;
        equity < maintenance_margin as i64
    }
}

// ============================================================================
// UTILITY FUNCTIONS
// ============================================================================

/// Convert human-readable USDC amount to on-chain format
pub fn usdc_to_raw(amount: f64) -> u64 {
    (amount * USDC_SCALE as f64) as u64
}

/// Convert on-chain USDC amount to human-readable format
pub fn usdc_from_raw(raw_amount: u64) -> f64 {
    raw_amount as f64 / USDC_SCALE as f64
}

/// Convert human-readable price to on-chain format
pub fn price_to_raw(price: f64) -> u64 {
    (price * PRICE_SCALE as f64) as u64
}

/// Convert on-chain price to human-readable format
pub fn price_from_raw(raw_price: u64) -> f64 {
    raw_price as f64 / PRICE_SCALE as f64
}

/// Convert human-readable quantity to on-chain format
pub fn qty_to_raw(qty: f64) -> u64 {
    (qty * QTY_SCALE as f64) as u64
}

/// Convert on-chain quantity to human-readable format
pub fn qty_from_raw(raw_qty: u64) -> f64 {
    raw_qty as f64 / QTY_SCALE as f64
}

/// Calculate PnL for a position
pub fn calculate_pnl(qty: i64, entry_price: u64, current_price: u64) -> i64 {
    let price_diff = current_price as i64 - entry_price as i64;
    qty * price_diff / PRICE_SCALE as i64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_usdc_conversion() {
        assert_eq!(usdc_to_raw(100.0), 100_000_000);
        assert_eq!(usdc_from_raw(100_000_000), 100.0);
    }

    #[test]
    fn test_price_conversion() {
        assert_eq!(price_to_raw(50000.0), 50_000_000_000);
        assert_eq!(price_from_raw(50_000_000_000), 50000.0);
    }

    #[test]
    fn test_qty_conversion() {
        assert_eq!(qty_to_raw(1.5), 1_500_000);
        assert_eq!(qty_from_raw(1_500_000), 1.5);
    }

    #[test]
    fn test_pnl_calculation() {
        // Long 1 BTC, entry $50k, current $55k = $5k profit
        let pnl = calculate_pnl(1_000_000, 50_000_000_000, 55_000_000_000);
        assert_eq!(pnl, 5_000_000); // $5 in scaled units

        // Short 1 BTC, entry $50k, current $55k = $5k loss
        let pnl = calculate_pnl(-1_000_000, 50_000_000_000, 55_000_000_000);
        assert_eq!(pnl, -5_000_000);
    }

    #[test]
    fn test_margin_calculation() {
        let client = PercolatorClient::new("http://localhost:8899").unwrap();
        
        let positions = vec![PositionInfo {
            slab_index: 0,
            instrument_index: 0,
            qty: 1_000_000, // 1 unit long
            entry_price: 50_000_000_000,
            entry_value: 50_000_000_000,
            last_mark_price: 50_000_000_000,
            unrealized_pnl: 0,
        }];

        let result = client.calculate_portfolio_margin(
            10_000_000_000, // 10k collateral
            0,
            &positions,
        );

        // 1 unit * $50k = $50k notional, 5% IM = $2500
        assert!(result.gross_im > 0);
    }
}
