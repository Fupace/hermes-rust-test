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

    let state = api::AppState { pool };

    let app = Router::new()
        .route("/health", get(api::handlers::health_check))
        .route(
            "/api/clusters",
            get(api::handlers::list_clusters).post(api::handlers::add_cluster),
        )
        .route(
            "/api/clusters/{id}",
            get(api::handlers::get_cluster).delete(api::handlers::delete_cluster),
        )
        .route(
            "/api/clusters/{id}/summary",
            get(api::handlers::cluster_summary),
        )
        .route("/api/clusters/{id}/pods", get(api::handlers::list_pods))
        .route(
            "/api/clusters/{id}/deployments",
            get(api::handlers::list_deployments).post(api::handlers::create_deployment),
        )
        .route(
            "/api/clusters/{id}/services",
            get(api::handlers::list_services),
        )
        .route("/api/clusters/{id}/nodes", get(api::handlers::list_nodes))
        .route("/api/clusters/{id}/pvcs", get(api::handlers::list_pvcs))
        .route(
            "/api/clusters/{id}/configmaps",
            get(api::handlers::list_configmaps),
        )
        .route(
            "/api/clusters/{id}/secrets",
            get(api::handlers::list_secrets),
        )
        .route(
            "/api/clusters/{id}/namespaces",
            get(api::handlers::list_namespaces),
        )
        .route(
            "/api/clusters/{id}/namespaces/{ns}/pods/{pod}/logs",
            get(api::handlers::pod_logs),
        )
        .route(
            "/api/clusters/{id}/namespaces/{ns}/pods/{pod}/yaml",
            get(api::handlers::pod_yaml),
        )
        .route(
            "/api/clusters/{id}/namespaces/{ns}/deployments/{name}/yaml",
            get(api::handlers::deployment_yaml),
        )
        .nest_service("/", ServeDir::new("static"))
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port: u16 = std::env::var("PORT")
        .unwrap_or_else(|_| "8080".into())
        .parse()
        .unwrap_or(8080);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    tracing::info!("Hermes K8s Platform v0.2 listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}
