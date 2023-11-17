use sqlx::PgPool;

mod diaries;
mod processed_signatures;
mod records;

#[derive(Clone)]
pub struct SqlxClient {
    pool: PgPool,
}
impl SqlxClient {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}
