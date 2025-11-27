use crate::{
    chain::{
        pools::{MintPoolData, PumpPool, RaydiumPool},
        constants::sol_mint,
    },
    dex::{
        traits::{Dex, DexRegistry, PoolInfo},
        pump::PumpDex,
        raydium::RaydiumDex,
    },
};
use anyhow::{anyhow, Result};
use solana_client::rpc_client::RpcClient;
use solana_sdk::{account::Account, pubkey::Pubkey};
use spl_associated_token_account;
use std::{
    collections::HashMap,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::time::sleep;
use tracing::{error, info, warn};

const TOKEN_2022_PROGRAM_ID: Pubkey = Pubkey::new_from_array([
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 134, 244,
    64, 118, 252, 1, 16, 241, 37, 236, 114, 157, 18, 16,
]);

/// Configuration for token fetching
#[derive(Debug, Clone)]
pub struct TokenFetchConfig {
    pub max_retries: u32,
    pub retry_delay_ms: u64,
    pub batch_size: usize,
    pub timeout_seconds: u64,
    pub enable_caching: bool,
    pub cache_ttl_seconds: u64,
}

impl Default for TokenFetchConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            retry_delay_ms: 1000,
            batch_size: 10,
            timeout_seconds: 30,
            enable_caching: true,
            cache_ttl_seconds: 300, // 5 minutes
        }
    }
}

/// Cache entry for token data
#[derive(Debug, Clone)]
struct CacheEntry {
    data: MintPoolData,
    timestamp: Instant,
}

/// Enhanced token fetcher with caching and retry logic
pub struct TokenFetcher {
    rpc_client: Arc<RpcClient>,
    config: TokenFetchConfig,
    cache: HashMap<String, CacheEntry>,
}

impl TokenFetcher {
    pub fn new(rpc_client: Arc<RpcClient>, config: TokenFetchConfig) -> Self {
        Self {
            rpc_client,
            config,
            cache: HashMap::new(),
        }
    }

    /// Initialize pool data with enhanced error handling and caching
    pub async fn initialize_pool_data(
        &mut self,
        mint: &str,
        wallet_account: &str,
        raydium_pools: Option<&Vec<String>>,
        raydium_cp_pools: Option<&Vec<String>>,
        pump_pools: Option<&Vec<String>>,
        dlmm_pools: Option<&Vec<String>>,
        whirlpool_pools: Option<&Vec<String>>,
        raydium_clmm_pools: Option<&Vec<String>>,
        meteora_damm_pools: Option<&Vec<String>>,
        solfi_pools: Option<&Vec<String>>,
        meteora_damm_v2_pools: Option<&Vec<String>>,
        vertigo_pools: Option<&Vec<String>>,
    ) -> Result<MintPoolData> {
        let cache_key = format!("{}_{}", mint, wallet_account);
        
        // Check cache first
        if self.config.enable_caching {
            if let Some(entry) = self.cache.get(&cache_key) {
                if entry.timestamp.elapsed().as_secs() < self.config.cache_ttl_seconds {
                    info!("Using cached pool data for mint: {}", mint);
                    return Ok(entry.data.clone());
                }
            }
        }

        info!("Initializing pool data for mint: {}", mint);
        let start_time = Instant::now();

        // Fetch mint account with retry logic
        let mint_pubkey = Pubkey::from_str(mint)?;
        let mint_account = self.fetch_account_with_retry(&mint_pubkey).await?;

        // Determine token program based on mint account owner
        let token_program = self.determine_token_program(&mint_account, mint)?;
        info!("Detected token program: {}", token_program);

        let mut pool_data = MintPoolData::new(mint, wallet_account, token_program)?;
        info!("Pool data initialized for mint: {}", mint);

        // Create DEX registry with unified implementations
        let mut dex_registry = DexRegistry::new();
        dex_registry.register(PumpDex::new(self.rpc_client.clone()));
        dex_registry.register(RaydiumDex::new(self.rpc_client.clone()));

        // Unified pool fetching using the registry
        let pool_configs = vec![
            ("pump", pump_pools),
            ("raydium", raydium_pools),
            // TODO: Add other DEXes as they are implemented
        ];

        for (dex_name, pool_list) in pool_configs {
            if let Some(pool_addresses) = pool_list {
                if let Some(dex) = dex_registry.get(dex_name) {
                    match dex.fetch_pools(pool_addresses, &mint_pubkey).await {
                        Ok(pools) => {
                            // Convert unified PoolInfo to legacy pool types
                            self.convert_and_add_pools(&mut pool_data, dex_name, pools).await?;
                            info!("Successfully fetched {} pools from {}", pools.len(), dex_name);
                        }
                        Err(e) => {
                            warn!("Failed to fetch {} pools: {}", dex_name, e);
                        }
                    }
                }
            }
        }

        // Cache the result
        if self.config.enable_caching {
            self.cache.insert(
                cache_key,
                CacheEntry {
                    data: pool_data.clone(),
                    timestamp: Instant::now(),
                },
            );
        }

        let elapsed = start_time.elapsed();
        info!(
            "Pool data initialization completed for mint: {} in {:?}",
            mint, elapsed
        );

        Ok(pool_data)
    }

