use crate::api::{k8s, AppState};
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct PodQuery {
    namespace: Option<String>,
}

#[derive(Deserialize)]
pub struct DeployQuery {
    namespace: Option<String>,
}

#[derive(Deserialize)]
pub struct ServiceListQuery {
    namespace: Option<String>,
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "hermes-k8s-platform",
        "version": "0.1.0"
    }))
}

pub async fn cluster_summary(State(_state): State<AppState>) -> Json<serde_json::Value> {
    match k8s::get_cluster_summary().await {
        Ok(summary) => Json(serde_json::json!({"success": true, "data": summary})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e})),
    }
}

pub async fn list_pods(
    State(_state): State<AppState>,
    Query(query): Query<PodQuery>,
) -> Json<serde_json::Value> {
    let ns = query.namespace.as_deref().filter(|s| !s.is_empty());
    match k8s::list_pods_handler(ns).await {
        Ok(pods) => Json(serde_json::json!({"success": true, "data": pods})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e})),
    }
}

pub async fn list_deployments(
    State(_state): State<AppState>,
    Query(query): Query<DeployQuery>,
) -> Json<serde_json::Value> {
    let ns = query.namespace.as_deref().filter(|s| !s.is_empty());
    match k8s::list_deployments_handler(ns).await {
        Ok(deployments) => Json(serde_json::json!({"success": true, "data": deployments})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e})),
    }
}

pub async fn list_services(
    State(_state): State<AppState>,
    Query(query): Query<ServiceListQuery>,
) -> Json<serde_json::Value> {
    let ns = query.namespace.as_deref().filter(|s| !s.is_empty());
    match k8s::list_services_handler(ns).await {
        Ok(services) => Json(serde_json::json!({"success": true, "data": services})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e})),
    }
}

pub async fn list_namespaces(State(_state): State<AppState>) -> Json<serde_json::Value> {
    match k8s::list_namespaces_handler().await {
        Ok(ns) => Json(serde_json::json!({"success": true, "data": ns})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e})),
    }
}

pub async fn pod_logs(
    State(_state): State<AppState>,
    Path((ns, pod)): Path<(String, String)>,
) -> Json<serde_json::Value> {
    match k8s::get_pod_logs(&ns, &pod).await {
        Ok(logs) => Json(serde_json::json!({"success": true, "data": logs})),
        Err(e) => Json(serde_json::json!({"success": false, "error": e})),
    }
}
