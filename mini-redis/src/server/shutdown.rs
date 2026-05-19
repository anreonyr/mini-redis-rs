use std::sync::atomic::{AtomicBool, Ordering};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Check whether a graceful shutdown has been requested (via SHUTDOWN command or SIGINT).
pub fn is_requested() -> bool {
    SHUTDOWN.load(Ordering::Relaxed)
}

/// Request a graceful shutdown. The accept loop will stop accepting new
/// connections, and the main task will drain active connections before exiting.
pub fn request() {
    SHUTDOWN.store(true, Ordering::Relaxed);
}
