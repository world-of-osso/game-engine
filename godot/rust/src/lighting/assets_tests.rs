use std::{
    fs,
    path::{Path, PathBuf},
};

use super::assets::LightingCatalog;

struct LightingFixture(PathBuf);

impl LightingFixture {
    fn new(name: &str) -> Self {
        let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
        let root =
            std::env::temp_dir().join(format!("zephras-lighting-{name}-{}", std::process::id()));
        copy_retail_fixture(&source, &root);
        write_forever_fixture(&source, &root.join("db2/1.60.1.70205"));
        Self(root)
    }
}

impl Drop for LightingFixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

fn copy_retail_fixture(source: &Path, root: &Path) {
    fs::create_dir_all(root.join("db2/12.1.0.69933")).unwrap();
    fs::create_dir_all(root.join("db2/1.60.1.70205")).unwrap();
    for directory in ["models", "textures"] {
        std::os::unix::fs::symlink(source.join(directory), root.join(directory)).unwrap();
    }
    for table in ["Light", "LightData", "ZoneLight", "ZoneLightPoint"] {
        fs::copy(
            source.join(format!("{table}.csv")),
            root.join(format!("{table}.csv")),
        )
        .unwrap();
    }
    for table in ["LightParams", "LightSkybox", "Map"] {
        let relative = format!("db2/12.1.0.69933/{table}.csv");
        fs::copy(source.join(&relative), root.join(&relative)).unwrap();
    }
}

fn write_forever_fixture(source: &Path, forever: &Path) {
    fs::write(forever.join("Map.csv"), "ID,Directory,MapName_lang,WdtFileDataID\n0,Azeroth,Forever Azeroth,1\n2991,2991,Zephras Isle,7198644\n").unwrap();
    let light = fs::read_to_string(source.join("Light.csv")).unwrap();
    let header = light.lines().next().unwrap();
    // Same LightParams ID as retail deliberately tests product namespace isolation.
    fs::write(forever.join("Light.csv"), format!("{header}\n1,0,0,0,0,0,2991,12,12,12,12,12,12,0,0\n2,0,0,0,0,0,0,12,12,12,12,12,12,0,0\n")).unwrap();
    let data = fs::read_to_string(source.join("LightData.csv")).unwrap();
    fs::write(forever.join("LightData.csv"), encode_green_keyframes(&data)).unwrap();
    fs::write(forever.join("LightParams.csv"), "ID,WaterShallowAlpha,WaterDeepAlpha,OceanShallowAlpha,OceanDeepAlpha,Flags,LightSkyboxID\n12,0.5,1,0.75,1,4,683\n").unwrap();
    fs::write(
        forever.join("LightSkybox.csv"),
        "ID,Flags,SkyboxFileDataID,CelestialSkyboxFileDataID\n683,3,7345733,0\n",
    )
    .unwrap();
    for table in ["ZoneLight", "ZoneLightPoint"] {
        fs::copy(
            source.join(format!("{table}.csv")),
            forever.join(format!("{table}.csv")),
        )
        .unwrap();
    }
}

fn encode_green_keyframes(data: &str) -> String {
    let mut lines = data.lines();
    let header = lines.next().unwrap();
    let columns: Vec<_> = header.split(',').collect();
    let params = columns
        .iter()
        .position(|name| *name == "LightParamID")
        .unwrap();
    let top = columns
        .iter()
        .position(|name| *name == "SkyTopColor")
        .unwrap();
    let rows: Vec<_> = lines
        .filter_map(|line| {
            let mut row: Vec<_> = line.split(',').collect();
            if row[params] != "12" {
                return None;
            }
            row[top] = "4278255360"; // Authored ARGB green.
            Some(row.join(","))
        })
        .collect();
    format!("{header}\n{}\n", rows.join("\n"))
}

#[test]
fn forever_lighting_samples_zephras_without_changing_retail() {
    let fixture = LightingFixture::new("isolation");
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let before = LightingCatalog::read(&source)
        .unwrap()
        .sample(0, [-8977.593, -179.765, 81.042], 1440.0)
        .unwrap();
    let catalog = LightingCatalog::read(&fixture.0).unwrap();
    let zephras = catalog
        .sample(2991, [2933.3333, 1333.3333, 746.1154], 1440.0)
        .unwrap();
    assert_eq!(zephras.sky.sky_top, [0.0, 1.0, 0.0]);
    assert_eq!(zephras.skyboxes.len(), 1);
    assert_eq!(zephras.skyboxes[0].fdid, 7345733);
    assert_eq!(zephras.skyboxes[0].flags, 3);
    let after = catalog
        .sample(0, [-8977.593, -179.765, 81.042], 1440.0)
        .unwrap();
    assert_eq!(format!("{:?}", before.sky), format!("{:?}", after.sky));
    assert_eq!(before.retail, after.retail);
    assert_eq!(before.fog, after.fog);
    assert_eq!(before.water, after.water);
    assert_eq!(before.skyboxes, after.skyboxes);
    assert_eq!(before.planets, after.planets);
    assert_eq!(before.stars_alpha, after.stars_alpha);
}

#[test]
fn forever_lighting_missing_keyframes_fails_without_retail_substitution() {
    let fixture = LightingFixture::new("missing");
    fs::remove_file(fixture.0.join("db2/1.60.1.70205/LightData.csv")).unwrap();
    let catalog = LightingCatalog::read(&fixture.0).unwrap();
    let sample = catalog.sample(2991, [2933.3333, 1333.3333, 746.1154], 1440.0);
    let error = match sample {
        Ok(_) => panic!("missing Forever keyframes silently took retail lighting"),
        Err(error) => error,
    };
    assert!(error.contains("1.60.1.70205/LightData.csv"), "{error}");
    assert!(catalog.sample(0, [0.0; 3], 1440.0).is_ok());
}
