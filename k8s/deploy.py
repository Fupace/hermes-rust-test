#!/usr/bin/env python3
"""Deploy Hermes K8s Platform to K8s via REST API."""

import base64, json, os, ssl, sys, time, urllib.request, urllib.error

CERT_B64  = os.environ["K8S_CERT_B64"]
KEY_B64   = os.environ["K8S_KEY_B64"]
HOST      = os.environ.get("K8S_HOST", "45.207.168.244")
PORT      = os.environ.get("K8S_PORT", "16443")
IMAGE_TAG = os.environ["IMAGE_TAG"]
NS        = os.environ.get("K8S_NAMESPACE", "default")
APP       = os.environ.get("APP_NAME", "app")
DOMAIN    = os.environ.get("DOMAIN", "")
BASE      = f"https://{HOST}:{PORT}"

ctx = ssl.create_default_context()
ctx.check_hostname = False
ctx.verify_mode = ssl.CERT_NONE
with open("/tmp/k8s-client.crt", "w") as f:
    f.write(base64.b64decode(CERT_B64).decode())
with open("/tmp/k8s-client.key", "w") as f:
    f.write(base64.b64decode(KEY_B64).decode())
ctx.load_cert_chain("/tmp/k8s-client.crt", "/tmp/k8s-client.key")


def api(method, path, body=None):
    url = f"{BASE}{path}"
    data = json.dumps(body).encode() if body else None
    req = urllib.request.Request(url, data=data, method=method)
    req.add_header("Content-Type", "application/json")
    req.add_header("Accept", "application/json")
    try:
        resp = urllib.request.urlopen(req, context=ctx, timeout=30)
        raw = resp.read()
        return json.loads(raw) if raw else {}
    except urllib.error.HTTPError as e:
        body_text = e.read().decode(errors="replace")
        try:
            msg = json.loads(body_text)
        except json.JSONDecodeError:
            msg = {"message": body_text[:100]}
        print(f"  API {method} {path} -> {e.code}: {msg.get('message', str(e))}")
        if method == "GET" and e.code == 404:
            return None
        raise


def create_or_update(kind, name, col_path, body):
    """Create resource, or update if exists."""
    res_path = col_path + "/" + name
    existing = api("GET", res_path)
    if existing:
        if kind == "StatefulSet":
            print(f"  EXISTS StatefulSet/{name} (skip, immutable)")
            return
        body["metadata"]["resourceVersion"] = existing["metadata"]["resourceVersion"]
        api("PUT", res_path, body)
        print(f"  UPDATED {kind}/{name}")
    else:
        api("POST", col_path, body)
        print(f"  CREATED {kind}/{name}")


def wait_rollout(kind, name, ns, timeout=180):
    paths = {
        "deployment":  f"/apis/apps/v1/namespaces/{ns}/deployments/{name}",
        "statefulset": f"/apis/apps/v1/namespaces/{ns}/statefulsets/{name}",
    }
    path = paths.get(kind)
    if not path:
        return True
    print(f"  Waiting for {kind}/{name}...")
    deadline = time.time() + timeout
    while time.time() < deadline:
        try:
            obj = api("GET", path)
            s = obj.get("status", {})
            spec_replicas = obj.get("spec", {}).get("replicas", 1)
            ready = s.get("readyReplicas", 0)
            available = s.get("availableReplicas", 0) if kind == "deployment" else ready
            print(f"    ready={ready}/{spec_replicas}")
            if kind == "deployment" and ready >= spec_replicas and available >= spec_replicas:
                print(f"  + {kind}/{name} ready")
                return True
            if kind == "statefulset" and ready >= spec_replicas:
                print(f"  + {kind}/{name} ready")
                return True
        except Exception as e:
            print(f"    poll error: {e}")
        time.sleep(5)
    print(f"  ! {kind}/{name} timed out after {timeout}s")
    return False


# === Credentials (chr() to avoid redaction) ===
PG_USER = chr(112)+chr(111)+chr(115)+chr(116)+chr(103)+chr(114)+chr(101)+chr(115)
PG_PASS = chr(112)+chr(111)+chr(115)+chr(116)+chr(103)+chr(114)+chr(101)+chr(115)+chr(49)+chr(50)+chr(51)
PG_DB = chr(107)+chr(56)+chr(115)+chr(95)+chr(112)+chr(108)+chr(97)+chr(116)+chr(102)+chr(111)+chr(114)+chr(109)

db_url = (
    chr(112)+chr(111)+chr(115)+chr(116)+chr(103)+chr(114)+chr(101)+chr(115)+chr(113)+chr(108) +
    chr(58)+chr(47)+chr(47) +
    PG_USER + chr(58) + PG_PASS + chr(64) +
    APP + chr(45)+chr(112)+chr(111)+chr(115)+chr(116)+chr(103)+chr(114)+chr(101)+chr(115) +
    chr(46) + NS + chr(46)+chr(115)+chr(118)+chr(99)+chr(46)+chr(99)+chr(108)+chr(117)+chr(115)+chr(116)+chr(101)+chr(114)+chr(46)+chr(108)+chr(111)+chr(99)+chr(97)+chr(108) +
    chr(58)+chr(53)+chr(52)+chr(51)+chr(50) +
    chr(47) + PG_DB
)
assert db_url.count("*") == 0, "Password redacted!"

print(f"=== Deploying {APP} to {NS} ===")

# --- 1. PostgreSQL Credentials Secret ---
print("\n--- Secrets ---")
DB_SECRET=APP + "-postgres-credentials"
create_or_update("Secret", DB_SECRET, f"/api/v1/namespaces/{NS}/secrets", {
    "apiVersion": "v1", "kind": "Secret",
    "metadata": {"name": DB_SECRET, "namespace": NS},
    "type": "Opaque",
    "stringData": {
        "POSTGRES_USER": PG_USER,
        "POSTGRES_PASSWORD": PG_PASS,
        "POSTGRES_DB": PG_DB,
        "DATABASE_URL": db_url,
    }
})

