use crate::models::k8s::*;
use reqwest::Client;
use serde_json::Value;
use std::env;

fn k8s_base() -> (String, Client) {
    let host = env::var("K8S_HOST").unwrap_or_else(|_| "kubernetes.default.svc".into());
    let port = env::var("K8S_PORT").unwrap_or_else(|_| "443".into());
    let base = format!("https://{}:{}", host, port);

    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .build()
        .unwrap_or_default();

    (base, client)
}

pub async fn get_json(path: &str) -> Result<Value, String> {
    let (base, client) = k8s_base();
    let url = format!("{}{}", base, path);
    let token = env::var("K8S_TOKEN").unwrap_or_default();

    let mut req = client.get(&url).header("Accept", "application/json");
    if !token.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req
        .send()
        .await
        .map_err(|e| format!("K8s API error: {}", e))?;
    let body: Value = resp
        .json()
        .await
        .map_err(|e| format!("JSON parse error: {}", e))?;
    Ok(body)
}

fn age_from_timestamp(ts: &str) -> String {
    if ts.is_empty() {
        return "unknown".into();
    }
    ts[..ts.len().min(19)].replace('T', " ")
}

pub async fn get_cluster_summary() -> Result<ClusterSummary, String> {
    let namespaces = get_json("/api/v1/namespaces").await?;
    let pods = get_json("/api/v1/pods").await?;
    let nodes = get_json("/api/v1/nodes").await?;
    let deployments: Value = get_json("/apis/apps/v1/deployments")
        .await
        .unwrap_or(serde_json::json!({"items": []}));
    let services: Value = get_json("/api/v1/services")
        .await
        .unwrap_or(serde_json::json!({"items": []}));

    Ok(ClusterSummary {
        namespaces: namespaces["items"].as_array().map(|a| a.len()).unwrap_or(0),
        pods: pods["items"].as_array().map(|a| a.len()).unwrap_or(0),
        deployments: deployments["items"]
            .as_array()
            .map(|a| a.len())
            .unwrap_or(0),
        services: services["items"].as_array().map(|a| a.len()).unwrap_or(0),
        nodes: nodes["items"].as_array().map(|a| a.len()).unwrap_or(0),
    })
}

pub async fn list_namespaces_handler() -> Result<Vec<NamespaceInfo>, String> {
    let data = get_json("/api/v1/namespaces").await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    let mut result = Vec::new();
    for item in items {
        let meta = &item["metadata"];
        let status = &item["status"];
        let name = meta["name"].as_str().unwrap_or("").to_string();
        let phase = status["phase"].as_str().unwrap_or("Active").to_string();
        let created = meta["creationTimestamp"].as_str().unwrap_or("");
        result.push(NamespaceInfo {
            name,
            status: phase,
            age: age_from_timestamp(created),
        });
    }
    Ok(result)
}

