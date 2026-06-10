//! HTTP API (Axum). Public health probe + auth endpoints, and an
//! authenticated `/api/v1` surface guarded by the `require_auth` middleware.
//!
//! File layout:
//! - [`dto`] — request / response types
//! - [`handlers`] — one function per endpoint
//! - [`extract`] — `AuthContext` request extractor

pub mod dto;
pub mod extract;
pub mod handlers;

use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tower_http::trace::TraceLayer;

use crate::auth::middleware::require_auth;
use crate::config::Config;
use crate::storage::AppState;

/// Build the Axum router.
pub fn router(state: AppState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Authenticated routes.
    let protected = Router::new()
        .route("/auth/whoami", get(handlers::whoami))
        .route(
            "/auth/keys",
            get(handlers::list_keys).post(handlers::create_key),
        )
        .route("/auth/keys/:id", delete(handlers::revoke_key))
        .route_layer(axum::middleware::from_fn_with_state(
            state.clone(),
            require_auth,
        ));

    // Public auth routes (no middleware).
    let public_auth = Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login));

    let api_v1 = public_auth.merge(protected);

    Router::new()
        .route("/", get(handlers::root))
        .route("/healthz", get(handlers::healthz))
        .nest("/api/v1", api_v1)
        .fallback(not_found)
        .layer(TraceLayer::new_for_http())
        .layer(cors)
        .layer(axum::extract::DefaultBodyLimit::max(
            crate::api::dto::MAX_BODY_BYTES,
        ))
        .with_state(state)
}

/// Run the HTTP server on the configured host:port.
pub async fn serve(state: AppState, config: &Config) -> std::io::Result<()> {
    let app = router(state);
    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(addr = %addr, "lineagent api listening");
    axum::serve(listener, app).await
}

async fn not_found() -> axum::response::Response {
    use axum::http::StatusCode;
    use axum::response::IntoResponse;
    (
        StatusCode::NOT_FOUND,
        axum::Json(serde_json::json!({
            "error": { "code": "not_found", "message": "route not found" }
        })),
    )
        .into_response()
}