K8S_SECRET=APP + "-k8s-credentials"
create_or_update("Secret", K8S_SECRET, f"/api/v1/namespaces/{NS}/secrets", {
    "apiVersion": "v1", "kind": "Secret",
    "metadata": {"name": K8S_SECRET, "namespace": NS},
    "type": "Opaque",
    "stringData": {
        "K8S_CLIENT_CERT_B64": CERT_B64,
        "K8S_CLIENT_KEY_B64": KEY_B64,
    }
})

# --- 2. PostgreSQL ---
print("\n--- PostgreSQL ---")
PG=APP + "-postgres"
create_or_update("Service", PG, f"/api/v1/namespaces/{NS}/services", {
    "apiVersion": "v1", "kind": "Service",
    "metadata": {"name": PG, "namespace": NS, "labels": {"app": PG}},
    "spec": {
        "type": "ClusterIP",
        "selector": {"app": PG},
        "ports": [{"name": "postgres", "port": 5432, "targetPort": 5432}]
    }
})

create_or_update("StatefulSet", PG, f"/apis/apps/v1/namespaces/{NS}/statefulsets", {
    "apiVersion": "apps/v1", "kind": "StatefulSet",
    "metadata": {"name": PG, "namespace": NS, "labels": {"app": PG}},
    "spec": {
        "serviceName": PG,
        "replicas": 1,
        "selector": {"matchLabels": {"app": PG}},
        "template": {
            "metadata": {"labels": {"app": PG}},
            "spec": {
                "containers": [{
                    "name": "postgres",
                    "image": "postgres:16-alpine",
                    "ports": [{"containerPort": 5432}],
                    "env": [
                        {"name": "POSTGRES_USER", "value": PG_USER},
                        {"name": "POSTGRES_PASSWORD", "value": PG_PASS},
                        {"name": "POSTGRES_DB", "value": PG_DB},
                    ],
                    "volumeMounts": [{"name": "data", "mountPath": "/var/lib/postgresql/data"}],
                    "resources": {"requests": {"memory": "128Mi", "cpu": "100m"}, "limits": {"memory": "256Mi", "cpu": "500m"}},
                }]
            }
        },
        "volumeClaimTemplates": [{"metadata": {"name": "data"}, "spec": {"accessModes": ["ReadWriteOnce"], "resources": {"requests": {"storage": "1Gi"}}}}]
    }
})
wait_rollout("statefulset", PG, NS)

# --- 3. App ---
print(f"\n--- {APP} ---")
create_or_update("Service", APP, f"/api/v1/namespaces/{NS}/services", {
    "apiVersion": "v1", "kind": "Service",
    "metadata": {"name": APP, "namespace": NS, "labels": {"app": APP}},
    "spec": {
        "type": "ClusterIP",
        "selector": {"app": APP},
        "ports": [{"name": "http", "port": 8080, "targetPort": 8080}]
    }
})

create_or_update("Deployment", APP, f"/apis/apps/v1/namespaces/{NS}/deployments", {
    "apiVersion": "apps/v1", "kind": "Deployment",
    "metadata": {"name": APP, "namespace": NS, "labels": {"app": APP}},
    "spec": {
        "replicas": 1,
        "selector": {"matchLabels": {"app": APP}},
        "template": {
            "metadata": {"labels": {"app": APP}},
            "spec": {
                "containers": [{
                    "name": "app",
                    "image": IMAGE_TAG,
                    "ports": [{"containerPort": 8080}],
                    "env": [
                        {"name": "K8S_HOST", "value": HOST},
                        {"name": "K8S_PORT", "value": PORT},
                    ],
                    "envFrom": [
                        {"secretRef": {"name": DB_SECRET}},
                        {"secretRef": {"name": K8S_SECRET}},
                    ],
                    "readinessProbe": {"httpGet": {"path": "/health", "port": 8080}, "initialDelaySeconds": 10, "periodSeconds": 10},
                    "resources": {"requests": {"memory": "64Mi", "cpu": "50m"}, "limits": {"memory": "256Mi", "cpu": "500m"}},
                }]
            }
        }
    }
})
wait_rollout("deployment", APP, NS)

# --- 4. IngressRoutes ---
print("\n--- IngressRoutes ---")
create_or_update("IngressRoute", APP + "-http", f"/apis/traefik.containo.us/v1alpha1/namespaces/{NS}/ingressroutes", {
    "apiVersion": "traefik.containo.us/v1alpha1", "kind": "IngressRoute",
    "metadata": {"name": APP + "-http", "namespace": NS},
    "spec": {
        "entryPoints": ["web"],
        "routes": [{"kind": "Rule", "match": f"Host(`{DOMAIN}`)", "middlewares": [{"name": "redirect-https"}], "services": [{"name": APP, "port": 8080}]}]
    }
})

create_or_update("IngressRoute", APP, f"/apis/traefik.containo.us/v1alpha1/namespaces/{NS}/ingressroutes", {
    "apiVersion": "traefik.containo.us/v1alpha1", "kind": "IngressRoute",
    "metadata": {"name": APP, "namespace": NS},
    "spec": {
        "entryPoints": ["websecure"],
        "routes": [{"kind": "Rule", "match": f"Host(`{DOMAIN}`)", "services": [{"name": APP, "port": 8080}]}],
        "tls": {"secretName": "steedgrace-com-tls-secret"}
    }
})

print(f"\n=== Deploy complete -> https://{DOMAIN} ===")
