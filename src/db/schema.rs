use sqlx::PgPool;

pub async fn run_migrations(pool: &PgPool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS clusters (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            kubeconfig_b64 TEXT NOT NULL,
            description TEXT DEFAULT '',
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create clusters table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            cluster_id UUID REFERENCES clusters(id) ON DELETE SET NULL,
            action VARCHAR(255) NOT NULL,
            resource_type VARCHAR(100),
            resource_name VARCHAR(255),
            namespace VARCHAR(100),
            details JSONB,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create audit_log table");

    tracing::info!("DB migrations complete");
}
