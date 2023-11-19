use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use borsh::BorshDeserialize;
use diary::instruction::{AddRecord, CreateDiary, RemoveRecord};
use diary::{DiaryEvent, RecordEvent};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_client::rpc_client::GetConfirmedSignaturesForAddress2Config;
use solana_client::rpc_config::RpcTransactionConfig;
use solana_program::program_pack::Pack;
use solana_sdk::account::ReadableAccount;
use solana_sdk::signature::Signature;
use solana_transaction_status::option_serializer::OptionSerializer;
use solana_transaction_status::{EncodedTransaction, UiTransactionEncoding};

use crate::models::{Diary, Record};
use crate::settings;
use crate::sqlx_client::SqlxClient;

pub struct DiaryIndexer {
    config: settings::Config,
    rpc_client: Arc<RpcClient>,
    sqlx_client: SqlxClient,
}

impl DiaryIndexer {
    pub async fn new(config: settings::Config, sqlx_client: SqlxClient) -> Result<Self> {
        let rpc_client = Arc::new(RpcClient::new_with_timeout(
            config.rpc_endpoint.clone(),
            Duration::from_secs(60),
        ));

        Ok(Self {
            config,
            rpc_client,
            sqlx_client,
        })
    }

    pub async fn update(&self) -> Result<()> {
        let mut signatures = Vec::new();

        let mut before = None;
        let until = match self.sqlx_client.get_latest_processed_signature().await? {
            Some(x) => Some(Signature::from_str(&String::from_utf8(
                x.signature.to_vec(),
            )?)?),
            None => None,
        };

        loop {
            let limit = self.config.signature_limit;
            let config = GetConfirmedSignaturesForAddress2Config {
                before,
                until,
                limit: Some(limit),
                ..Default::default()
            };

            let sigs = self
                .rpc_client
                .get_signatures_for_address_with_config(&self.config.program_address, config)
                .await?;

            let oldest_signature = match sigs.last() {
                Some(oldest_signature) => oldest_signature,
                None => break,
            };

            if sigs.len() == limit {
                let oldest_signature = Signature::from_str(&oldest_signature.signature)?;
                before = Some(oldest_signature);
                signatures.extend(sigs);
            } else {
                signatures.extend(sigs);
                break;
            }
        }

        for transaction_status in signatures.into_iter().rev() {
            if transaction_status.err.is_some() {
                // Skip failed transactions
                continue;
            }

            let signature = Signature::from_str(&transaction_status.signature)?;
            let config = RpcTransactionConfig {
                encoding: Some(UiTransactionEncoding::Base64),
                ..Default::default()
            };
            let transaction = self
                .rpc_client
                .get_transaction_with_config(&signature, config)
                .await?;

            let events = transaction
                .transaction
                .meta
                .and_then(|t| {
                    if let OptionSerializer::Some(log_messages) = t.log_messages {
                        Some(get_events_from_log(
                            &mut log_messages.into_iter(),
                            &self.config.program_address.to_string(),
                        ))
                    } else {
                        None
                    }
                })
                .unwrap_or_default();

            let raw_transaction =
                if let EncodedTransaction::Binary(ref r, _) = transaction.transaction.transaction {
                    r.as_bytes().to_vec()
                } else {
                    return Err(
                        DiaryIndexerError::DecodeTransactionError(signature.to_string()).into(),
                    );
                };

            let encoded_transaction = transaction
                .transaction
                .transaction
                .decode()
                .ok_or_else(|| DiaryIndexerError::DecodeTransactionError(signature.to_string()))?;

            for c_ix in encoded_transaction.message.instructions() {
                if let Ok(CreateDiary { name, .. }) = CreateDiary::try_from_slice(&c_ix.data) {
                    let accounts = c_ix.accounts.clone();
                    let signature = transaction_status.signature.as_bytes().to_vec();
                    let account_keys = encoded_transaction.message.static_account_keys();

                    let user_pubkey = account_keys[accounts[0] as usize];
                    let diary_pubkey = account_keys[accounts[1] as usize];

                    let name = events
                        .iter()
                        .find_map(|event| {
                            DiaryEvent::try_from_slice(&base64::decode(event).unwrap_or_default())
                                .ok()
                        })
                        .map(|DiaryEvent { name, .. }| name)
                        .unwrap_or(name);

                    let diary = Diary {
                        account: diary_pubkey.to_string(),
                        user_address: user_pubkey.to_string(),
                        signature,
                        name,
                        raw_transaction: raw_transaction.clone(),
                    };

                    self.sqlx_client
                        .insert_diary(diary)
                        .await
                        .with_context(|| {
                            format!("Failed to insert diary `{}` in db", diary_pubkey)
                        })?;
                } else if let Ok(AddRecord { .. }) = AddRecord::try_from_slice(&c_ix.data) {
                    let accounts = c_ix.accounts.clone();
                    let signature = transaction_status.signature.as_bytes().to_vec();
                    let account_keys = encoded_transaction.message.static_account_keys();

                    let diary_pubkey = account_keys[accounts[1] as usize];
                    let record_pubkey = account_keys[accounts[2] as usize];

                    let text_from_event = events
                        .iter()
                        .find_map(|event| {
                            RecordEvent::try_from_slice(&base64::decode(event).unwrap_or_default())
                                .ok()
                        })
                        .map(|RecordEvent { text }| text);

                    let text = if let Some(text_from_event) = text_from_event {
                        text_from_event
                    } else {
                        let record_account = self
                            .rpc_client
                            .get_account_with_commitment(&record_pubkey, Default::default())
                            .await?
                            .value
                            .ok_or(DiaryIndexerError::AccountNotFound(
                                record_pubkey.to_string(),
                            ))?;
                        let record_account_data = diary::Record::unpack(record_account.data())?;
                        record_account_data.text
                    };

                    let record = Record {
                        account: record_pubkey.to_string(),
                        diary: diary_pubkey.to_string(),
                        text,
                        signature,
                        raw_transaction: raw_transaction.clone(),
                    };

                    self.sqlx_client
                        .insert_record(record)
                        .await
                        .with_context(|| {
                            format!("Failed to insert record{}` in db", record_pubkey)
                        })?;
                } else if let Ok(RemoveRecord { .. }) = RemoveRecord::try_from_slice(&c_ix.data) {
                    let accounts = c_ix.accounts.clone();
                    let account_keys = encoded_transaction.message.static_account_keys();
                    let record_pubkey = account_keys[accounts[2] as usize];

                    self.sqlx_client
                        .delete_record(record_pubkey.to_string())
                        .await
                        .with_context(|| {
                            format!("Failed to delete record {}` in db", record_pubkey)
                        })?;
                }
            }

            // Insert processed signature
            self.sqlx_client
                .insert_processed_signatures(
                    transaction_status.signature.as_bytes(),
                    transaction_status.block_time,
                )
                .await?;
        }

        Ok(())
    }
}

