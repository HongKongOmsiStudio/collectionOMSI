use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use sqlx::PgPool;
use std::net::SocketAddr;

// 假設你的 AppState 包含資料庫連線池
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. 載入 .env 檔案中的環境變數 (例如 DATABASE_URL)
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")
        .expect("必須在環境變數中設定 DATABASE_URL");

    // 2. 建立 PostgreSQL 資料庫連線池 (Connection Pool)
    let pool = PgPool::connect(&database_url).await.map_err(|e| {
        eprintln!("無法連線至資料庫: {}", e);
        e
    })?;
    
    println!("📦 成功連線至 PostgreSQL 資料庫！");

    // 3. 建立 Axum Router 並綁定路由與 State
    let app = Router::new()
        .route("/api/v1/resources/:id", get(get_resource_detail))
        .with_state(pool); // 將 PgPool 直接作為共用 State 注入所有 Handler

    // 4. 設定伺服器監聽位址 (例如 127.0.0.1:8080)
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 OMSI 2 後端伺服器已啟動，正在聆聽 http://{}", addr);

    // 5. 使用 tokio 及 axum::serve 啟動服務
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
