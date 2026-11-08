use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Extension, Json,
};
use sqlx::QueryBuilder;
use uuid::Uuid;
use crate::state::AppState;
use crate::models::resource::*;

// 取得資源詳情
pub async fn get_resource_detail(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<ResourceDetailResponse>, StatusCode> {
    let row = sqlx::query!(
        r#"SELECT r.id, r.title, r.slug, r.category::TEXT AS category, r.description, r.cover_image_url, r.download_url, r.version, r.tags, r.views_count, r.likes_count, r.created_at, u.username AS author_username, u.avatar_url AS author_avatar_url
           FROM resources r JOIN users u ON r.author_id = u.id WHERE r.id = $1"#,
        id
    ).fetch_optional(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::NOT_FOUND)?;

    let dependencies = sqlx::query_as!(
        DependencyDto,
        r#"SELECT d.id, COALESCE(dep_res.title, d.external_name) AS "name!", CASE WHEN d.dependent_resource_id IS NULL THEN true ELSE false END AS "is_external!", COALESCE(dep_res.download_url, d.external_url) AS url, d.is_required, d.notes
           FROM resource_dependencies d LEFT JOIN resources dep_res ON d.dependent_resource_id = dep_res.id WHERE d.resource_id = $1"#,
        id
    ).fetch_all(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(ResourceDetailResponse {
        id: row.id, title: row.title, slug: row.slug, category: row.category,
        description: row.description, cover_image_url: row.cover_image_url,
        download_url: row.download_url, version: row.version, tags: row.tags,
        views_count: row.views_count, likes_count: row.likes_count, created_at: row.created_at,
        author: AuthorDto { username: row.author_username, avatar_url: row.author_avatar_url },
        dependencies,
    }))
}

// 發布資源
pub async fn publish_resource_handler(
    State(state): State<AppState>,
    Extension(user_id): Extension<Uuid>,
    Json(payload): Json<CreateResourceRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), StatusCode> {
    let id = Uuid::new_v4();
    let version = payload.version.unwrap_or_else(|| "1.0.0".to_string());

    let res = sqlx::query!(
        r#"INSERT INTO resources (id, author_id, title, slug, category, description, cover_image_url, download_url, version, tags)
           VALUES ($1, $2, $3, $4, $5::resource_category, $6, $7, $8, $9, $10) RETURNING id"#,
        id, user_id, payload.title, payload.slug, payload.category,
        payload.description, payload.cover_image_url, payload.download_url, version, &payload.tags
    ).fetch_one(&state.db).await;

    match res {
        Ok(rec) => Ok((StatusCode::CREATED, Json(serde_json::json!({ "message": "發布成功", "id": rec.id })))),
        Err(_) => Err(StatusCode::CONFLICT),
    }
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ResourceSummaryResponse {
    pub id: Uuid, pub title: String, pub slug: String, pub category: String,
    pub cover_image_url: Option<String>, pub version: String, pub tags: Vec<String>,
    pub views_count: i32, pub likes_count: i32, pub created_at: chrono::DateTime<chrono::Utc>,
    pub author_username: String,
}

// 搜尋與篩選
pub async fn search_resources_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchParams>,
) -> Result<Json<Vec<ResourceSummaryResponse>>, StatusCode> {
    let limit = params.limit.unwrap_or(20).clamp(1, 100);
    let page = params.page.unwrap_or(1).max(1);
    let offset = (page - 1) * limit;

    let mut qb: QueryBuilder<sqlx::Postgres> = QueryBuilder::new(
        r#"SELECT r.id, r.title, r.slug, r.category::TEXT AS category, r.cover_image_url, r.version, r.tags, r.views_count, r.likes_count, r.created_at, u.username AS author_username
           FROM resources r JOIN users u ON r.author_id = u.id WHERE 1=1"#
    );

    if let Some(cat) = &params.category {
        qb.push(" AND r.category = "); qb.push_bind(cat); qb.push("::resource_category");
    }
    if let Some(kw) = &params.q {
        qb.push(" AND (r.title ILIKE "); qb.push_bind(format!("%{}%", kw));
        qb.push(" OR r.description ILIKE "); qb.push_bind(format!("%{}%", kw)); qb.push(")");
    }
    if let Some(tag) = &params.tag {
        qb.push(" AND "); qb.push_bind(tag); qb.push(" = ANY(r.tags)");
    }

    qb.push(" ORDER BY r.created_at DESC LIMIT "); qb.push_bind(limit);
    qb.push(" OFFSET "); qb.push_bind(offset);

    let list = qb.build_query_as::<ResourceSummaryResponse>().fetch_all(&state.db).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(list))
}

// 收藏
pub async fn add_favorite_handler(State(state): State<AppState>, Extension(user_id): Extension<Uuid>, Path(res_id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    sqlx::query!(r#"INSERT INTO favorites (user_id, resource_id) VALUES ($1, $2) ON CONFLICT DO NOTHING"#, user_id, res_id)
        .execute(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

pub async fn remove_favorite_handler(State(state): State<AppState>, Extension(user_id): Extension<Uuid>, Path(res_id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    sqlx::query!(r#"DELETE FROM favorites WHERE user_id = $1 AND resource_id = $2"#, user_id, res_id)
        .execute(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::NO_CONTENT)
}

// 按讚
pub async fn like_resource_handler(State(state): State<AppState>, Extension(user_id): Extension<Uuid>, Path(res_id): Path<Uuid>) -> Result<StatusCode, StatusCode> {
    let mut tx = state.db.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let inserted = sqlx::query!(r#"INSERT INTO resource_likes (user_id, resource_id) VALUES ($1, $2) ON CONFLICT DO NOTHING RETURNING resource_id"#, user_id, res_id)
        .fetch_optional(&mut *tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if inserted.is_some() {
        sqlx::query!(r#"UPDATE resources SET likes_count = likes_count + 1 WHERE id = $1"#, res_id)
            .execute(&mut *tx).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }
    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct RateRequest { pub rating: i16, pub comment: Option<String> }

// 評分
pub async fn rate_resource_handler(State(state): State<AppState>, Extension(user_id): Extension<Uuid>, Path(res_id): Path<Uuid>, Json(payload): Json<RateRequest>) -> Result<StatusCode, StatusCode> {
    if payload.rating < 1 || payload.rating > 5 { return Err(StatusCode::BAD_REQUEST); }
    sqlx::query!(
        r#"INSERT INTO resource_ratings (user_id, resource_id, rating, comment) VALUES ($1, $2, $3, $4)
           ON CONFLICT (user_id, resource_id) DO UPDATE SET rating = EXCLUDED.rating, comment = EXCLUDED.comment, updated_at = NOW()"#,
        user_id, res_id, payload.rating, payload.comment
    ).execute(&state.db).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}
