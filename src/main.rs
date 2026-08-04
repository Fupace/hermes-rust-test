use axum::{routing::get, Router};
use sqlx::postgres::PgPoolOptions;
use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, services::ServeDir};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod api;
mod db;
mod models;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hermes_k8s_platform=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:***@localhost:5432/k8s_platform".into());

    let pool = PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    db::schema::run_migrations(&pool).await;

    let app_state = api::AppState { pool: pool.clone() };

    let app = Router::new()
        .route("/health", get(api::handlers::health_check))
        .route("/api/cluster/summary", get(api::handlers::cluster_summary))
        .route("/api/pods", get(api::handlers::list_pods))
        .route("/api/deployments", get(api::handlers::list_deployments))
        .route("/api/services", get(api::handlers::list_services))
        .route("/api/namespaces", get(api::handlers::list_namespaces))
        .route(
            "/api/namespaces/{ns}/pods/{pod}/logs",
            get(api::handlers::pod_logs),
        )
        .nest_service("/", ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap_or(8080);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Hermes K8s Platform listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
