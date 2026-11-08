use sqlx::{PgPool, Error as SqlxError};
use std::time::Duration;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
}

pub async fn init_db_pool() -> Result<PgPool, SqlxError> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("必須在 .env 檔案中設定 DATABASE_URL 變數");

    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(25)
        .min_connections(5)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .connect(&database_url)
        .await?;

    println!("✅ PostgreSQL 資料庫連線池建立成功！");
    Ok(pool)
}
