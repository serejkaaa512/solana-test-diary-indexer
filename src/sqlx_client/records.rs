use anyhow::Result;

use crate::models::*;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn insert_record(&self, record: Record) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO records
            (account, diary, text, signature, raw_transaction)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (account) DO UPDATE SET text = $3"#,
            record.account,
            record.diary,
            record.text,
            record.signature,
            record.raw_transaction,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_record(&self, account: String) -> Result<()> {
        sqlx::query_as!(Record, r#"DELETE FROM records WHERE account = $1"#, account,)
            .execute(&self.pool)
            .await?;

        Ok(())
    }
}
