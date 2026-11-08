use axum::Router;
use sqlx::PgPool;
use std::env;

pub async fn setup_test_app() -> (Router, PgPool) {
    dotenvy::dotenv().ok();
    let db_url = env::var("TEST_DATABASE_URL")
        .unwrap_or_else(|_| env::var("DATABASE_URL").expect("必須設定 DATABASE_URL"));

    let pool = PgPool::connect(&db_url).await.expect("無法連線至測試資料庫");
    let app = omsi2_platform_backend::create_app(pool.clone());
    (app, pool)
}
