mod state;
mod error;
mod utils;

use axum::{routing::{get, post}, Router};
use std::{net::SocketAddr, time::Duration};
use tokio::signal;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tower::limit::RateLimitLayer;
use http::{Method, HeaderValue};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use state::{init_db_pool, AppState};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info,tower_http=debug".into()))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let db_pool = match init_db_pool().await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("❌ 初始化資料庫連線池失敗: {:?}", e);
            std::process::exit(1);
        }
    };

    let app_state = AppState { db: db_pool };

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PUT, Method::DELETE])
        .allow_headers(tower_http::cors::Any)
        .allow_credentials(true);

    let rate_limiter = RateLimitLayer::new(20, Duration::from_secs(1));

    let app = Router::new()
        .route("/health", get(health_check_handler))
        .with_state(app_state.clone())
        .layer(rate_limiter)
        .layer(cors)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    println!("🚀 伺服器運行中：http://{}", addr);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(app_state))
        .await?;

    Ok(())
}

async fn health_check_handler(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> &'static str {
    match sqlx::query("SELECT 1").execute(&state.db).await {
        Ok(_) => "Database is healthy!",
        Err(_) => "Database connection error!",
    }
}

async fn shutdown_signal(state: AppState) {
    let ctrl_c = async {
        signal::ctrl_c().await.expect("無法安裝 Ctrl+C 訊號處理器");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("無法安裝 SIGTERM 訊號處理器")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    println!("\n⚠️ 收到關閉訊號，正在執行優雅關閉...");
    tokio::time::sleep(Duration::from_secs(2)).await;
    state.db.close().await;
    println!("✅ 資料庫連線已安全關閉。");
}
