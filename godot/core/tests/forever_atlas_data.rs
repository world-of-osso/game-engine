use std::path::Path;

use game_engine_core::blp::decode_rgba;
use game_engine_core::warband_scene_data::{AtlasArt, read_forever_atlas_art};

/// Forever (1.60.1.69913) re-skins Retail atlas names onto `UiTextureAtlasSetID` 1 `c60`
/// textures; `UiTextureAtlasMember` rows `<name>-c60` crop them at UI scale 1.
#[test]
fn retail_atlas_names_resolve_to_forever_set_one_c60_art() {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let names = [
        "UI-HUD-UnitFrame-Player-PortraitOn",
        "UI-HUD-ActionBar-IconFrame",
        "Tooltip-NineSlice-CornerTopLeft",
        "UI-HUD-Minimap-Frame",
    ];
    let art = read_forever_atlas_art(&data_root, &names).expect("read Forever atlas tables");
    let crop = |fdid, [width, height]: [f32; 2], [left, right, top, bottom]: [f32; 4]| AtlasArt {
        fdid,
        tex_coords: [left / width, right / width, top / height, bottom / height],
    };

    // interface/hud/uiunitframec60.blp, 256x512; member 38478.
    assert_eq!(
        art["ui-hud-unitframe-player-portraiton"],
        crop(8036204, [256.0, 512.0], [1.0, 199.0, 246.0, 317.0])
    );
    // interface/hud/uiactionbarc60.blp, 512x512; member 39247.
    assert_eq!(
        art["ui-hud-actionbar-iconframe"],
        crop(7948328, [512.0, 512.0], [1.0, 47.0, 449.0, 494.0])
    );
    // interface/tooltips/uiframetooltipc60.blp, 16x64; member 38829.
    assert_eq!(
        art["tooltip-nineslice-cornertopleft"],
        crop(8070606, [16.0, 64.0], [1.0, 8.0, 37.0, 44.0])
    );
    // interface/hud/uiminimapc60.blp, 512x512; member 39072.
    assert_eq!(
        art["ui-hud-minimap-frame"],
        crop(8026705, [512.0, 512.0], [1.0, 254.0, 1.0, 254.0])
    );
}

/// The extracted `c60` textures decode at their `UiTextureAtlas` size with opaque art inside
/// each crop.
#[test]
fn forever_c60_textures_decode_at_their_atlas_size() {
    let data_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data");
    let names = [
        "UI-HUD-UnitFrame-Player-PortraitOn",
        "UI-HUD-ActionBar-IconFrame",
        "UI-HUD-Minimap-Frame",
    ];
    let art = read_forever_atlas_art(&data_root, &names).expect("read Forever atlas tables");
    for (name, size) in names.iter().zip([[256, 512], [512, 512], [512, 512]]) {
        let art = art[&name.to_ascii_lowercase()];
        let path = data_root.join(format!("textures/{}.blp", art.fdid));
        let bytes = std::fs::read(&path).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        let image = decode_rgba(&bytes).unwrap_or_else(|err| panic!("{name}: {err}"));
        assert_eq!([image.width, image.height], size, "{name}");
        let [left, right, top, bottom] = art.tex_coords;
        let pixel = |x: f32, y: f32| {
            let (x, y) = ((x * size[0] as f32) as u32, (y * size[1] as f32) as u32);
            image.pixels[((y * image.width + x) * 4 + 3) as usize]
        };
        let opaque = (0..16)
            .flat_map(|i| (0..16).map(move |j| (i, j)))
            .filter(|&(i, j)| {
                let x = left + (right - left) * (i as f32 + 0.5) / 16.0;
                let y = top + (bottom - top) * (j as f32 + 0.5) / 16.0;
                pixel(x, y) == 255
            })
            .count();
        assert!(opaque > 0, "{name}: no opaque pixel inside {art:?}");
    }
}
