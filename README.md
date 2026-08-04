# Hermes K8s Platform

A lightweight Kubernetes cluster management platform (Kuboard-like) built with Rust.

## Features

- **Dashboard** — Cluster overview with namespace, pod, deployment, service, and node counts
- **Pod Management** — List all pods across namespaces, view status, view logs
- **Deployment Management** — List deployments with replica status
- **Service Management** — List services with type and port information
- **Namespace Overview** — Browse all namespaces

## Tech Stack

- **Backend**: Rust + Axum web framework
- **Database**: PostgreSQL (via sqlx)
- **K8s Client**: Direct REST API via reqwest
- **Deployment**: GitHub Actions CI/CD → ghcr.io → Kubernetes

## Quick Start

```bash
# Set environment variables
export DATABASE_URL="postgres://postgres:***@localhost:5432/k8s_platform"
export K8S_HOST="your-k8s-api-host"
export K8S_PORT="6443"
export K8S_TOKEN="your-service-account-token"

# Run
cargo run
```

Open http://localhost:8080

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| GET /health | Health check |
| GET /api/cluster/summary | Cluster overview |
| GET /api/pods | List pods |
| GET /api/deployments | List deployments |
| GET /api/services | List services |
| GET /api/namespaces | List namespaces |
| GET /api/namespaces/{ns}/pods/{pod}/logs | Pod logs |
