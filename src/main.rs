use anyhow::Result;

use diary_lib::start_service;

#[tokio::main(worker_threads = 8)]
async fn main() -> Result<()> {
    start_service().await
}
