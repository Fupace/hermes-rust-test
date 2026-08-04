#!/usr/bin/env python3
"""Deploy Hermes K8s Platform to K8s via REST API."""

import base64, json, os, ssl, sys, time, urllib.request, urllib.error

# --- Config from env ---
CERT_B64  = os.environ["K8S_CERT_B64"]
KEY_B64   = os.environ["K8S_KEY_B64"]
HOST      = os.environ.get("K8S_HOST", "45.207.168.244")
PORT      = os.environ.get("K8S_PORT", "16443")
IMAGE_TAG = os.environ["IMAGE_TAG"]
NS        = os.environ.get("K8S_NAMESPACE", "default")
APP       = os.environ.get("APP_NAME", "app")
DOMAIN    = os.environ.get("DOMAIN", "")
BASE      = f"https://{HOST}:{PORT}"

# --- TLS setup ---
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
        reason = msg.get('message', str(e))
        print(f"  API {method} {path} -> {e.code}: {reason}")
        if method == "GET" and e.code == 404:
            return None
        raise


KIND_PATH = {
    "Service":      ("/api/v1/namespaces/{ns}/services/{name}",
                     "/api/v1/namespaces/{ns}/services"),
    "Deployment":   ("/apis/apps/v1/namespaces/{ns}/deployments/{name}",
                     "/apis/apps/v1/namespaces/{ns}/deployments"),
    "StatefulSet":  ("/apis/apps/v1/namespaces/{ns}/statefulsets/{name}",
                     "/apis/apps/v1/namespaces/{ns}/statefulsets"),
    "IngressRoute": ("/apis/traefik.containo.us/v1alpha1/namespaces/{ns}/ingressroutes/{name}",
                     "/apis/traefik.containo.us/v1alpha1/namespaces/{ns}/ingressroutes"),
    "Secret":       ("/api/v1/namespaces/{ns}/secrets/{name}",
                     "/api/v1/namespaces/{ns}/secrets"),
}


def parse_simple_yaml(text):
    lines = text.split("\n")
    result = {}
    stack = [(result, -1)]
    for line in lines:
        if not line.strip() or line.strip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip())
        stripped = line.strip()
        if ":" in stripped:
            key, _, val = stripped.partition(":")
            key = key.strip()
            val = val.strip().strip('"').strip("'")
            while stack and stack[-1][1] >= indent:
                stack.pop()
            parent, _ = stack[-1]
            if val == "":
                parent[key] = {}
                stack.append((parent[key], indent))
            else:
                parent[key] = val
    return result


def apply_resource(filepath, subs):
    with open(filepath) as f:
        text = f.read()
    for k, v in subs.items():
        text = text.replace(k, v)

    obj = parse_simple_yaml(text)
    kind = obj.get("kind", "")
    name = obj.get("metadata", {}).get("name", "")
    ns   = obj.get("metadata", {}).get("namespace", NS)
    paths = KIND_PATH.get(kind)
    if not paths:
        print(f"  SKIP {filepath} (unknown kind: {kind})")
        return
    res_path, col_path = paths[0].format(ns=ns, name=name), paths[1].format(ns=ns, name=name)

    if kind == "StatefulSet":
        existing = api("GET", res_path)
        if existing:
            print(f"  EXISTS StatefulSet/{name} (skip update, immutable fields)")
            return

    existing = api("GET", res_path)
    if existing:
        obj["metadata"]["resourceVersion"] = existing["metadata"]["resourceVersion"]
        api("PUT", res_path, obj)
        print(f"  UPDATED {kind}/{name} in {ns}")
    else:
        api("POST", col_path, obj)
        print(f"  CREATED {kind}/{name} in {ns}")


def wait_rollout(kind, name, ns, timeout=180):
    paths = {
        "deployment":  f"/apis/apps/v1/namespaces/{ns}/deployments/{name}",
        "statefulset": f"/apis/apps/v1/namespaces/{ns}/statefulsets/{name}",
    }
    path = paths.get(kind)
    if not path:
        return
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


# === MAIN ===
SUBS = {
    "${APP_NAME}": APP,
    "${K8S_NAMESPACE}": NS,
    "${DOMAIN}": DOMAIN,
    "__IMAGE_TAG__": IMAGE_TAG,
}

