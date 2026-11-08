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

pub fn create_app(db_pool: PgPool) -> Router {
    let app_state = AppState { db: db_pool };

    let public_routes = Router::new()
        .route("/api/v1/auth/register", post(handlers::auth::register_handler))
        .route("/api/v1/auth/login", post(handlers::auth::login_handler))
        .route("/api/v1/auth/google", get(handlers::auth::google_login_handler))
        .route("/api/v1/auth/google/callback", get(handlers::auth::google_callback_handler))
        .route("/api/v1/resources", get(handlers::resource::search_resources_handler))
        .route("/api/v1/resources/:id", get(handlers::resource::get_resource_detail));

    let protected_routes = Router::new()
        .route("/api/v1/resources", post(handlers::resource::publish_resource_handler))
        .route("/api/v1/resources/:id/favorite", post(handlers::resource::add_favorite_handler).delete(handlers::resource::remove_favorite_handler))
        .route("/api/v1/resources/:id/like", post(handlers::resource::like_resource_handler))
        .route("/api/v1/resources/:id/rate", post(handlers::resource::rate_resource_handler));

    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .with_state(app_state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL").expect("必須設定 DATABASE_URL");

    let db_pool = PgPool::connect(&database_url).await?;
    println!("📦 成功連線至 PostgreSQL 資料庫！");

    let app = create_app(db_pool);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("🚀 OMSI 2 平台後端已啟動於 http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
