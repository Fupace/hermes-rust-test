use serde::{Deserialize, Serialize};
use sqlx::FromRow;

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct Cluster {
    pub id: uuid::Uuid,
    pub name: String,
    pub description: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddClusterRequest {
    pub name: String,
    pub kubeconfig: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ClusterSummary {
    pub namespaces: usize,
    pub pods: usize,
    pub deployments: usize,
    pub services: usize,
    pub nodes: usize,
    pub pvcs: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NamespaceInfo {
    pub name: String,
    pub status: String,
    pub age: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PodInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub node: String,
    pub restarts: i32,
    pub age: String,
    pub containers: Vec<ContainerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub ready: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DeploymentInfo {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,
    pub ready: i32,
    pub available: i32,
    pub age: String,
    pub containers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateDeploymentRequest {
    pub namespace: String,
    pub name: String,
    pub image: String,
    pub replicas: Option<i32>,
    pub port: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub namespace: String,
    pub service_type: String,
    pub cluster_ip: String,
    pub ports: Vec<String>,
    pub age: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct NodeInfo {
    pub name: String,
    pub status: String,
    pub roles: String,
    pub version: String,
    pub cpu: String,
    pub memory: String,
    pub age: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PvcInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub capacity: String,
    pub storage_class: String,
    pub age: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ConfigMapInfo {
    pub name: String,
    pub namespace: String,
    pub keys: Vec<String>,
    pub age: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecretInfo {
    pub name: String,
    pub namespace: String,
    pub secret_type: String,
    pub keys: Vec<String>,
    pub age: String,
}
