use std::future::Future;
use tracing::{error, info};

/// Spawn a long-lived actor and log how it ends. A panic is reported with the
/// actor's name instead of vanishing with the task; `on_exit` runs either way
/// so owners can drop stale handles (e.g. the field registry).
pub fn spawn_supervised<F, E>(name: String, actor: F, on_exit: E)
where
    F: Future<Output = ()> + Send + 'static,
    E: FnOnce() + Send + 'static,
{
    let task = tokio::spawn(actor);
    tokio::spawn(async move {
        match task.await {
            Ok(()) => info!(actor = %name, "Actor stopped"),
            Err(join_error) if join_error.is_panic() => {
                error!(actor = %name, "Actor panicked")
            }
            Err(join_error) => error!(actor = %name, error = %join_error, "Actor failed"),
        }
        on_exit();
    });
}
