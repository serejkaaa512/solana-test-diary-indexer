use anyhow::Result;

use crate::models::Diary;
use crate::sqlx_client::*;

impl SqlxClient {
    pub async fn insert_diary(&self, diary: Diary) -> Result<()> {
        sqlx::query!(
            r#"INSERT INTO diaries
            (account, user_address, name, signature, raw_transaction)
            VALUES ($1, $2, $3, $4, $5)
            ON CONFLICT (account) DO NOTHING"#,
            diary.account,
            diary.user_address,
            diary.name,
            diary.signature,
            diary.raw_transaction,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
