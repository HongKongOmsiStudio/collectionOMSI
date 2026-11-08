mod state;
mod models;
mod handlers;

use axum::{
    routing::{get, post, delete},
    Router,
};
use sqlx::PgPool;
use std::net::SocketAddr;
use state::AppState;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("必須設定 DATABASE_URL");

    let db_pool = PgPool::connect(&database_url).await?;
    println!("📦 成功連線至 PostgreSQL 資料庫！");

    let app_state = AppState { db: db_pool };

    // 公開路由
    let public_routes = Router::new()
        .route("/auth/register", post(handlers::auth::register_handler))
        .route("/auth/login", post(handlers::auth::login_handler))
        .route("/auth/google", get(handlers::auth::google_login_handler))
        .route("/auth/google/callback", get(handlers::auth::google_callback_handler))
        .route("/resources", get(handlers::resource::search_resources_handler))
        .route("/resources/:id", get(handlers::resource::get_resource_detail));

    // 受保護路由 (模擬 auth_middleware，實務上可加上認證 Layer)
    let protected_routes = Router::new()
        .route("/resources", post(handlers::resource::publish_resource_handler))
        .route("/resources/:id/favorite", post(handlers::resource::add_favorite_handler).delete(handlers::resource::remove_favorite_handler))
        .route("/resources/:id/like", post(handlers::resource::like_resource_handler))
        .route("/resources/:id/rate", post(handlers::resource::rate_resource_handler));

    let app = Router::new()
        .nest("/api/v1", public_routes.merge(protected_routes))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 OMSI 2 平台後端已啟動於 http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
