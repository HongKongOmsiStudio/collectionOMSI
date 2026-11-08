use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, sqlx::FromRow)]
pub struct DependencyDto {
    pub id: Uuid,
    pub name: String,
    pub is_external: bool,
    pub url: Option<String>,
    pub is_required: bool,
    pub notes: Option<String>,
}

#[derive(Serialize)]
pub struct AuthorDto {
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Serialize)]
pub struct ResourceDetailResponse {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub category: String,
    pub description: String,
    pub cover_image_url: Option<String>,
    pub download_url: String,
    pub version: String,
    pub tags: Vec<String>,
    pub views_count: i32,
    pub likes_count: i32,
    pub created_at: DateTime<Utc>,
    pub author: AuthorDto,
    pub dependencies: Vec<DependencyDto>,
}

#[derive(Deserialize)]
pub struct CreateResourceRequest {
    pub title: String,
    pub slug: String,
    pub category: String,
    pub description: String,
    pub cover_image_url: Option<String>,
    pub download_url: String,
    pub version: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}
