//! Explicit authored identities exercise actual resolver file IO in one process.
//! Receipts are not M2 parsing or GPU/rendering evidence.
use osso_asset_resolver::{AssetIdentity, AssetResolverConfig, CascListfileResolver};
use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};

const CASES: [(u32, u32); 2] = [(139403, 1100087), (139409, 1100258)];
const RETAIL_KEY: &str = "dcfc90fffd79ba00406ae46f5f657592";
const FOREVER_KEY: &str = "e8dd824cf6c3d96cd01f804ca2ea5a63";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "m2isolation-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&root).unwrap();
        Self(root)
    }
    fn resolver(&self, identity: &AssetIdentity) -> CascListfileResolver {
        CascListfileResolver::new(
            AssetResolverConfig::new()
                .with_data_root(&self.0)
                .with_shared_data_root(&self.0)
                .with_cache_root(self.0.join("resolver-cache"))
                .with_identity(identity.clone()),
        )
    }
    fn seed(&self, path: &std::path::Path, bytes: &[u8]) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, bytes).unwrap();
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}
fn payload(identity: &AssetIdentity, fdid: u32) -> Vec<u8> {
    format!(
        "{}:{}:M2 receipt for FDID {fdid}",
        identity.product(),
        identity.build_key()
    )
    .into_bytes()
}

#[test]
fn forever_model_requests_do_not_reuse_retail_cached_bytes() {
    let fixture = Fixture::new();
    let retail = AssetIdentity::new("wow", RETAIL_KEY).unwrap();
    let forever = AssetIdentity::new("wow_classic_beta", FOREVER_KEY).unwrap();
    for (_, fdid) in CASES {
        let relative = format!("models/{fdid}.m2");
        fixture.seed(
            &fixture.0.join(&relative),
            b"unqualified legacy Retail decoy",
        );
        for identity in [&retail, &forever] {
            fixture.seed(
                &identity.asset_path(&fixture.0, &relative),
                &payload(identity, fdid),
            );
        }
    }
    let retail_resolver = fixture.resolver(&retail);
    let forever_resolver = fixture.resolver(&forever);
    // Alternate both namespaces, then return to Retail: no environment switch or
    // child process can mask a shared-state collision.
    for (identity, resolver) in [
        (&retail, &retail_resolver),
        (&forever, &forever_resolver),
        (&retail, &retail_resolver),
    ] {
        for (display, fdid) in CASES {
            let relative = format!("models/{fdid}.m2");
            let actual = resolver
                .ensure_cached_checked(fdid, &fixture.0.join(&relative))
                .unwrap();
            assert_eq!(
                fs::read(&actual).unwrap(),
                payload(identity, fdid),
                "display {display}"
            );
            assert_eq!(actual, identity.asset_path(&fixture.0, relative));
        }
    }
}

#[test]
fn legacy_bytes_do_not_satisfy_a_missing_authored_identity() {
    let fixture = Fixture::new();
    let identity =
        AssetIdentity::new("wow_classic_beta", "00000000000000000000000000000000").unwrap();
    let path = fixture.0.join("models/1100087.m2");
    fixture.seed(&path, b"unqualified legacy Retail decoy");
    let error = fixture
        .resolver(&identity)
        .ensure_cached_checked(1100087, &path)
        .unwrap_err();
    assert!(error.contains(identity.build_key()), "{error}");
    assert_eq!(fs::read(path).unwrap(), b"unqualified legacy Retail decoy");
}
