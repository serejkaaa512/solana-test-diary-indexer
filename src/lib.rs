use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use sqlx::postgres::PgPoolOptions;

use self::sqlx_client::*;
use crate::indexer::DiaryIndexer;
use crate::settings::Config;

mod indexer;
mod models;
mod settings;
mod sqlx_client;

pub async fn start_service() -> Result<()> {
    let config: Config = Config::default();

    let db_pool = PgPoolOptions::new()
        .max_connections(config.db_pool_size)
        .connect(&config.database_url)
        .await
        .expect("Failed to create pg pool");

    sqlx::migrate!().run(&db_pool).await?;

    let sqlx_client = SqlxClient::new(db_pool.clone());

    let poll_interval = config.poll_interval_sec;
    let diary_indexer = Arc::new(
        DiaryIndexer::new(config, sqlx_client)
            .await
            .context("Failed to create diary indexer")?,
    );

    tokio::spawn(async move {
        loop {
            if let Err(e) = diary_indexer.update().await {
                tracing::error!("error occurred during indexer update: {e:?}");
            }

            tokio::time::sleep(Duration::from_secs(poll_interval)).await;
        }
    });

    futures::future::pending().await
}
