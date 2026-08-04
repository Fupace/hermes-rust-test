use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterSummary {
    pub namespaces: usize,
    pub pods: usize,
    pub deployments: usize,
    pub services: usize,
    pub nodes: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NamespaceInfo {
    pub name: String,
    pub status: String,
    pub age: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PodInfo {
    pub name: String,
    pub namespace: String,
    pub status: String,
    pub node: String,
    pub restarts: i32,
    pub age: String,
    pub containers: Vec<ContainerInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ContainerInfo {
    pub name: String,
    pub image: String,
    pub ready: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeploymentInfo {
    pub name: String,
    pub namespace: String,
    pub replicas: i32,
    pub ready: i32,
    pub available: i32,
    pub age: String,
    pub containers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServiceInfo {
    pub name: String,
    pub namespace: String,
    pub service_type: String,
    pub cluster_ip: String,
    pub ports: Vec<String>,
    pub age: String,
}
