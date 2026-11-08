use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
    Json,
};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, PasswordVerifier, PasswordHash, SaltString},
    Argon2,
};
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret,
    CsrfToken, RedirectUrl, Scope, TokenResponse, TokenUrl,
};
use serde::{Deserialize, Serialize};
use std::env;
use uuid::Uuid;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub message: String,
    pub user_id: Uuid,
}

// 註冊
pub async fn register_handler(
    State(state): State<AppState>,
    Json(payload): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<AuthResponse>), StatusCode> {
    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(payload.password.as_bytes(), &salt)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .to_string();

    let new_id = Uuid::new_v4();
    let result = sqlx::query!(
        r#"INSERT INTO users (id, username, email, password_hash) VALUES ($1, $2, $3, $4) RETURNING id"#,
        new_id, payload.username, payload.email, password_hash
    )
    .fetch_one(&state.db)
    .await;

    match result {
        Ok(rec) => Ok((StatusCode::CREATED, Json(AuthResponse { message: "註冊成功".to_string(), user_id: rec.id }))),
        Err(_) => Err(StatusCode::CONFLICT),
    }
}

// 登入
pub async fn login_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let user = sqlx::query!(r#"SELECT id, password_hash FROM users WHERE email = $1"#, payload.email)
        .fetch_optional(&state.db)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let stored_hash = user.password_hash.ok_or(StatusCode::UNAUTHORIZED)?;
    let parsed_hash = PasswordHash::new(&stored_hash).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    
    let is_valid = Argon2::default().verify_password(payload.password.as_bytes(), &parsed_hash).is_ok();
    if !is_valid { return Err(StatusCode::UNAUTHORIZED); }

    Ok(Json(AuthResponse { message: "登入成功".to_string(), user_id: user.id }))
}

// Google OAuth 導向
pub async fn google_login_handler() -> Result<Redirect, StatusCode> {
    let client = google_oauth_client().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let (auth_url, _) = client.authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.email".to_string()))
        .add_scope(Scope::new("https://www.googleapis.com/auth/userinfo.profile".to_string()))
        .url();
    Ok(Redirect::to(auth_url.as_str()))
}

#[derive(Deserialize)]
pub struct AuthQuery { code: String }
#[derive(Deserialize)]
struct GoogleUser { id: String, email: String, name: String, picture: Option<String> }

fn google_oauth_client() -> Result<BasicClient, String> {
    let cid = env::var("GOOGLE_CLIENT_ID").map_err(|_| "")?;
    let csec = env::var("GOOGLE_CLIENT_SECRET").map_err(|_| "")?;
    let rurl = env::var("GOOGLE_REDIRECT_URL").map_err(|_| "")?;
    Ok(BasicClient::new(
        ClientId::new(cid), Some(ClientSecret::new(csec)),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string()).unwrap(),
        Some(TokenUrl::new("https://oauth2.googleapis.com/token".to_string()).unwrap())
    ).set_redirect_uri(RedirectUrl::new(rurl).unwrap()))
}

// Google Callback
pub async fn google_callback_handler(
    State(state): State<AppState>, Query(query): Query<AuthQuery>
) -> Result<Json<serde_json::Value>, StatusCode> {
    let client = google_oauth_client().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let token = client.exchange_code(AuthorizationCode::new(query.code))
        .request_async(oauth2::reqwest::async_http_client).await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    let g_user = reqwest::Client::new().get("https://www.googleapis.com/oauth2/v2/userinfo")
        .bearer_auth(token.access_token().secret()).send().await.unwrap()
        .json::<GoogleUser>().await.unwrap();

    let mut tx = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let existing = sqlx::query!(r#"SELECT user_id FROM oauth_accounts WHERE provider = 'google' AND provider_user_id = $1"#, g_user.id)
        .fetch_optional(&mut *tx).await.unwrap();

    let uid = match existing {
        Some(rec) => rec.user_id,
        None => {
            let u = sqlx::query!(r#"SELECT id FROM users WHERE email = $1"#, g_user.email).fetch_optional(&mut *tx).await.unwrap();
            let id = match u { Some(rec) => rec.id, None => {
                let new_id = Uuid::new_v4();
                sqlx::query!(r#"INSERT INTO users (id, username, email, avatar_url) VALUES ($1, $2, $3, $4)"#, new_id, g_user.name, g_user.email, g_user.picture)
                    .execute(&mut *tx).await.unwrap();
                new_id
            }};
            sqlx::query!(r#"INSERT INTO oauth_accounts (user_id, provider, provider_user_id) VALUES ($1, 'google', $2)"#, id, g_user.id)
                .execute(&mut *tx).await.unwrap();
            id
        }
    };
    tx.commit().await.unwrap();

    Ok(Json(serde_json::json!({ "message": "Google 登入成功", "user_id": uid })))
}
