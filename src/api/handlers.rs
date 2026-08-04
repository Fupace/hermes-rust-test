use crate::api::{k8s, AppState};
use crate::models::k8s::*;
use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;
use sqlx::{PgPool, Row};

#[derive(Deserialize)]
pub struct PodQuery {
    namespace: Option<String>,
}
#[derive(Deserialize)]
pub struct ResourceQuery {
    namespace: Option<String>,
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({"status":"healthy","service":"hermes-k8s-platform","version":"0.2.0"}))
}

// --- Cluster CRUD ---
pub async fn list_clusters(State(state): State<AppState>) -> Json<serde_json::Value> {
    match sqlx::query_as::<_, Cluster>("SELECT id, name, COALESCE(description,'') as description, created_at FROM clusters ORDER BY created_at DESC")
        .fetch_all(&state.pool).await
    {
        Ok(clusters) => Json(serde_json::json!({"success":true,"data":clusters})),
        Err(e) => Json(serde_json::json!({"success":false,"error":e.to_string()})),
    }
}

pub async fn add_cluster(
    State(state): State<AppState>,
    Json(body): Json<AddClusterRequest>,
) -> Json<serde_json::Value> {
    match sqlx::query(
        "INSERT INTO clusters (name, kubeconfig_b64, description) VALUES ($1, $2, $3) RETURNING id",
    )
    .bind(&body.name)
    .bind(&body.kubeconfig)
    .bind(body.description.unwrap_or_default())
    .fetch_one(&state.pool)
    .await
    {
        Ok(row) => {
            let id: uuid::Uuid = row.try_get::<uuid::Uuid, _>("id").unwrap();
            Json(serde_json::json!({"success":true,"id":id}))
        }
        Err(e) => Json(serde_json::json!({"success":false,"error":e.to_string()})),
    }
}

