//! Procedural macros for boilerplate reduction

/// Macro to generate DEX implementations with common boilerplate
#[macro_export]
macro_rules! impl_dex {
    ($struct_name:ident, $name:expr, $program_id:expr) => {
        impl $struct_name {
            pub fn new(rpc_client: std::sync::Arc<solana_client::rpc_client::RpcClient>) -> Self {
                Self { rpc_client }
            }
        }

        impl crate::dex::traits::Dex for $struct_name {
            fn name(&self) -> &'static str {
                $name
            }

            fn program_id(&self) -> solana_sdk::pubkey::Pubkey {
                $program_id
            }
        }
    };
}

/// Macro to generate pool parsing boilerplate
#[macro_export]
macro_rules! parse_pool_account {
    ($account_data:expr, $pool_struct:ty, $offset_map:expr) => {{
        use std::collections::HashMap;
        let mut pool_info = <$pool_struct>::default();

        for (field_name, offset) in $offset_map.iter() {
            match *field_name {
                "token_mint" => {
                    pool_info.token_mint = solana_sdk::pubkey::Pubkey::new_from_array(
                        $account_data[*offset..*offset + 32].try_into().unwrap()
                    );
                }
                "base_mint" => {
                    pool_info.base_mint = solana_sdk::pubkey::Pubkey::new_from_array(
                        $account_data[*offset..*offset + 32].try_into().unwrap()
                    );
                }
                "token_vault" => {
                    pool_info.token_vault = solana_sdk::pubkey::Pubkey::new_from_array(
                        $account_data[*offset..*offset + 32].try_into().unwrap()
                    );
                }
                "base_vault" => {
                    pool_info.base_vault = solana_sdk::pubkey::Pubkey::new_from_array(
                        $account_data[*offset..*offset + 32].try_into().unwrap()
                    );
                }
                _ => {}
            }
        }

        pool_info
    }};
}

/// Macro to generate common error handling patterns
#[macro_export]
macro_rules! handle_dex_error {
    ($result:expr, $dex_name:expr, $operation:expr) => {
        match $result {
            Ok(value) => Ok(value),
            Err(e) => {
                tracing::error!("{} {} failed: {}", $dex_name, $operation, e);
                Err(crate::error::BotError::Dex(format!("{} {} error: {}", $dex_name, $operation, e)))
            }
        }
    };
}

/// Macro to generate retry logic for RPC calls
#[macro_export]
macro_rules! retry_rpc_call {
    ($rpc_client:expr, $call:expr, $max_retries:expr, $delay_ms:expr) => {{
        let mut last_error = None;
        for attempt in 0..$max_retries {
            match $call {
                Ok(result) => return Ok(result),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < $max_retries - 1 {
                        tracing::warn!("RPC call failed (attempt {}/{}), retrying in {}ms", attempt + 1, $max_retries, $delay_ms);
                        tokio::time::sleep(tokio::time::Duration::from_millis($delay_ms)).await;
                    }
                }
            }
        }
        Err(crate::error::BotError::Rpc(format!("RPC call failed after {} attempts: {:?}", $max_retries, last_error)))
    }};
}