    /// Fetch account with retry logic
    async fn fetch_account_with_retry(&self, pubkey: &Pubkey) -> Result<Account> {
        let mut last_error = None;
        
        for attempt in 0..self.config.max_retries {
            match self.rpc_client.get_account(pubkey) {
                Ok(account) => return Ok(account),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < self.config.max_retries - 1 {
                        warn!(
                            "Failed to fetch account {} (attempt {}/{}), retrying in {}ms",
                            pubkey, attempt + 1, self.config.max_retries, self.config.retry_delay_ms
                        );
                        sleep(Duration::from_millis(self.config.retry_delay_ms)).await;
                    }
                }
            }
        }

        Err(anyhow!(
            "Failed to fetch account {} after {} attempts: {:?}",
            pubkey,
            self.config.max_retries,
            last_error
        ))
    }

    /// Determine token program from mint account
    fn determine_token_program(&self, mint_account: &Account, mint: &str) -> Result<Pubkey> {
        if mint_account.owner == spl_token::ID {
            Ok(spl_token::ID)
        } else if mint_account.owner == TOKEN_2022_PROGRAM_ID {
            Ok(TOKEN_2022_PROGRAM_ID)
        } else {
            Err(anyhow!("Unknown token program for mint: {}", mint))
        }
    }

    /// Convert unified PoolInfo to legacy pool types and add to pool_data
    async fn convert_and_add_pools(&self, pool_data: &mut MintPoolData, dex_name: &str, pools: Vec<PoolInfo>) -> Result<()> {
        match dex_name {
            "pump" => {
                for pool_info in pools {
                    // For Pump pools, we need additional account info
                    let coin_creator_vault_ata = pool_info.additional_accounts
                        .get("coin_creator_vault_ata")
                        .copied()
                        .ok_or_else(|| anyhow!("Missing coin_creator_vault_ata for Pump pool"))?;

                    let pump_pool = PumpPool {
                        pool: pool_info.pool_address,
                        token_vault: pool_info.token_vault,
                        sol_vault: pool_info.base_vault,
                        fee_token_wallet: pool_info.fee_wallet.unwrap_or_default(),
                        coin_creator_vault_ata,
                        coin_creator_vault_authority: pool_info.additional_accounts
                            .get("coin_creator_vault_authority")
                            .copied()
                            .unwrap_or_default(), // This would need to be fetched separately
                        token_mint: pool_info.token_mint,
                        base_mint: pool_info.base_mint,
                    };
                    pool_data.pump_pools.push(pump_pool);
                }
            }
            "raydium" => {
                for pool_info in pools {
                    let raydium_pool = RaydiumPool {
                        pool: pool_info.pool_address,
                        token_vault: pool_info.token_vault,
                        sol_vault: pool_info.base_vault,
                        token_mint: pool_info.token_mint,
                        base_mint: pool_info.base_mint,
                    };
                    pool_data.raydium_pools.push(raydium_pool);
                }
            }
            _ => {
                warn!("Unknown DEX type: {}", dex_name);
            }
        }
        Ok(())
    }

    /// Clear expired cache entries
    pub fn clear_expired_cache(&mut self) {
        let now = Instant::now();
        self.cache.retain(|_, entry| {
            now.duration_since(entry.timestamp).as_secs() < self.config.cache_ttl_seconds
        });
    }

    /// Get cache statistics
    pub fn get_cache_stats(&self) -> (usize, usize) {
        let total_entries = self.cache.len();
        let expired_entries = self
            .cache
            .values()
            .filter(|entry| {
                entry.timestamp.elapsed().as_secs() >= self.config.cache_ttl_seconds
            })
            .count();
        (total_entries, expired_entries)
    }
}
