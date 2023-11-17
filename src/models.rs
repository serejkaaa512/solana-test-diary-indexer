use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct LastProcessedSignature {
    pub signature: Vec<u8>,
    pub block_time: Option<i64>,
}

#[derive(Serialize, Deserialize)]
pub struct Diary {
    pub account: String,
    pub user_address: String,
    pub name: String,
    pub signature: Vec<u8>,
    pub raw_transaction: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
pub struct Record {
    pub account: String,
    pub diary: String,
    pub text: String,
    pub signature: Vec<u8>,
    pub raw_transaction: Vec<u8>,
}
