use super::*;
use std::collections::HashMap;

struct TestDirectory(PathBuf);

impl TestDirectory {
    fn new() -> Self {
        Self(game_engine::test_harness::temp_test_dir("terrain-resolver"))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).expect("remove terrain resolver fixture");
    }
}

struct FixtureResolver {
    files: HashMap<u32, (&'static str, Option<&'static [u8]>)>,
}

impl FixtureResolver {
    fn new() -> Self {
        Self {
            files: HashMap::from([
                (
                    778037,
                    (
                        "world/maps/azeroth/azeroth_32_50.adt",
                        Some(b"root fixture".as_slice()),
                    ),
                ),
                (
                    778040,
                    (
                        "world/maps/azeroth/azeroth_32_50_tex0.adt",
                        Some(b"texture fixture".as_slice()),
                    ),
                ),
                (
                    778038,
                    (
                        "world/maps/azeroth/azeroth_32_50_obj0.adt",
                        Some(b"object fixture".as_slice()),
                    ),
                ),
            ]),
        }
    }
}

impl AssetResolver for FixtureResolver {
    fn resolve_bytes(&self, fdid: u32) -> Option<Vec<u8>> {
        self.files.get(&fdid)?.1.map(<[u8]>::to_vec)
    }

    fn ensure_cached(&self, fdid: u32, path: &Path) -> Option<PathBuf> {
        if path.is_file() {
            return Some(path.to_path_buf());
        }
        let bytes = self.resolve_bytes(fdid)?;
        std::fs::create_dir_all(path.parent()?).ok()?;
        std::fs::write(path, bytes).ok()?;
        Some(path.to_path_buf())
    }

    fn resolve_path(&self, fdid: u32) -> Option<String> {
        self.files.get(&fdid).map(|(path, _)| path.to_string())
    }

    fn lookup_path(&self, path: &str) -> Option<u32> {
        self.files
            .iter()
            .find_map(|(&id, (candidate, _))| (*candidate == path).then_some(id))
    }
}

#[test]
fn uncached_tile_and_declared_companions_are_extracted_to_canonical_paths() {
    let directory = TestDirectory::new();
    let cache = directory.path().join("terrain");
    let resolver = FixtureResolver::new();
    let root = resolve_tile_path_with(&resolver, &cache, "azeroth", 32, 50).unwrap();
    assert_eq!(root, cache.join("778037.adt"));
    assert_eq!(std::fs::read(&root).unwrap(), b"root fixture");
    for (suffix, fdid, bytes) in [
        ("_tex0", 778040, b"texture fixture".as_slice()),
        ("_obj0", 778038, b"object fixture".as_slice()),
    ] {
        let path = resolve_companion_path_with(&resolver, &cache, &root, suffix)
            .unwrap()
            .unwrap();
        assert_eq!(path, cache.join(format!("{fdid}.adt")));
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}

#[test]
fn canonical_cached_files_remain_usable_without_archive_payload() {
    let directory = TestDirectory::new();
    let cache = directory.path();
    let mut resolver = FixtureResolver::new();
    for (&id, (_, bytes)) in &mut resolver.files {
        std::fs::write(cache.join(format!("{id}.adt")), bytes.unwrap()).unwrap();
        *bytes = None;
    }
    // Official resolution must not select a stale named copy over the FDID cache.
    std::fs::write(cache.join("azeroth_32_50.adt"), b"stale named copy").unwrap();
    let root = resolve_tile_path_with(&resolver, cache, "azeroth", 32, 50).unwrap();
    assert_eq!(std::fs::read(&root).unwrap(), b"root fixture");
    let companion = resolve_companion_path_with(&resolver, cache, &root, "_tex0")
        .unwrap()
        .unwrap();
    assert_eq!(std::fs::read(companion).unwrap(), b"texture fixture");
}

#[test]
fn missing_optional_entry_differs_from_declared_extraction_failure() {
    let directory = TestDirectory::new();
    let mut resolver = FixtureResolver::new();
    let root = directory.path().join("778037.adt");
    assert_eq!(
        resolve_companion_path_with(&resolver, directory.path(), &root, "_obj2").unwrap(),
        None
    );
    resolver.files.get_mut(&778040).unwrap().1 = None;
    let error =
        resolve_companion_path_with(&resolver, directory.path(), &root, "_tex0").unwrap_err();
    assert!(
        error.contains("778040") && error.contains("azeroth_32_50_tex0.adt"),
        "{error}"
    );
    resolver.files.get_mut(&778037).unwrap().1 = None;
    let error = resolve_tile_path_with(&resolver, directory.path(), "azeroth", 32, 50).unwrap_err();
    assert!(
        error.contains("778037") && error.contains("azeroth_32_50.adt"),
        "{error}"
    );
    let error = resolve_tile_path_with(&resolver, directory.path(), "missing", 1, 2).unwrap_err();
    assert!(
        error.contains("not in listfile") && error.contains("missing_1_2.adt"),
        "{error}"
    );
}

#[test]
fn named_official_input_extracts_a_declared_missing_sidecar() {
    let directory = TestDirectory::new();
    let cache = directory.path().join("cache");
    let root = directory.path().join("azeroth_32_50.adt");
    let resolver = FixtureResolver::new();
    let path = resolve_companion_path_with(&resolver, &cache, &root, "_obj0")
        .unwrap()
        .unwrap();
    assert_eq!(path, cache.join("778038.adt"));
    assert_eq!(std::fs::read(path).unwrap(), b"object fixture");
}

#[test]
fn unknown_numeric_root_reports_missing_reverse_lookup() {
    let directory = TestDirectory::new();
    let root = directory.path().join("99999.adt");
    let error =
        resolve_companion_path_with(&FixtureResolver::new(), directory.path(), &root, "_obj0")
            .unwrap_err();
    assert!(
        error.contains("99999") && error.contains("not in listfile"),
        "{error}"
    );
}

#[test]
fn explicit_named_sidecar_is_preserved_without_listfile_membership() {
    let directory = TestDirectory::new();
    let root = directory.path().join("custom_scene.adt");
    let sidecar = directory.path().join("custom_scene_obj0.adt");
    std::fs::write(&sidecar, b"authored local sidecar").unwrap();
    let resolver = FixtureResolver::new();
    let actual = resolve_companion_path_with(&resolver, directory.path(), &root, "_obj0")
        .unwrap()
        .unwrap();
    assert_eq!(actual, sidecar);
    assert_eq!(std::fs::read(actual).unwrap(), b"authored local sidecar");
}
