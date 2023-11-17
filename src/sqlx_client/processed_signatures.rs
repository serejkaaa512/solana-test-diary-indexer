use anyhow::{Context, Result};

use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn get_latest_processed_signature(&self) -> Result<Option<LastProcessedSignature>> {
        sqlx::query_as!(
            LastProcessedSignature,
            r#"SELECT signature, block_time
               FROM processed_signatures
               WHERE block_time = (
                    SELECT MAX (block_time)
                    FROM processed_signatures
               )"#,
        )
        .fetch_optional(&self.pool)
        .await
        .context("Failed to get last processed signature")
    }

    pub async fn insert_processed_signatures(
        &self,
        signature: &[u8],
        block_time: Option<i64>,
    ) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO processed_signatures
                (signature, block_time)
                VALUES ($1, $2)"#,
            signature,
            block_time,
        )
        .execute(&self.pool)
        .await
        .context("Failed to insert last processed signature")?;

        Ok(())
    }
}
