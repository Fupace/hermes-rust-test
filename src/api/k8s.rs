use crate::models::k8s::*;
use base64::Engine;
use reqwest::Client;
use serde_json::Value;

fn build_client(kubeconfig_b64: &str) -> Result<Client, String> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(kubeconfig_b64)
        .map_err(|e| format!("base64: {}", e))?;
    let config: Value = serde_yaml::from_slice(&decoded).map_err(|e| format!("yaml: {}", e))?;

    let mut builder = Client::builder().danger_accept_invalid_certs(true);

    if let Some(users) = config["users"].as_array() {
        if let Some(user) = users.first() {
            if let (Some(cert), Some(key)) = (
                user["user"]["client-certificate-data"].as_str(),
                user["user"]["client-key-data"].as_str(),
            ) {
                let cert_pem = base64::engine::general_purpose::STANDARD
                    .decode(cert)
                    .unwrap_or_default();
                let key_pem = base64::engine::general_purpose::STANDARD
                    .decode(key)
                    .unwrap_or_default();
                let pem = String::from_utf8_lossy(&cert_pem).to_string()
                    + "
" + &String::from_utf8_lossy(&key_pem);
                if let Ok(id) = reqwest::Identity::from_pem(pem.as_bytes()) {
                    builder = builder.identity(id);
                }
            }
        }
    }

    Ok(builder.build().unwrap_or_default())
}

fn get_server(kubeconfig_b64: &str) -> Result<String, String> {
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(kubeconfig_b64)
        .map_err(|e| format!("base64: {}", e))?;
    let config: Value = serde_yaml::from_slice(&decoded).map_err(|e| format!("yaml: {}", e))?;
    config["clusters"][0]["cluster"]["server"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "no server found".into())
}

pub async fn k8s_get(kubeconfig: &str, path: &str) -> Result<Value, String> {
    let client = build_client(kubeconfig)?;
    let server = get_server(kubeconfig)?;
    let url = format!("{}{}", server, path);
    let resp = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("request: {}", e))?;
    resp.json().await.map_err(|e| format!("json: {}", e))
}

pub async fn k8s_get_raw(kubeconfig: &str, path: &str) -> Result<String, String> {
    let client = build_client(kubeconfig)?;
    let server = get_server(kubeconfig)?;
    let url = format!("{}{}", server, path);
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("request: {}", e))?;
    resp.text().await.map_err(|e| format!("text: {}", e))
}

pub async fn k8s_post(kubeconfig: &str, path: &str, body: &Value) -> Result<Value, String> {
    let client = build_client(kubeconfig)?;
    let server = get_server(kubeconfig)?;
    let url = format!("{}{}", server, path);
    let resp = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| format!("request: {}", e))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("text: {}", e))?;
    if status.is_success() {
        serde_json::from_str(&text)
            .map_err(|e| format!("json parse: {} from '{}'", e, &text[..200.min(text.len())]))
    } else {
        Err(format!(
            "HTTP {}: {}",
            status.as_u16(),
            &text[..300.min(text.len())]
        ))
    }
}

#[allow(dead_code)]
pub async fn k8s_delete(kubeconfig: &str, path: &str) -> Result<Value, String> {
    let client = build_client(kubeconfig)?;
    let server = get_server(kubeconfig)?;
    let url = format!("{}{}", server, path);
    let resp = client
        .delete(&url)
        .send()
        .await
        .map_err(|e| format!("request: {}", e))?;
    resp.json().await.map_err(|e| format!("json: {}", e))
}

fn age(ts: &str) -> String {
    if ts.is_empty() {
        return "unknown".into();
    }
    ts[..ts.len().min(19)].replace('T', " ")
}

// --- Resource fetchers ---

pub async fn get_cluster_summary(kc: &str) -> Result<ClusterSummary, String> {
    let ns = k8s_get(kc, "/api/v1/namespaces").await?;
    let pods = k8s_get(kc, "/api/v1/pods").await?;
    let nodes = k8s_get(kc, "/api/v1/nodes").await?;
    let deploys = k8s_get(kc, "/apis/apps/v1/deployments")
        .await
        .unwrap_or(serde_json::json!({"items":[]}));
    let svcs = k8s_get(kc, "/api/v1/services")
        .await
        .unwrap_or(serde_json::json!({"items":[]}));
    let pvcs = k8s_get(kc, "/api/v1/persistentvolumeclaims")
        .await
        .unwrap_or(serde_json::json!({"items":[]}));
    Ok(ClusterSummary {
        namespaces: ns["items"].as_array().map(|a| a.len()).unwrap_or(0),
        pods: pods["items"].as_array().map(|a| a.len()).unwrap_or(0),
        deployments: deploys["items"].as_array().map(|a| a.len()).unwrap_or(0),
        services: svcs["items"].as_array().map(|a| a.len()).unwrap_or(0),
        nodes: nodes["items"].as_array().map(|a| a.len()).unwrap_or(0),
        pvcs: pvcs["items"].as_array().map(|a| a.len()).unwrap_or(0),
    })
}