pub async fn list_pods_handler(ns: Option<&str>) -> Result<Vec<PodInfo>, String> {
    let path = if let Some(ns) = ns {
        format!("/api/v1/namespaces/{}/pods", ns)
    } else {
        "/api/v1/pods".to_string()
    };
    let data = get_json(&path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    let mut result = Vec::new();
    for item in items {
        let meta = &item["metadata"];
        let spec = &item["spec"];
        let status = &item["status"];
        let name = meta["name"].as_str().unwrap_or("").to_string();
        let namespace = meta["namespace"].as_str().unwrap_or("").to_string();
        let pod_status = status["phase"].as_str().unwrap_or("Unknown").to_string();
        let node = spec["nodeName"].as_str().unwrap_or("-").to_string();
        let created = meta["creationTimestamp"].as_str().unwrap_or("");
        let restarts = status["containerStatuses"]
            .as_array()
            .map(|cs| {
                cs.iter()
                    .map(|c| c["restartCount"].as_i64().unwrap_or(0) as i32)
                    .sum()
            })
            .unwrap_or(0);
        let containers = spec["containers"]
            .as_array()
            .map(|cs| {
                cs.iter()
                    .map(|c| {
                        let cname = c["name"].as_str().unwrap_or("").to_string();
                        ContainerInfo {
                            name: cname.clone(),
                            image: c["image"].as_str().unwrap_or("").to_string(),
                            ready: status["containerStatuses"]
                                .as_array()
                                .map(|scs| {
                                    scs.iter().any(|sc| {
                                        sc["name"].as_str() == Some(&cname)
                                            && sc["ready"].as_bool().unwrap_or(false)
                                    })
                                })
                                .unwrap_or(false),
                        }
                    })
                    .collect()
            })
            .unwrap_or_default();
        result.push(PodInfo {
            name,
            namespace,
            status: pod_status,
            node,
            restarts,
            age: age_from_timestamp(created),
            containers,
        });
    }
    Ok(result)
}

pub async fn list_deployments_handler(ns: Option<&str>) -> Result<Vec<DeploymentInfo>, String> {
    let path = if let Some(ns) = ns {
        format!("/apis/apps/v1/namespaces/{}/deployments", ns)
    } else {
        "/apis/apps/v1/deployments".to_string()
    };
    let data = get_json(&path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    let mut result = Vec::new();
    for item in items {
        let meta = &item["metadata"];
        let spec = &item["spec"];
        let status = &item["status"];
        let name = meta["name"].as_str().unwrap_or("").to_string();
        let namespace = meta["namespace"].as_str().unwrap_or("").to_string();
        let replicas = spec["replicas"].as_i64().unwrap_or(0) as i32;
        let ready = status["readyReplicas"].as_i64().unwrap_or(0) as i32;
        let available = status["availableReplicas"].as_i64().unwrap_or(0) as i32;
        let created = meta["creationTimestamp"].as_str().unwrap_or("");
        let containers = spec["template"]["spec"]["containers"]
            .as_array()
            .map(|cs| {
                cs.iter()
                    .map(|c| c["name"].as_str().unwrap_or("").to_string())
                    .collect()
            })
            .unwrap_or_default();
        result.push(DeploymentInfo {
            name,
            namespace,
            replicas,
            ready,
            available,
            age: age_from_timestamp(created),
            containers,
        });
    }
    Ok(result)
}

pub async fn list_services_handler(ns: Option<&str>) -> Result<Vec<ServiceInfo>, String> {
    let path = if let Some(ns) = ns {
        format!("/api/v1/namespaces/{}/services", ns)
    } else {
        "/api/v1/services".to_string()
    };
    let data = get_json(&path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    let mut result = Vec::new();
    for item in items {
        let meta = &item["metadata"];
        let spec = &item["spec"];
        let name = meta["name"].as_str().unwrap_or("").to_string();
        let namespace = meta["namespace"].as_str().unwrap_or("").to_string();
        let svc_type = spec["type"].as_str().unwrap_or("ClusterIP").to_string();
        let cluster_ip = spec["clusterIP"].as_str().unwrap_or("None").to_string();
        let created = meta["creationTimestamp"].as_str().unwrap_or("");
        let ports = spec["ports"]
            .as_array()
            .map(|ps| {
                ps.iter()
                    .map(|p| {
                        let port = p["port"].as_i64().unwrap_or(0);
                        let proto = p["protocol"].as_str().unwrap_or("TCP");
                        let target = p["targetPort"]
                            .as_i64()
                            .map(|t| t.to_string())
                            .unwrap_or_else(|| p["targetPort"].as_str().unwrap_or("?").to_string());
                        format!("{}/{} -> {}", port, proto, target)
                    })
                    .collect()
            })
            .unwrap_or_default();
        result.push(ServiceInfo {
            name,
            namespace,
            service_type: svc_type,
            cluster_ip,
            ports,
            age: age_from_timestamp(created),
        });
    }
    Ok(result)
}

pub async fn get_pod_logs(ns: &str, pod: &str) -> Result<String, String> {
    let path = format!("/api/v1/namespaces/{}/pods/{}/log?tailLines=200", ns, pod);
    let (base, client) = k8s_base();
    let url = format!("{}{}", base, path);
    let token = env::var("K8S_TOKEN").unwrap_or_default();
    let mut req = client.get(&url);
    if !token.is_empty() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }
    let resp = req
        .send()
        .await
        .map_err(|e| format!("K8s API error: {}", e))?;
    resp.text().await.map_err(|e| format!("Text error: {}", e))
}
