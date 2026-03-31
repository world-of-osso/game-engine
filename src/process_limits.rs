pub fn apply_resource_limits() {
    let Some(max_mem_gb) = read_max_memory_gb() else {
        eprintln!("Resource limits: RLIMIT_AS disabled by default");
        return;
    };
    let max_mem_bytes = max_mem_gb * 1024 * 1024 * 1024;
    let mem_limit = libc::rlimit {
        rlim_cur: max_mem_bytes,
        rlim_max: max_mem_bytes,
    };
    let rc = unsafe { libc::setrlimit(libc::RLIMIT_AS, &mem_limit) };
    if rc != 0 {
        eprintln!(
            "Failed to set RLIMIT_AS to {max_mem_gb}GB: {}",
            std::io::Error::last_os_error()
        );
        return;
    }
    log_current_limit();
}

fn read_max_memory_gb() -> Option<u64> {
    std::env::var("GAME_ENGINE_MAX_MEM_GB")
        .ok()
        .and_then(|s| s.parse().ok())
}

fn log_current_limit() {
    let mut current = libc::rlimit {
        rlim_cur: 0,
        rlim_max: 0,
    };
    let rc = unsafe { libc::getrlimit(libc::RLIMIT_AS, &mut current) };
    if rc != 0 {
        eprintln!(
            "Set RLIMIT_AS but failed to read it back: {}",
            std::io::Error::last_os_error()
        );
        return;
    }
    eprintln!(
        "Resource limits: {} bytes soft, {} bytes hard",
        current.rlim_cur, current.rlim_max
    );
}
