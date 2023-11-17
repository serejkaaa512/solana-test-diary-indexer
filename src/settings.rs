use serde::Deserialize;
use solana_program::pubkey::Pubkey;
use std::str::FromStr;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub database_url: String,
    pub db_pool_size: u32,
    pub rpc_endpoint: String,
    pub poll_interval_sec: u64,
    pub signature_limit: usize,
    pub program_address: Pubkey,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            database_url: "postgresql://myusername:mypassword@localhost:5432/solana_diary_indexer"
                .to_string(),
            db_pool_size: 2,
            rpc_endpoint: "https://api.mainnet-beta.solana.com".to_string(),
            poll_interval_sec: 60,
            signature_limit: 10,
            program_address: Pubkey::from_str("bNFMSsTXGZxhAA7mUcdUid5Yir3zWJf1myfP4TSQ46x")
                .unwrap(),
        }
    }
}
