//! Real file acquisition/parsing and BLP decode boundaries; no GPU allocation.
use super::*;
use osso_asset_resolver::AssetIdentity;
use std::sync::atomic::{AtomicU64, Ordering};

const MODEL_FDID: u32 = 126278;
const RETAIL_KEY: &str = "dcfc90fffd79ba00406ae46f5f657592";
const FOREVER_KEY: &str = "e8dd824cf6c3d96cd01f804ca2ea5a63";

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        let root = std::env::temp_dir().join(format!(
            "native-model-identity-{}-{}",
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
                .with_cache_root(self.0.join("cache"))
                .with_identity(identity.clone()),
        )
    }
    fn seed(&self, identity: &AssetIdentity, relative: &str, bytes: &[u8]) -> PathBuf {
        let path = identity.asset_path(&self.0, relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, bytes).unwrap();
        path
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut bytes = tag.to_vec();
    bytes.extend((body.len() as u32).to_le_bytes());
    bytes.extend(body);
    bytes
}

fn model_with_vertex(x: f32) -> Vec<u8> {
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models/126278.m2");
    let bytes = fs::read(source).unwrap();
    let mut model = Vec::new();
    let mut offset = 0;
    while offset + 8 <= bytes.len() {
        let tag: [u8; 4] = bytes[offset..offset + 4].try_into().unwrap();
        let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().unwrap()) as usize;
        let mut body = bytes[offset + 8..offset + 8 + size].to_vec();
        if &tag == b"MD21" {
            let vertex = u32::from_le_bytes(body[64..68].try_into().unwrap()) as usize;
            body[vertex..vertex + 4].copy_from_slice(&x.to_le_bytes());
        }
        // Animation acquisition has its own concrete chain fixture below.
        if &tag != b"AFID" {
            model.extend(chunk(&tag, &body));
        }
        offset += size + 8;
    }
    model
}

#[test]
fn model_asset_parsed_same_fdid_models_keep_both_product_namespaces() {
    let fixture = Fixture::new();
    let skin =
        fs::read(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/models/12627800.skin"))
            .unwrap();
    let retail = AssetIdentity::new("wow", RETAIL_KEY).unwrap();
    let forever = AssetIdentity::new("wow_classic_beta", FOREVER_KEY).unwrap();
    for (identity, x) in [(&retail, 1.25), (&forever, 7.5)] {
        fixture.seed(identity, "models/126278.m2", &model_with_vertex(x));
        fixture.seed(identity, "models/12627800.skin", &skin);
    }
    for (identity, x) in [(&retail, 1.25), (&forever, 7.5), (&retail, 1.25)] {
        let cached = load_model_files(&fixture.resolver(identity), &fixture.0, MODEL_FDID).unwrap();
        assert_eq!(cached.model.vertices[0].position[0], x);
        assert_eq!(
            cached.path,
            identity.asset_path(&fixture.0, "models/126278.m2")
        );
    }
}

#[test]
fn model_asset_sfid_skid_and_external_animation_acquisition_inherit_the_model_identity() {
    let fixture = Fixture::new();
    let retail = AssetIdentity::new("wow", RETAIL_KEY).unwrap();
    let forever = AssetIdentity::new("wow_classic_beta", FOREVER_KEY).unwrap();
    let mut references = chunk(b"MD21", b"MD20");
    references.extend(chunk(b"SFID", &1001u32.to_le_bytes()));
    references.extend(chunk(b"SKID", &1002u32.to_le_bytes()));
    let mut afid = 0u16.to_le_bytes().to_vec();
    afid.extend(0u16.to_le_bytes());
    afid.extend(1003u32.to_le_bytes());
    let skeleton = chunk(b"AFID", &afid);
    for (identity, receipt) in [
        (&retail, b"retail".as_slice()),
        (&forever, b"forever".as_slice()),
    ] {
        fixture.seed(identity, "models/126278.m2", &references);
        fixture.seed(identity, "models/12627800.skin", receipt);
        fixture.seed(identity, "models/126278.skel", &skeleton);
        fixture.seed(identity, "models/1003.anim", receipt);
        let path = cache_model_files(&fixture.resolver(identity), &fixture.0, MODEL_FDID).unwrap();
        assert_eq!(path, identity.asset_path(&fixture.0, "models/126278.m2"));
        let dir = path.parent().unwrap();
        assert_eq!(fs::read(dir.join("12627800.skin")).unwrap(), receipt);
        assert_eq!(fs::read(dir.join("126278.skel")).unwrap(), skeleton);
        assert_eq!(fs::read(dir.join("1003.anim")).unwrap(), receipt);
    }
}

fn solid_palettized_blp(red: u8, green: u8) -> Vec<u8> {
    const HEADER_SIZE: usize = 148;
    const PALETTE_SIZE: usize = 256 * 4;
    const MIP_OFFSET: usize = HEADER_SIZE + PALETTE_SIZE;
    let mut bytes = vec![0; MIP_OFFSET];
    bytes[..4].copy_from_slice(b"BLP2");
    bytes[4..8].copy_from_slice(&1u32.to_le_bytes());
    bytes[8] = 1; // Palettized encoding, supported by the GPU decoder.
    bytes[9] = 8; // Eight-bit alpha.
    bytes[12..16].copy_from_slice(&1u32.to_le_bytes());
    bytes[16..20].copy_from_slice(&1u32.to_le_bytes());
    bytes[20..24].copy_from_slice(&(MIP_OFFSET as u32).to_le_bytes());
    bytes[84..88].copy_from_slice(&2u32.to_le_bytes());
    bytes[HEADER_SIZE..HEADER_SIZE + 4].copy_from_slice(&[0, green, red, 255]);
    bytes.extend([0, 255]); // Palette index zero, opaque alpha.
    bytes
}

#[test]
fn model_asset_decoding_same_texture_fdid_preserves_both_products_pixels() {
    let fixture = Fixture::new();
    let retail = AssetIdentity::new("wow", RETAIL_KEY).unwrap();
    let forever = AssetIdentity::new("wow_classic_beta", FOREVER_KEY).unwrap();
    let fdid = 2_000_000_101;
    for (identity, red, green) in [(&retail, 11, 22), (&forever, 33, 44)] {
        fixture.seed(
            identity,
            &format!("textures/{fdid}.blp"),
            &solid_palettized_blp(red, green),
        );
        let decoded =
            decode_new_textures(&identity.asset_root(&fixture.0), &BTreeSet::from([fdid])).unwrap();
        assert_eq!(
            decoded.len(),
            1,
            "{} lost its decoded texture",
            identity.product()
        );
        assert_eq!(decoded[0].1.data, [red, green, 0, 255]);
        assert_eq!(
            decoded[0].0.dir,
            identity.asset_root(&fixture.0).join("textures")
        );
        assert_eq!(decoded[0].0.fdid, fdid);
    }
}

#[test]
fn model_asset_missing_matching_texture_errors_and_does_not_poison_a_later_decode() {
    let fixture = Fixture::new();
    let retail = AssetIdentity::new("wow", RETAIL_KEY).unwrap();
    let fdid = 2_000_000_102;
    let root = retail.asset_root(&fixture.0);
    let requested = BTreeSet::from([fdid]);
    assert!(decode_new_textures(&root, &requested).is_err());
    fixture.seed(
        &retail,
        &format!("textures/{fdid}.blp"),
        &solid_palettized_blp(55, 66),
    );
    let decoded = decode_new_textures(&root, &requested).unwrap();
    assert_eq!(decoded.len(), 1);
    assert_eq!(decoded[0].1.data, [55, 66, 0, 255]);
}
