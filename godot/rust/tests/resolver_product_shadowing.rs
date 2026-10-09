//! Pending design regression: cached files must not cross product boundaries.
//! This exercises real resolver IO, not M2 parsing or native GPU publication.
use std::{env, fs, path::PathBuf, process::Command};

use osso_asset_resolver::{AssetResolverConfig, CascListfileResolver};

const CHILD_ROOT: &str = "M2_ISOLATION_TEST_ROOT";
const CASES: [(u32, u32); 2] = [(139403, 1100087), (139409, 1100258)];

fn payload(product: &str, fdid: u32) -> Vec<u8> {
    format!("{product}:M2 cache receipt for FDID {fdid}").into_bytes()
}

fn read_and_assert_selected_product(root: PathBuf) {
    let product = env::var("WOW_PRODUCT").expect("child product");
    let resolver = CascListfileResolver::new(
        AssetResolverConfig::new()
            .with_data_root(&root)
            .with_shared_data_root(&root)
            .with_cache_root(root.join("resolver-cache")),
    );
    let mut mismatches = Vec::new();
    for (display, fdid) in CASES {
        let destination = root.join("models").join(format!("{fdid}.m2"));
        let path = resolver
            .ensure_cached(fdid, &destination)
            .expect("cached file");
        let actual = fs::read(&path).expect("cached bytes");
        let expected = payload(&product, fdid);
        if actual != expected {
            mismatches.push(format!(
                "display {display}, FDID {fdid}: requested {product}, got {:?} at {}",
                String::from_utf8_lossy(&actual),
                path.display()
            ));
        }
    }
    assert!(mismatches.is_empty(), "{}", mismatches.join("\n"));
}

#[test]
#[ignore = "RED: product-isolated resolver design not approved; run explicitly with --ignored"]
fn forever_model_requests_do_not_reuse_retail_cached_bytes() {
    if let Some(root) = env::var_os(CHILD_ROOT) {
        read_and_assert_selected_product(PathBuf::from(root));
        return;
    }
    let root = env::temp_dir().join(format!("m2isolation-{}", std::process::id()));
    fs::create_dir(&root).expect("fresh fixture directory");
    fs::create_dir(root.join("models")).unwrap();
    for (_, fdid) in CASES {
        fs::write(
            root.join("models").join(format!("{fdid}.m2")),
            payload("wow", fdid),
        )
        .unwrap();
    }
    // Fresh processes isolate WOW_PRODUCT and the resolver's process-global state.
    // The Retail control must pass before testing the Forever request.
    let mut results = Vec::new();
    for product in ["wow", "wow_classic_beta"] {
        let output = Command::new(env::current_exe().unwrap())
            .args([
                "--exact",
                "forever_model_requests_do_not_reuse_retail_cached_bytes",
                "--ignored",
                "--nocapture",
            ])
            .env(CHILD_ROOT, &root)
            .env("WOW_PRODUCT", product)
            .output()
            .expect("spawn isolated resolver test");
        eprintln!("{product}: {}", String::from_utf8_lossy(&output.stdout));
        eprintln!("{}", String::from_utf8_lossy(&output.stderr));
        results.push(output.status.success());
    }
    fs::remove_dir_all(root).expect("remove fixture");
    assert!(results[0], "Retail control failed");
    assert!(
        results[1],
        "Forever request reused Retail bytes for both displays"
    );
}
