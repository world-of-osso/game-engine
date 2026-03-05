use std::collections::HashMap;

/// WASM addon host. Loads .wasm files and provides game-api functions.
///
/// This is a stub implementation that provides the correct API shape
/// without actual WASM execution. A real implementation would use
/// wasmi or wasmtime to instantiate and run WASM modules.
#[derive(Default)]
pub struct WasmHost {
    /// Loaded addon instances, keyed by addon name.
    addons: HashMap<String, AddonInstance>,
}

/// Represents a loaded WASM addon instance.
struct AddonInstance {
    /// Raw WASM bytes (retained for hot-reload comparison).
    _wasm_bytes: Vec<u8>,
}

impl WasmHost {
    pub fn new() -> Self {
        Self::default()
    }

    /// Load a WASM addon from bytes.
    ///
    /// TODO: Parse and instantiate the WASM module via wasmi/wasmtime.
    /// TODO: Link host functions (create_frame, set_text, etc.).
    /// TODO: Call on_load entry point if exported.
    pub fn load_addon(&mut self, name: &str, wasm_bytes: &[u8]) -> Result<(), String> {
        if wasm_bytes.is_empty() {
            return Err("empty WASM bytes".to_string());
        }
        self.addons.insert(
            name.to_string(),
            AddonInstance {
                _wasm_bytes: wasm_bytes.to_vec(),
            },
        );
        Ok(())
    }

    /// Unload an addon by name.
    ///
    /// TODO: Call on_unload entry point before dropping.
    pub fn unload_addon(&mut self, name: &str) {
        self.addons.remove(name);
    }

    /// List loaded addon names.
    pub fn loaded_addons(&self) -> Vec<&str> {
        self.addons.keys().map(|s| s.as_str()).collect()
    }

    /// Check if an addon is loaded.
    pub fn is_loaded(&self, name: &str) -> bool {
        self.addons.contains_key(name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_host_has_no_addons() {
        let host = WasmHost::new();
        assert!(host.loaded_addons().is_empty());
    }

    #[test]
    fn load_addon_succeeds() {
        let mut host = WasmHost::new();
        let result = host.load_addon("test-addon", &[0x00, 0x61, 0x73, 0x6D]);
        assert!(result.is_ok());
        assert!(host.is_loaded("test-addon"));
    }

    #[test]
    fn load_empty_bytes_fails() {
        let mut host = WasmHost::new();
        let result = host.load_addon("bad", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn unload_addon_removes_it() {
        let mut host = WasmHost::new();
        host.load_addon("a", &[1]).unwrap();
        assert!(host.is_loaded("a"));
        host.unload_addon("a");
        assert!(!host.is_loaded("a"));
    }

    #[test]
    fn unload_nonexistent_is_noop() {
        let mut host = WasmHost::new();
        host.unload_addon("missing"); // should not panic
    }

    #[test]
    fn loaded_addons_lists_all() {
        let mut host = WasmHost::new();
        host.load_addon("alpha", &[1]).unwrap();
        host.load_addon("beta", &[2]).unwrap();
        let mut names = host.loaded_addons();
        names.sort();
        assert_eq!(names, vec!["alpha", "beta"]);
    }
}
