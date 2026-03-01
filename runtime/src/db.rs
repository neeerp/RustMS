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
