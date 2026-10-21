use std::time::{Duration, SystemTime};

/// Half of Docker's default soft limit of 1024; a healthy process holds ~15.
const MAX_OPEN_FDS: usize = 512;

/// Records a successful iteration for the healthcheck.
pub fn record_success(health_file: &str) -> std::io::Result<()> {
    std::fs::write(health_file, chrono::Utc::now().to_rfc3339())
}

/// Entry point for `mqttbot healthcheck`, run by the container healthcheck.
/// Checks that the main process (PID 1 in the container) published recently
/// and isn't leaking file descriptors. Returns the process exit code.
pub fn check(health_file: &str, max_age: Duration) -> i32 {
    match check_inner(health_file, max_age) {
        Ok(msg) => {
            println!("ok: {}", msg);
            0
        }
        Err(msg) => {
            println!("unhealthy: {}", msg);
            1
        }
    }
}

fn check_inner(health_file: &str, max_age: Duration) -> Result<String, String> {
    let modified = std::fs::metadata(health_file)
        .and_then(|m| m.modified())
        .map_err(|e| format!("cannot read {}: {}", health_file, e))?;
    let age = SystemTime::now()
        .duration_since(modified)
        .unwrap_or_default();
    if age > max_age {
        return Err(format!(
            "last successful publish {}s ago (max {}s)",
            age.as_secs(),
            max_age.as_secs()
        ));
    }

    let fds = std::fs::read_dir("/proc/1/fd")
        .map_err(|e| format!("cannot read /proc/1/fd: {}", e))?
        .count();
    if fds >= MAX_OPEN_FDS {
        return Err(format!(
            "{} open file descriptors (max {})",
            fds, MAX_OPEN_FDS
        ));
    }

    Ok(format!(
        "last successful publish {}s ago, {} open file descriptors",
        age.as_secs(),
        fds
    ))
}