fn get_events_from_log(
    logs: &mut (dyn Iterator<Item = String> + 'static),
    program: &str,
) -> Vec<String> {
    let mut result = vec![];
    get_events_raw(None, logs, program, &mut result);
    result
}

fn get_events_raw(
    current_program: Option<&str>,
    logs: &mut (dyn Iterator<Item = String> + 'static),
    program: &str,
    result: &mut Vec<String>,
) {
    if let Some(current_program) = current_program {
        if current_program != program {
            while let Some(log) = logs.next() {
                if log.starts_with(&format!("Program {} success", current_program))
                    || log.starts_with(&format!("Program {} failed", current_program))
                {
                    return;
                }
            }
        } else {
            while let Some(log) = logs.next() {
                if log.starts_with(&format!("Program {} success", program))
                    || log.starts_with(&format!("Program {} failed", program))
                {
                    break;
                }
                let mut split = log.split_whitespace();
                if let Some(f) = split.next() {
                    if f == "Program" {
                        if let Some(t) = split.next() {
                            if t != "data:" && t != "log:" && t.len() == 43 {
                                if let Some(tt) = split.next() {
                                    if tt == "invoke" {
                                        get_events_raw(Some(t), logs, program, result);
                                    }
                                }
                            }
                        }
                    }
                }
                if log.starts_with("Program data: ") {
                    if let Some(data) = log.split_whitespace().rev().next() {
                        result.push(data.to_string())
                    }
                }
            }
        }
    } else {
        while let Some(log) = logs.next() {
            if log.starts_with(&format!("Program {} invoke", program)) {
                get_events_raw(Some(program), logs, program, result);
            }
        }
    }
}

#[derive(thiserror::Error, Debug)]
enum DiaryIndexerError {
    #[error("Account `{0}` not found")]
    AccountNotFound(String),
    #[error("Failed to decode solana transaction: `{0}`")]
    DecodeTransactionError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_test() {
        let log = vec![
            "Program bNFMSsTXGZxhAA7mUcdUid5Yir3zWJf1myfP4TSQ46x invoke [1]".to_string(),
            "Program log: Instruction: AddRecord".to_string(),
            "Program log: Program".to_string(),
            "Program data: k8iH+OXeD5YPAAAAAAAAAAAAAAAAAAAAMTIzBgAAAGZkc2Zkcw==".to_string(),
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA invoke [2]".to_string(),
            "Program log: Instruction: Transfer".to_string(),
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA consumed 4736 of 100973 compute units".to_string(),
            "Program TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA success".to_string(),
            "Program data: A8iH+OXeD5YPAAAAAAAAAAAAAAAAAAAAMTIzBgAAAGZkc2Zkcw==".to_string(),
            "Program bNFMSsTXGZxhAA7mUcdUid5Yir3zWJf1myfP4TSQ46x consumed 8120 of 200000 compute units".to_string(),
            "Program bNFMSsTXGZxhAA7mUcdUid5Yir3zWJf1myfP4TSQ46x success".to_string(),
            ];
        let events = get_events_from_log(
            &mut log.into_iter(),
            "bNFMSsTXGZxhAA7mUcdUid5Yir3zWJf1myfP4TSQ46x",
        );
        assert_eq!(
            events,
            vec![
                "k8iH+OXeD5YPAAAAAAAAAAAAAAAAAAAAMTIzBgAAAGZkc2Zkcw==".to_string(),
                "A8iH+OXeD5YPAAAAAAAAAAAAAAAAAAAAMTIzBgAAAGZkc2Zkcw==".to_string()
            ]
        );
    }
}
