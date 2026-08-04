pub mod handlers;
pub mod k8s;

use sqlx::PgPool;

#[derive(Clone)]
#[allow(dead_code)]
pub struct AppState {
    pub pool: PgPool,
}
