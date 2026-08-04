use sqlx::PgPool;

pub async fn run_migrations(pool: &PgPool) {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS audit_log (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
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

    sqlx::query("CREATE INDEX IF NOT EXISTS idx_audit_log_created_at ON audit_log(created_at)")
        .execute(pool)
        .await
        .expect("Failed to create audit_log index");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS cluster_config (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            name VARCHAR(255) NOT NULL,
            api_url VARCHAR(512) NOT NULL,
            kubeconfig_b64 TEXT,
            is_active BOOLEAN NOT NULL DEFAULT false,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create cluster_config table");

    sqlx::query(
        "CREATE TABLE IF NOT EXISTS user_preferences (
            id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
            pref_key VARCHAR(255) NOT NULL UNIQUE,
            pref_value JSONB NOT NULL,
            created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
            updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
        )",
    )
    .execute(pool)
    .await
    .expect("Failed to create user_preferences table");

    tracing::info!("Database migrations completed");
}