pub async fn list_namespaces(kc: &str) -> Result<Vec<NamespaceInfo>, String> {
    let data = k8s_get(kc, "/api/v1/namespaces").await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    Ok(items
        .iter()
        .map(|i| NamespaceInfo {
            name: i["metadata"]["name"].as_str().unwrap_or("").into(),
            status: i["status"]["phase"].as_str().unwrap_or("Active").into(),
            age: age(i["metadata"]["creationTimestamp"].as_str().unwrap_or("")),
        })
        .collect())
}

pub async fn list_pods(kc: &str, ns: Option<&str>) -> Result<Vec<PodInfo>, String> {
    let path = ns.map_or("/api/v1/pods".into(), |n| {
        format!("/api/v1/namespaces/{}/pods", n)
    });
    let data = k8s_get(kc, &path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    Ok(items
        .iter()
        .map(|i| {
            let meta = &i["metadata"];
            let spec = &i["spec"];
            let status = &i["status"];
            let name = meta["name"].as_str().unwrap_or("");
            let containers = spec["containers"]
                .as_array()
                .map(|cs| {
                    cs.iter()
                        .map(|c| {
                            let cn = c["name"].as_str().unwrap_or("");
                            ContainerInfo {
                                name: cn.into(),
                                image: c["image"].as_str().unwrap_or("").into(),
                                ready: status["containerStatuses"]
                                    .as_array()
                                    .map(|scs| {
                                        scs.iter().any(|sc| {
                                            sc["name"].as_str() == Some(cn)
                                                && sc["ready"].as_bool().unwrap_or(false)
                                        })
                                    })
                                    .unwrap_or(false),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            PodInfo {
                name: name.into(),
                namespace: meta["namespace"].as_str().unwrap_or("").into(),
                status: status["phase"].as_str().unwrap_or("Unknown").into(),
                node: spec["nodeName"].as_str().unwrap_or("-").into(),
                restarts: status["containerStatuses"]
                    .as_array()
                    .map(|cs| {
                        cs.iter()
                            .map(|c| c["restartCount"].as_i64().unwrap_or(0) as i32)
                            .sum()
                    })
                    .unwrap_or(0),
                age: age(meta["creationTimestamp"].as_str().unwrap_or("")),
                containers,
            }
        })
        .collect())
}

pub async fn list_deployments(kc: &str, ns: Option<&str>) -> Result<Vec<DeploymentInfo>, String> {
    let path = ns.map_or("/apis/apps/v1/deployments".into(), |n| {
        format!("/apis/apps/v1/namespaces/{}/deployments", n)
    });
    let data = k8s_get(kc, &path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    Ok(items
        .iter()
        .map(|i| {
            let m = &i["metadata"];
            let s = &i["spec"];
            let st = &i["status"];
            DeploymentInfo {
                name: m["name"].as_str().unwrap_or("").into(),
                namespace: m["namespace"].as_str().unwrap_or("").into(),
                replicas: s["replicas"].as_i64().unwrap_or(0) as i32,
                ready: st["readyReplicas"].as_i64().unwrap_or(0) as i32,
                available: st["availableReplicas"].as_i64().unwrap_or(0) as i32,
                age: age(m["creationTimestamp"].as_str().unwrap_or("")),
                containers: s["template"]["spec"]["containers"]
                    .as_array()
                    .map(|cs| {
                        cs.iter()
                            .map(|c| c["name"].as_str().unwrap_or("").into())
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect())
}

pub async fn list_services(kc: &str, ns: Option<&str>) -> Result<Vec<ServiceInfo>, String> {
    let path = ns.map_or("/api/v1/services".into(), |n| {
        format!("/api/v1/namespaces/{}/services", n)
    });
    let data = k8s_get(kc, &path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    Ok(items
        .iter()
        .map(|i| {
            let m = &i["metadata"];
            let s = &i["spec"];
            ServiceInfo {
                name: m["name"].as_str().unwrap_or("").into(),
                namespace: m["namespace"].as_str().unwrap_or("").into(),
                service_type: s["type"].as_str().unwrap_or("ClusterIP").into(),
                cluster_ip: s["clusterIP"].as_str().unwrap_or("None").into(),
                ports: s["ports"]
                    .as_array()
                    .map(|ps| {
                        ps.iter()
                            .map(|p| {
                                format!(
                                    "{}/{}",
                                    p["port"].as_i64().unwrap_or(0),
                                    p["protocol"].as_str().unwrap_or("TCP")
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
                age: age(m["creationTimestamp"].as_str().unwrap_or("")),
            }
        })
        .collect())
}

pub async fn list_nodes(kc: &str) -> Result<Vec<NodeInfo>, String> {
    let data = k8s_get(kc, "/api/v1/nodes").await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    Ok(items
        .iter()
        .map(|i| {
            let m = &i["metadata"];
            let s = &i["status"];
            let empty_obj = serde_json::json!({});
            let cap = &s.get("capacity").unwrap_or(&empty_obj);
            NodeInfo {
                name: m["name"].as_str().unwrap_or("").into(),
                status: s["conditions"]
                    .as_array()
                    .map(|cs| {
                        cs.iter()
                            .find(|c| c["type"].as_str() == Some("Ready"))
                            .map(|c| {
                                if c["status"].as_str() == Some("True") {
                                    "Ready"
                                } else {
                                    "NotReady"
                                }
                            })
                            .unwrap_or("Unknown")
                    })
                    .unwrap_or("Unknown")
                    .into(),
                roles: m["labels"]["kubernetes.io/role"]
                    .as_str()
                    .unwrap_or("worker")
                    .into(),
                version: s["nodeInfo"]["kubeletVersion"]
                    .as_str()
                    .unwrap_or("")
                    .into(),
                cpu: cap["cpu"].as_str().unwrap_or("?").into(),
                memory: cap["memory"].as_str().unwrap_or("?").into(),
                age: age(m["creationTimestamp"].as_str().unwrap_or("")),
            }
        })
        .collect())
}

pub async fn list_pvcs(kc: &str, ns: Option<&str>) -> Result<Vec<PvcInfo>, String> {
    let path = ns.map_or("/api/v1/persistentvolumeclaims".into(), |n| {
        format!("/api/v1/namespaces/{}/persistentvolumeclaims", n)
    });
    let data = k8s_get(kc, &path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    Ok(items
        .iter()
        .map(|i| {
            let m = &i["metadata"];
            let s = &i["spec"];
            let st = &i["status"];
            PvcInfo {
                name: m["name"].as_str().unwrap_or("").into(),
                namespace: m["namespace"].as_str().unwrap_or("").into(),
                status: st["phase"].as_str().unwrap_or("Pending").into(),
                capacity: s["resources"]["requests"]["storage"]
                    .as_str()
                    .unwrap_or("?")
                    .into(),
                storage_class: s["storageClassName"].as_str().unwrap_or("-").into(),
                age: age(m["creationTimestamp"].as_str().unwrap_or("")),
            }
        })
        .collect())
}

pub async fn list_configmaps(kc: &str, ns: Option<&str>) -> Result<Vec<ConfigMapInfo>, String> {
    let path = ns.map_or("/api/v1/configmaps".into(), |n| {
        format!("/api/v1/namespaces/{}/configmaps", n)
    });
    let data = k8s_get(kc, &path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    Ok(items
        .iter()
        .map(|i| {
            let m = &i["metadata"];
            ConfigMapInfo {
                name: m["name"].as_str().unwrap_or("").into(),
                namespace: m["namespace"].as_str().unwrap_or("").into(),
                keys: i["data"]
                    .as_object()
                    .map(|o| o.keys().cloned().collect())
                    .unwrap_or_default(),
                age: age(m["creationTimestamp"].as_str().unwrap_or("")),
            }
        })
        .collect())
}

pub async fn list_secrets(kc: &str, ns: Option<&str>) -> Result<Vec<SecretInfo>, String> {
    let path = ns.map_or("/api/v1/secrets".into(), |n| {
        format!("/api/v1/namespaces/{}/secrets", n)
    });
    let data = k8s_get(kc, &path).await?;
    let Some(items) = data["items"].as_array() else {
        return Ok(vec![]);
    };
    Ok(items
        .iter()
        .map(|i| {
            let m = &i["metadata"];
            SecretInfo {
                name: m["name"].as_str().unwrap_or("").into(),
                namespace: m["namespace"].as_str().unwrap_or("").into(),
                secret_type: i["type"].as_str().unwrap_or("Opaque").into(),
                keys: i["data"]
                    .as_object()
                    .map(|o| o.keys().cloned().collect())
                    .unwrap_or_default(),
                age: age(m["creationTimestamp"].as_str().unwrap_or("")),
            }
        })
        .collect())
}

pub async fn get_pod_logs(kc: &str, ns: &str, pod: &str) -> Result<String, String> {
    k8s_get_raw(
        kc,
        &format!("/api/v1/namespaces/{}/pods/{}/log?tailLines=200", ns, pod),
    )
    .await
}

pub async fn get_resource_yaml(kc: &str, path: &str) -> Result<String, String> {
    k8s_get_raw(kc, path).await
}

pub async fn create_deployment_api(
    kc: &str,
    req: &CreateDeploymentRequest,
) -> Result<Value, String> {
    let body = serde_json::json!({
        "apiVersion": "apps/v1",
        "kind": "Deployment",
        "metadata": {"name": req.name, "namespace": req.namespace},
        "spec": {
            "replicas": req.replicas.unwrap_or(1),
            "selector": {"matchLabels": {"app": req.name}},
            "template": {
                "metadata": {"labels": {"app": req.name}},
                "spec": {"containers": [{
                    "name": req.name,
                    "image": req.image,
                    "ports": [{"containerPort": req.port.unwrap_or(8080)}]
                }]}
            }
        }
    });
    k8s_post(
        kc,
        &format!("/apis/apps/v1/namespaces/{}/deployments", req.namespace),
        &body,
    )
    .await
}
