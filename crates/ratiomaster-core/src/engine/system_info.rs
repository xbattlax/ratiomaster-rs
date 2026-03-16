/// System information logging on engine startup.
///
/// Logs OS, hostname, version, working directory, and CPU count
/// for diagnostic and debugging purposes.
use tracing::info;

/// Logs system information at startup.
pub fn log_system_info() {
    info!("OS: {}", std::env::consts::OS);
    info!("arch: {}", std::env::consts::ARCH);

    if let Ok(hostname) = hostname() {
        info!("hostname: {hostname}");
    }

    info!("version: {}", env!("CARGO_PKG_VERSION"));

    if let Ok(cwd) = std::env::current_dir() {
        info!("working directory: {}", cwd.display());
    }

    info!("CPU count: {}", num_cpus());
}

/// Returns a system info summary string (for non-logging use).
pub fn system_info_string() -> String {
    let mut parts = Vec::new();
    parts.push(format!("OS: {}", std::env::consts::OS));
    parts.push(format!("arch: {}", std::env::consts::ARCH));

    if let Ok(h) = hostname() {
        parts.push(format!("hostname: {h}"));
    }

    parts.push(format!("version: {}", env!("CARGO_PKG_VERSION")));

    if let Ok(cwd) = std::env::current_dir() {
        parts.push(format!("working directory: {}", cwd.display()));
    }

    parts.push(format!("CPU count: {}", num_cpus()));
    parts.join(", ")
}

fn hostname() -> Result<String, std::io::Error> {
    Ok(gethostname::gethostname().to_string_lossy().into_owned())
}

fn num_cpus() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_info_string_not_empty() {
        let info = system_info_string();
        assert!(!info.is_empty());
        assert!(info.contains("OS:"));
        assert!(info.contains("CPU count:"));
    }

    #[test]
    fn num_cpus_positive() {
        assert!(num_cpus() >= 1);
    }

    #[test]
    fn hostname_returns_something() {
        // Should work on any CI/dev machine
        let h = hostname();
        assert!(h.is_ok());
        assert!(!h.unwrap().is_empty());
    }
}
