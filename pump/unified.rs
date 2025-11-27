//! Unified Pump DEX implementation using the Dex trait

use crate::dex::traits::{Dex, PoolInfo, PriceInfo};
use crate::dex::pump::{amm_info::PumpAmmInfo, constants::*};
use async_trait::async_trait;
use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::sync::Arc;
use spl_associated_token_account;
use anyhow::Result;

pub struct PumpDex {
    rpc_client: Arc<RpcClient>,
}

dex_boilerplate!(PumpDex, "pump", pump_program_id());

#[async_trait]
impl Dex for PumpDex {
    async fn fetch_pools(&self, pool_addresses: &[String], token_mint: &Pubkey) -> Result<Vec<PoolInfo>> {
        let mut pools = Vec::new();

        for pool_address in pool_addresses {
            match self.fetch_single_pool(pool_address, token_mint).await {
                Ok(pool) => pools.push(pool),
                Err(e) => {
                    tracing::error!("Failed to fetch Pump pool {}: {}", pool_address, e);
                }
            }
        }

        Ok(pools)
    }

    async fn calculate_price(&self, pool_info: &PoolInfo) -> Result<PriceInfo> {
        // Pump.fun has a bonding curve pricing mechanism
        // For now, return a placeholder - would need actual bonding curve calculation
        Ok(PriceInfo {
            price: 0.0, // TODO: Implement Pump.fun bonding curve pricing
            liquidity: 0, // TODO: Calculate actual liquidity
            fee: 0.01, // Pump.fun fee
        })
    }

    fn get_swap_instruction_data(&self, pool_info: &PoolInfo, amount_in: u64, minimum_out: u64) -> Result<Vec<u8>> {
        // TODO: Implement Pump.fun swap instruction encoding
        Ok(Vec::new())
    }
}

impl PumpDex {
    pub fn new(rpc_client: Arc<RpcClient>) -> Self {
        Self { rpc_client }
    }

    async fn fetch_single_pool(&self, pool_address: &str, token_mint: &Pubkey) -> Result<PoolInfo> {
        let pool_pubkey = Pubkey::from_str(pool_address)?;
        let account = self.rpc_client.get_account(&pool_pubkey)?;

        if account.owner != pump_program_id() {
            return Err(anyhow::anyhow!(
                "Account is not owned by Pump program: {}",
                pool_address
            ));
        }

        let amm_info = PumpAmmInfo::load_checked(&account.data)?;

        let (token_vault, base_vault) = if crate::chain::constants::sol_mint() == amm_info.base_mint {
            (amm_info.pool_base_token_account, amm_info.pool_quote_token_account)
        } else if crate::chain::constants::sol_mint() == amm_info.quote_mint {
            (amm_info.pool_quote_token_account, amm_info.pool_base_token_account)
        } else {
            (amm_info.pool_base_token_account, amm_info.pool_quote_token_account)
        };

        let fee_token_wallet = spl_associated_token_account::get_associated_token_address(
            &pump_fee_wallet(),
            &amm_info.quote_mint,
        );

        let coin_creator_vault_ata = spl_associated_token_account::get_associated_token_address(
            &amm_info.coin_creator_vault_authority,
            &amm_info.quote_mint,
        );

        let (token_mint_final, base_mint) = if *token_mint == amm_info.base_mint {
            (amm_info.base_mint, amm_info.quote_mint)
        } else {
            (amm_info.quote_mint, amm_info.base_mint)
        };

        let mut additional_accounts = std::collections::HashMap::new();
        additional_accounts.insert("coin_creator_vault_ata".to_string(), coin_creator_vault_ata);

        Ok(PoolInfo {
            pool_address: pool_pubkey,
            token_mint: token_mint_final,
            base_mint,
            token_vault,
            base_vault,
            fee_wallet: Some(fee_token_wallet),
            additional_accounts,
        })
    }
}
