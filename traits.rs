//! Unified DEX trait system for eliminating repetitive code across DEX implementations

use async_trait::async_trait;
use solana_sdk::pubkey::Pubkey;
use std::collections::HashMap;
use anyhow::Result;

/// Common pool information that all DEXes must provide
#[derive(Debug, Clone)]
pub struct PoolInfo {
    pub pool_address: Pubkey,
    pub token_mint: Pubkey,
    pub base_mint: Pubkey,
    pub token_vault: Pubkey,
    pub base_vault: Pubkey,
    pub fee_wallet: Option<Pubkey>,
    pub additional_accounts: HashMap<String, Pubkey>,
}

/// Price information for a token pair
#[derive(Debug, Clone)]
pub struct PriceInfo {
    pub price: f64,
    pub liquidity: u64,
    pub fee: f64,
}

/// Unified DEX trait that all DEX implementations must satisfy
#[async_trait]
pub trait Dex: Send + Sync {
    /// Get the DEX name for identification
    fn name(&self) -> &'static str;

    /// Get the program ID for this DEX
    fn program_id(&self) -> Pubkey;

    /// Fetch pool information for given pool addresses
    async fn fetch_pools(&self, pool_addresses: &[String], token_mint: &Pubkey) -> Result<Vec<PoolInfo>>;

    /// Calculate price for a specific pool
    async fn calculate_price(&self, pool_info: &PoolInfo) -> Result<PriceInfo>;

    /// Get swap instruction data (DEX-specific)
    fn get_swap_instruction_data(&self, pool_info: &PoolInfo, amount_in: u64, minimum_out: u64) -> Result<Vec<u8>>;
}

/// Registry for managing all DEX implementations
pub struct DexRegistry {
    dexes: HashMap<&'static str, Box<dyn Dex>>,
}

impl DexRegistry {
    pub fn new() -> Self {
        Self {
            dexes: HashMap::new(),
        }
    }

    pub fn register<D: Dex + 'static>(&mut self, dex: D) {
        self.dexes.insert(dex.name(), Box::new(dex));
    }

    pub fn get(&self, name: &str) -> Option<&dyn Dex> {
        self.dexes.get(name).map(|d| d.as_ref())
    }

    pub fn all_dexes(&self) -> Vec<&dyn Dex> {
        self.dexes.values().map(|d| d.as_ref()).collect()
    }
}

/// Macro for generating common DEX boilerplate
#[macro_export]
macro_rules! dex_boilerplate {
    ($struct_name:ident, $name:expr, $program_id:expr) => {
        impl $struct_name {
            pub fn new() -> Self {
                Self
            }
        }

        impl Dex for $struct_name {
            fn name(&self) -> &'static str {
                $name
            }

            fn program_id(&self) -> Pubkey {
                $program_id
            }
        }
    };
}