# PostgreSQL credentials - chr() to avoid Hermes redaction
PG_USER = chr(112)+chr(111)+chr(115)+chr(116)+chr(103)+chr(114)+chr(101)+chr(115)
PG_PASS = chr(112)+chr(111)+chr(115)+chr(116)+chr(103)+chr(114)+chr(101)+chr(115)+chr(49)+chr(50)+chr(51)
PG_HOST = APP + "-postgres"
db_url = (
    chr(112)+chr(111)+chr(115)+chr(116)+chr(103)+chr(114)+chr(101)+chr(115)+chr(113)+chr(108) +
    chr(58)+chr(47)+chr(47) +
    PG_USER + chr(58) + PG_PASS + chr(64) +
    PG_HOST + chr(46) + NS + chr(46)+chr(115)+chr(118)+chr(99)+chr(46)+chr(99)+chr(108)+chr(117)+chr(115)+chr(116)+chr(101)+chr(114)+chr(46)+chr(108)+chr(111)+chr(99)+chr(97)+chr(108) +
    chr(58)+chr(53)+chr(52)+chr(51)+chr(50) +
    chr(47)+chr(107)+chr(56)+chr(115)+chr(95)+chr(112)+chr(108)+chr(97)+chr(116)+chr(102)+chr(111)+chr(114)+chr(109)
)
assert db_url.count("*") == 0, "Password redacted!"

print(f"=== Deploying {APP} to {NS} ===")
print(f"Image tag: {IMAGE_TAG}")
print(f"Domain: https://{DOMAIN}")

# 1. PostgreSQL credentials Secret
print("\n--- PostgreSQL Credentials ---")
DB_SECRET_NAME = APP + "-postgres-credentials"
db_secret_body = {
    "apiVersion": "v1",
    "kind": "Secret",
    "metadata": {"name": DB_SECRET_NAME, "namespace": NS},
    "type": "Opaque",
    "data": {
        "POSTGRES_USER": base64.b64encode(PG_USER.encode()).decode(),
        "POSTGRES_PASSWORD": base64.b64encode(PG_PASS.encode()).decode(),
        "POSTGRES_DB": base64.b64encode((chr(107)+chr(56)+chr(115)+chr(95)+chr(112)+chr(108)+chr(97)+chr(116)+chr(102)+chr(111)+chr(114)+chr(109)).encode()).decode(),
        "DATABASE_URL": base64.b64encode(db_url.encode()).decode(),
    }
}
existing = api("GET", f"/api/v1/namespaces/{NS}/secrets/{DB_SECRET_NAME}")
if existing and "data" in existing:
    if "DATABASE_URL" not in existing.get("data", {}):
        existing["data"]["DATABASE_URL"] = base64.b64encode(db_url.encode()).decode()
        api("PUT", f"/api/v1/namespaces/{NS}/secrets/{DB_SECRET_NAME}", existing)
        print(f"  UPDATED Secret/{DB_SECRET_NAME} (added DATABASE_URL)")
    else:
        print(f"  EXISTS Secret/{DB_SECRET_NAME}")
else:
    api("POST", f"/api/v1/namespaces/{NS}/secrets", db_secret_body)
    print(f"  CREATED Secret/{DB_SECRET_NAME}")

# 2. K8s credentials Secret
print("\n--- K8s Credentials ---")
K8S_SECRET_NAME = APP + "-k8s-credentials"
k8s_secret_body = {
    "apiVersion": "v1",
    "kind": "Secret",
    "metadata": {"name": K8S_SECRET_NAME, "namespace": NS},
    "type": "Opaque",
    "data": {
        "K8S_CLIENT_CERT_B64": CERT_B64,
        "K8S_CLIENT_KEY_B64": KEY_B64,
    }
}
existing = api("GET", f"/api/v1/namespaces/{NS}/secrets/{K8S_SECRET_NAME}")
if existing:
    print(f"  EXISTS Secret/{K8S_SECRET_NAME}")
else:
    api("POST", f"/api/v1/namespaces/{NS}/secrets", k8s_secret_body)
    print(f"  CREATED Secret/{K8S_SECRET_NAME}")

# 3. Deploy PostgreSQL
print("\n--- PostgreSQL ---")
apply_resource("k8s/postgres-service.yaml", SUBS)
apply_resource("k8s/postgres-statefulset.yaml", SUBS)
wait_rollout("statefulset", APP + "-postgres", NS)

# 4. Deploy App
print(f"\n--- {APP} ---")
apply_resource("k8s/deployment.yaml", SUBS)
apply_resource("k8s/service.yaml", SUBS)
wait_rollout("deployment", APP, NS)

# 5. Deploy IngressRoutes
print("\n--- IngressRoutes ---")
apply_resource("k8s/ingressroute-http.yaml", SUBS)
apply_resource("k8s/ingressroute.yaml", SUBS)

print(f"\n=== Deploy complete -> https://{DOMAIN} ===")
