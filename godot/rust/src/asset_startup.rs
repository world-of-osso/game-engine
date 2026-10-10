//! Nonblocking startup boundary for process-wide CASC initialization.
use std::{
    path::PathBuf,
    thread::{self, JoinHandle},
};

pub(crate) struct AssetStartup {
    worker: Option<JoinHandle<Result<(), String>>>,
    ready: bool,
}

impl AssetStartup {
    pub(crate) fn start(data_root: PathBuf) -> Result<Self, String> {
        let mode = osso_asset_resolver::configure_runtime_mode_from_env()?;
        if mode == osso_asset_resolver::AssetRuntimeMode::ExtractedOnly {
            return Ok(Self {
                worker: None,
                ready: true,
            });
        }
        Self::spawn(move || crate::assets::creature::local_resolver(&data_root).initialize())
            .map_err(|error| format!("Cannot spawn CASC initialization worker: {error}"))
    }

    fn spawn(
        initialize: impl FnOnce() -> Result<(), String> + Send + 'static,
    ) -> std::io::Result<Self> {
        let worker = thread::Builder::new()
            .name("casc-startup".into())
            .spawn(initialize)?;
        Ok(Self {
            worker: Some(worker),
            ready: false,
        })
    }

    pub(crate) fn poll(&mut self) -> Option<Result<(), String>> {
        if std::mem::take(&mut self.ready) {
            return Some(Ok(()));
        }
        if !self.worker.as_ref()?.is_finished() {
            return None;
        }
        Some(
            self.worker
                .take()?
                .join()
                .map_err(|_| "CASC initialization worker panicked".to_string())
                .and_then(|result| result),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::AssetStartup;
    use std::{
        sync::mpsc,
        thread,
        time::{Duration, Instant},
    };

    fn finish(startup: &mut AssetStartup) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(result) = startup.poll() {
                return result;
            }
            assert!(Instant::now() < deadline, "startup worker did not finish");
            thread::yield_now();
        }
    }

