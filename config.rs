use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub bot: BotConfig,
    pub routing: RoutingConfig,
    pub rpc: RpcConfig,
    pub spam: Option<SpamConfig>,
    pub wallet: WalletConfig,
    pub flashloan: Option<FlashloanConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BotConfig {
    pub compute_unit_limit: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingConfig {
    pub mint_config_list: Vec<MintConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintConfig {
    pub mint: String,

    #[serde(default)]
    pub raydium_pool_list: Option<Vec<String>>,
    #[serde(default)]
    pub raydium_cp_pool_list: Option<Vec<String>>,
    #[serde(default)]
    pub raydium_clmm_pool_list: Option<Vec<String>>,

    #[serde(default)]
    pub meteora_dlmm_pool_list: Option<Vec<String>>,
    #[serde(default)]
    pub meteora_damm_pool_list: Option<Vec<String>>,
    #[serde(default)]
    pub meteora_damm_v2_pool_list: Option<Vec<String>>,

    #[serde(default)]
    pub pump_pool_list: Option<Vec<String>>,

    #[serde(default)]
    pub whirlpool_pool_list: Option<Vec<String>>,

    #[serde(default)]
    pub solfi_pool_list: Option<Vec<String>>,

    #[serde(default)]
    pub vertigo_pool_list: Option<Vec<String>>,

    #[serde(default)]
    pub lookup_table_accounts: Option<Vec<String>>,
    pub process_delay: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpamConfig {
    pub enabled: bool,
    pub sending_rpc_urls: Vec<String>,
    pub compute_unit_price: u64,
    pub max_retries: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    pub private_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlashloanConfig {
    pub enabled: bool,
}

impl Config {
    /// Load configuration from multiple sources with priority:
    /// 1. Environment variables (highest priority)
    /// 2. TOML/YAML config files
    /// 3. Default values (lowest priority)
    pub fn load() -> Result<Self, ConfigError> {
        // Load environment variables from .env file
        dotenv().ok();

        let mut builder = ConfigBuilder::builder()
            // Start with default values
            .set_default("bot.compute_unit_limit", 600000)?
            .set_default("rpc.url", "https://api.mainnet-beta.solana.com")?
            .set_default("wallet.private_key", "")?;

        // Try to load from config files (TOML or YAML)
        let config_files = ["config.toml", "config.yaml", "config.yml"];

        for file in &config_files {
            if std::path::Path::new(file).exists() {
                builder = builder.add_source(File::with_name(file));
                break; // Use first found config file
            }
        }

        // Add environment variables (highest priority)
        builder = builder.add_source(
            Environment::with_prefix("BOT")
                .separator("_")
                .try_parsing(true)
        );

        let config = builder.build()?;
        config.try_deserialize()
    }
}
