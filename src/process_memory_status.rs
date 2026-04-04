#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ProcessMemoryKb {
    pub rss_kb: u64,
    pub anon_kb: u64,
    pub data_kb: u64,
}

#[cfg(target_os = "linux")]
pub fn current_process_memory_kb() -> ProcessMemoryKb {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return ProcessMemoryKb::default();
    };
    let mut memory = ProcessMemoryKb::default();
    for line in status.lines() {
        if let Some(value) = line.strip_prefix("VmRSS:") {
            memory.rss_kb = parse_kb_field(value);
        } else if let Some(value) = line.strip_prefix("RssAnon:") {
            memory.anon_kb = parse_kb_field(value);
        } else if let Some(value) = line.strip_prefix("VmData:") {
            memory.data_kb = parse_kb_field(value);
        }
    }
    memory
}

#[cfg(not(target_os = "linux"))]
pub fn current_process_memory_kb() -> ProcessMemoryKb {
    ProcessMemoryKb::default()
}

fn parse_kb_field(value: &str) -> u64 {
    value
        .split_whitespace()
        .next()
        .and_then(|part| part.parse().ok())
        .unwrap_or(0)
}
