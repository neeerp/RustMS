use crate::error::RuntimeError;

/// Execute a blocking database operation on the Tokio blocking thread pool.
pub async fn spawn_db<F, T>(f: F) -> Result<T, RuntimeError>
where
    F: FnOnce() -> Result<T, db::Error> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(|e| RuntimeError::Handler(format!("Task join error: {}", e)))?
        .map_err(RuntimeError::Database)
}

/// Apply pending migrations before serving traffic, retrying while the
/// database is still starting up.
pub async fn prepare_database() -> Result<(), RuntimeError> {
    const ATTEMPTS: u32 = 30;
    for attempt in 1..=ATTEMPTS {
        match spawn_db(db::run_migrations).await {
            Ok(applied) => {
                tracing::info!(applied, "Database migrations are up to date");
                return Ok(());
            }
            Err(RuntimeError::Database(db::Error::Unavailable(reason))) if attempt < ATTEMPTS => {
                tracing::warn!(attempt, reason, "Database unavailable; retrying");
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            Err(error) => return Err(error),
        }
    }
    unreachable!("the final attempt always returns")
}