pub async fn get_cluster(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Json<serde_json::Value> {
    match sqlx::query_as::<_, Cluster>("SELECT id, name, COALESCE(description,'') as description, created_at FROM clusters WHERE id = $1")
        .bind(id).fetch_optional(&state.pool).await
    {
        Ok(Some(c)) => Json(serde_json::json!({"success":true,"data":c})),
        Ok(None) => Json(serde_json::json!({"success":false,"error":"not found"})),
        Err(e) => Json(serde_json::json!({"success":false,"error":e.to_string()})),
    }
}

pub async fn delete_cluster(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Json<serde_json::Value> {
    match sqlx::query("DELETE FROM clusters WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await
    {
        Ok(_) => Json(serde_json::json!({"success":true})),
        Err(e) => Json(serde_json::json!({"success":false,"error":e.to_string()})),
    }
}

// --- Helper: get kubeconfig from DB ---
async fn get_kc(pool: &PgPool, cluster_id: uuid::Uuid) -> Result<String, String> {
    sqlx::query_scalar::<_, String>("SELECT kubeconfig_b64 FROM clusters WHERE id = $1")
        .bind(cluster_id)
        .fetch_optional(pool)
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "cluster not found".into())
}

// --- Resource handlers ---
pub async fn cluster_summary(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Json<serde_json::Value> {
    match get_kc(&state.pool, id).await {
        Ok(kc) => match k8s::get_cluster_summary(&kc).await {
            Ok(s) => Json(serde_json::json!({"success":true,"data":s})),
            Err(e) => Json(serde_json::json!({"success":false,"error":e})),
        },
        Err(e) => Json(serde_json::json!({"success":false,"error":e})),
    }
}

macro_rules! list_handler {
    ($name:ident, $func:ident, $query:ty) => {
        pub async fn $name(State(state): State<AppState>, Path(id): Path<uuid::Uuid>, Query(q): Query<$query>) -> Json<serde_json::Value> {
            match get_kc(&state.pool, id).await {
                Ok(kc) => {
                    let ns = q.namespace.as_deref().filter(|s| !s.is_empty());
                    match k8s::$func(&kc, ns).await {
                        Ok(data) => Json(serde_json::json!({"success":true,"data":data})),
                        Err(e) => Json(serde_json::json!({"success":false,"error":e})),
                    }
                }
                Err(e) => Json(serde_json::json!({"success":false,"error":e})),
            }
        }
    };
}

list_handler!(list_pods, list_pods, PodQuery);
list_handler!(list_deployments, list_deployments, ResourceQuery);
list_handler!(list_services, list_services, ResourceQuery);
list_handler!(list_pvcs, list_pvcs, ResourceQuery);
list_handler!(list_configmaps, list_configmaps, ResourceQuery);
list_handler!(list_secrets, list_secrets, ResourceQuery);

pub async fn list_nodes(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Json<serde_json::Value> {
    match get_kc(&state.pool, id).await {
        Ok(kc) => match k8s::list_nodes(&kc).await {
            Ok(d) => Json(serde_json::json!({"success":true,"data":d})),
            Err(e) => Json(serde_json::json!({"success":false,"error":e})),
        },
        Err(e) => Json(serde_json::json!({"success":false,"error":e})),
    }
}

pub async fn list_namespaces(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Json<serde_json::Value> {
    match get_kc(&state.pool, id).await {
        Ok(kc) => match k8s::list_namespaces(&kc).await {
            Ok(d) => Json(serde_json::json!({"success":true,"data":d})),
            Err(e) => Json(serde_json::json!({"success":false,"error":e})),
        },
        Err(e) => Json(serde_json::json!({"success":false,"error":e})),
    }
}

pub async fn pod_logs(
    State(state): State<AppState>,
    Path((id, ns, pod)): Path<(uuid::Uuid, String, String)>,
) -> Json<serde_json::Value> {
    match get_kc(&state.pool, id).await {
        Ok(kc) => match k8s::get_pod_logs(&kc, &ns, &pod).await {
            Ok(logs) => Json(serde_json::json!({"success":true,"data":logs})),
            Err(e) => Json(serde_json::json!({"success":false,"error":e})),
        },
        Err(e) => Json(serde_json::json!({"success":false,"error":e})),
    }
}

pub async fn pod_yaml(
    State(state): State<AppState>,
    Path((id, ns, pod)): Path<(uuid::Uuid, String, String)>,
) -> Json<serde_json::Value> {
    match get_kc(&state.pool, id).await {
        Ok(kc) => {
            match k8s::get_resource_yaml(&kc, &format!("/api/v1/namespaces/{}/pods/{}", ns, pod))
                .await
            {
                Ok(y) => Json(serde_json::json!({"success":true,"data":y})),
                Err(e) => Json(serde_json::json!({"success":false,"error":e})),
            }
        }
        Err(e) => Json(serde_json::json!({"success":false,"error":e})),
    }
}

pub async fn deployment_yaml(
    State(state): State<AppState>,
    Path((id, ns, name)): Path<(uuid::Uuid, String, String)>,
) -> Json<serde_json::Value> {
    match get_kc(&state.pool, id).await {
        Ok(kc) => match k8s::get_resource_yaml(
            &kc,
            &format!("/apis/apps/v1/namespaces/{}/deployments/{}", ns, name),
        )
        .await
        {
            Ok(y) => Json(serde_json::json!({"success":true,"data":y})),
            Err(e) => Json(serde_json::json!({"success":false,"error":e})),
        },
        Err(e) => Json(serde_json::json!({"success":false,"error":e})),
    }
}

pub async fn create_deployment(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(body): Json<CreateDeploymentRequest>,
) -> Json<serde_json::Value> {
    match get_kc(&state.pool, id).await {
        Ok(kc) => match k8s::create_deployment_api(&kc, &body).await {
            Ok(d) => Json(serde_json::json!({"success":true,"data":d})),
            Err(e) => Json(serde_json::json!({"success":false,"error":e})),
        },
        Err(e) => Json(serde_json::json!({"success":false,"error":e})),
    }
}