    #[test]
    fn extracted_only_cold_start() {
        if std::env::var_os("SHIPPED_ASSETS_CHILD").is_some() {
            let root = std::path::PathBuf::from(std::env::var_os("SHIPPED_ASSETS_ROOT").unwrap());
            osso_asset_resolver::set_casc_access_hook(|error| panic!("{error}")).unwrap();
            let mut startup = AssetStartup::start(root.clone()).unwrap();
            assert_eq!(
                startup.poll(),
                Some(Ok(())),
                "extracted-only startup is immediately ready"
            );
            assert_eq!(startup.poll(), None);
            assert_shipped_fixtures(&root);
            assert_eq!(osso_asset_resolver::forbidden_casc_access_count(), 0);
            assert!(
                !root.join("cache").exists(),
                "CASC cache must not be created"
            );
            return;
        }
        let root = std::env::temp_dir().join(format!("shipped-assets-{}", std::process::id()));
        std::fs::create_dir_all(root.join("home")).unwrap();
        copy_shipped_fixtures(&root);
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "asset_startup::tests::extracted_only_cold_start",
                "--nocapture",
            ])
            .env_clear()
            .env("HOME", root.join("home"))
            .env("WOW_INSTALL_PATH", root.join("no-install"))
            .env("WOW_DATA_PATH", root.join("no-install/Data"))
            .env("GAME_ENGINE_ASSET_MODE", "extracted-only")
            .env("SHIPPED_ASSETS_CHILD", "1")
            .env("SHIPPED_ASSETS_ROOT", &root)
            .output()
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        assert!(
            result.status.success(),
            "cold start failed: {}{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
    }

    #[test]
    fn extracted_only_legacy_worker_missing_texture_completes() {
        legacy_texture_worker_process(false);
    }

    #[test]
    fn extracted_only_legacy_worker_present_texture_loads() {
        legacy_texture_worker_process(true);
    }

    fn legacy_texture_worker_process(present: bool) {
        let name = if present {
            "asset_startup::tests::extracted_only_legacy_worker_present_texture_loads"
        } else {
            "asset_startup::tests::extracted_only_legacy_worker_missing_texture_completes"
        };
        if std::env::var_os("LEGACY_TEXTURE_WORKER_CHILD").is_some() {
            assert_legacy_texture_worker(present);
            return;
        }
        let root =
            std::env::temp_dir().join(format!("legacy-texture-{}-{present}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", name, "--nocapture"])
            .env_clear()
            .env("GAME_ENGINE_ASSET_MODE", "extracted-only")
            .env("LEGACY_TEXTURE_WORKER_CHILD", "1")
            .env("LEGACY_TEXTURE_ROOT", &root)
            .output()
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!String::from_utf8_lossy(&output.stderr).contains("panicked"));
    }

    fn assert_legacy_texture_worker(present: bool) {
        use game_engine_core::asset_loader::{AssetLoader, LoadState, Priority};
        use osso_asset_resolver::{AssetIdentity, AssetResolverConfig, CascListfileResolver};
        use std::time::{Duration, Instant};
        osso_asset_resolver::configure_runtime_mode_from_env().unwrap();
        let root = std::path::PathBuf::from(std::env::var_os("LEGACY_TEXTURE_ROOT").unwrap());
        let identity =
            AssetIdentity::new("wow_classic_beta", "00000000000000000000000000000000").unwrap();
        let fdid = 896467;
        let destination = root.join(format!("textures/{fdid}.blp"));
        let expected = identity.asset_path(&root, format!("textures/{fdid}.blp"));
        if present {
            let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/textures/896467.blp");
            std::fs::create_dir_all(expected.parent().unwrap()).unwrap();
            std::fs::copy(source, &expected).unwrap();
        }
        let resolver = CascListfileResolver::new(
            AssetResolverConfig::new()
                .with_data_root(&root)
                .with_shared_data_root(&root)
                .with_identity(identity),
        );
        let checked_error = resolver.ensure_cached_checked(fdid, &destination).err();
        let mut loader = AssetLoader::new("legacy-texture", 1, move |&key: &u32| {
            let path = resolver
                .ensure_cached(key, &destination)
                .ok_or_else(|| "legacy cache returned None".to_owned())?;
            let bytes = std::fs::read(path).map_err(|error| error.to_string())?;
            game_engine_core::blp::decode_rgba(&bytes)
        });
        assert!(loader.request(fdid, Priority::Now));
        let deadline = Instant::now() + Duration::from_secs(2);
        let completion = loop {
            let mut results = loader.poll();
            if let Some(result) = results.pop() {
                break result;
            }
            assert!(
                Instant::now() < deadline,
                "missing completion; state {:?}, loading {}",
                loader.state(&fdid),
                loader.loading()
            );
            std::thread::sleep(Duration::from_millis(1));
        };
        assert_eq!(completion.0, fdid);
        if present {
            let image = completion.1.unwrap();
            assert!(image.width > 0 && image.height > 0);
        } else {
            assert_eq!(completion.1.err().unwrap(), checked_error.unwrap());
        }
        assert_eq!(loader.state(&fdid), Some(LoadState::Done));
        assert_eq!(loader.loading(), 0);
        assert!(!loader.request(fdid, Priority::Now));
        assert!(loader.poll().is_empty());
        assert_eq!(osso_asset_resolver::forbidden_casc_access_count(), 0);
    }

    const FIXTURES: &[(&str, u32)] = &[
        ("textures/896467.blp", 896467),
        ("models/143187.m2", 143187),
        ("models/14318700.skin", 143187),
        ("models/108121.wmo", 108121),
        ("models/108122.wmo", 108122),
        ("terrain/7199999.adt", 7199999),
        ("dbfilesclient/1308499.db2", 1308499),
        ("dbfilesclient/1284822.db2", 1284822),
        ("sounds/spells/632305.ogg", 632305),
    ];

    fn copy_shipped_fixtures(root: &std::path::Path) {
        let source = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
        for (relative, _) in FIXTURES {
            let target = root.join(relative);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::copy(source.join(relative), target).unwrap();
        }
    }

    fn assert_shipped_fixtures(root: &std::path::Path) {
        use game_engine_core::{adt, blp, ground_effect_data, wmo};
        let resolver = crate::assets::creature::local_resolver(root);
        assert_eq!(
            resolver.runtime_mode(),
            osso_asset_resolver::AssetRuntimeMode::ExtractedOnly
        );
        resolver.initialize().unwrap();
        for (relative, fdid) in FIXTURES {
            let expected = root.join(relative);
            assert_eq!(
                resolver.ensure_cached_checked(*fdid, &expected).unwrap(),
                expected
            );
            assert_eq!(resolver.ensure_cached(*fdid, &expected).unwrap(), expected);
        }
        let read = |relative: &str| std::fs::read(root.join(relative)).unwrap();
        let image = blp::decode_rgba(&read("textures/896467.blp")).unwrap();
        assert!(image.width > 0 && image.height > 0);
        crate::assets::read_model_file(&root.join("models/143187.m2")).unwrap();
        wmo::parse_root(&read("models/108121.wmo")).unwrap();
        wmo::parse_group(&read("models/108122.wmo")).unwrap();
        adt::parse_root(&read("terrain/7199999.adt")).unwrap();
        assert!(
            !ground_effect_data::parse_ground_effect_entries(&read("dbfilesclient/1308499.db2"))
                .unwrap()
                .is_empty()
        );
        assert!(
            !ground_effect_data::parse_terrain_type_sounds(&read("dbfilesclient/1284822.db2"))
                .unwrap()
                .is_empty()
        );
        assert!(
            !crate::spell_assets::load_sound(&resolver, root, 632305)
                .unwrap()
                .is_empty()
        );
        let missing = root.join("textures/4294967295.blp");
        let error = resolver
            .ensure_cached_checked(u32::MAX, &missing)
            .unwrap_err();
        assert!(
            error.contains("extracted-only")
                && error.contains("FDID 4294967295")
                && error.contains(&missing.display().to_string()),
            "{error}"
        );
        let legacy = std::panic::catch_unwind(|| resolver.ensure_cached(u32::MAX, &missing));
        assert!(
            legacy.is_err(),
            "legacy Option API must not silently omit required assets"
        );
    }

    #[test]
    fn low_level_tripwire_cannot_be_hidden_by_option_or_logging() {
        if std::env::var_os("CASC_TRIPWIRE_CHILD").is_some() {
            osso_asset_resolver::configure_runtime_mode_from_env().unwrap();
            osso_asset_resolver::set_casc_access_hook(|error| panic!("{error}")).unwrap();
            // Deliberately swallow the low-level panic: the counter still fails acceptance.
            let _ = std::panic::catch_unwind(osso_asset_resolver::wow_install_path);
            assert_eq!(
                osso_asset_resolver::forbidden_casc_access_count(),
                0,
                "forbidden CASC entry was swallowed"
            );
            return;
        }
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "asset_startup::tests::low_level_tripwire_cannot_be_hidden_by_option_or_logging",
                "--nocapture",
            ])
            .env_clear()
            .env("GAME_ENGINE_ASSET_MODE", "extracted-only")
            .env("CASC_TRIPWIRE_CHILD", "1")
            .output()
            .unwrap();
        let error = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success());
        assert!(
            error.contains("extracted-only forbids CASC access")
                && error.contains("forbidden CASC entry was swallowed"),
            "{error}"
        );
    }

    #[test]
    fn pending_initialization_does_not_block_caller_or_run_on_its_thread() {
        let caller = thread::current().id();
        let (started, observed) = mpsc::channel();
        let (release, gate) = mpsc::channel();
        let mut startup = AssetStartup::spawn(move || {
            started.send(thread::current().id()).unwrap();
            gate.recv().unwrap();
            Ok(())
        })
        .unwrap();
        let worker = observed.recv_timeout(Duration::from_secs(5));
        // Release even if the assertion fails, so a test failure cannot orphan a worker.
        let pending = startup.poll();
        let _ = release.send(());
        assert_ne!(worker.expect("initializer executes"), caller);
        assert_eq!(pending, None);
        assert_eq!(finish(&mut startup), Ok(()));
        assert_eq!(startup.poll(), None, "completion is delivered once");
    }

    #[test]
    fn initialization_failure_reaches_startup_caller() {
        let mut startup = AssetStartup::spawn(|| Err("CASC open: denied".into())).unwrap();
        assert_eq!(finish(&mut startup), Err("CASC open: denied".into()));
    }

    #[test]
    fn initialization_panic_is_an_explicit_startup_failure() {
        let mut startup = AssetStartup::spawn(|| panic!("broken initialization")).unwrap();
        assert_eq!(
            finish(&mut startup),
            Err("CASC initialization worker panicked".into())
        );
    }
}
