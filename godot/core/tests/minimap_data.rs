use std::io::Cursor;
use std::path::Path;

use game_engine_core::minimap_data::{
    AreaCatalog, FACTION_GROUP_ALLIANCE, FACTION_GROUP_HORDE, MISSING_TILE_COLOR, MapMask,
    MinimapView, OUTDOOR_DIAMETERS, TILE_YARDS, TileImage, ZonePvp, clock_text, compose,
    parse_race_faction_groups, parse_vignettes, tile_path, tint_quest_areas, zoom_in, zoom_out,
};
use game_engine_core::terrain_height_data::bevy_to_tile_coords;

/// Human start in Northshire, Retail world (-8949.95, -132.49) as engine (x, z).
const NORTHSHIRE: [f32; 2] = [-8949.95, 132.49];

fn solid(color: [u8; 4]) -> TileImage {
    TileImage {
        pixels: color.repeat(4 * 4),
        width: 4,
        height: 4,
    }
}

fn pixel(image: &[u8], size: u32, x: u32, y: u32) -> [u8; 4] {
    let i = ((y * size + x) * 4) as usize;
    image[i..i + 4].try_into().unwrap()
}

#[test]
fn northshire_player_resolves_the_azeroth_tile_its_adt_uses() {
    let view = MinimapView::new(NORTHSHIRE, 0);
    let key = bevy_to_tile_coords(NORTHSHIRE[0], NORTHSHIRE[1]);
    assert_eq!(key, (32, 48));
    assert_eq!(
        tile_path("azeroth", key),
        "world/minimaps/azeroth/map32_48.blp"
    );
    // 466.67 yards across, 0.25 into tile 32 east-west and 0.78 north-south: the view
    // reaches west into 31 and south into 49.
    assert_eq!(view.tiles(), vec![(31, 48), (31, 49), (32, 48), (32, 49)]);
    assert_eq!(
        tile_path("azeroth", (3, 7)),
        "world/minimaps/azeroth/map03_07.blp"
    );
}

#[test]
fn composite_is_north_up_with_east_on_the_right_and_a_round_mask() {
    const RED: [u8; 4] = [200, 0, 0, 255];
    const GREEN: [u8; 4] = [0, 200, 0, 255];
    const BLUE: [u8; 4] = [0, 0, 200, 255];
    let (red, green, blue) = (solid(RED), solid(GREEN), solid(BLUE));
    // Centre exactly on the corner shared by tiles (32, 48) (north-west of it),
    // (33, 48) (north-east) and (32, 49) (south-west); (33, 49) is not on disk.
    let corner = [(32.0 - 49.0) * TILE_YARDS, (33.0 - 32.0) * TILE_YARDS];
    let view = MinimapView::new(corner, 0);
    let size = 64;
    let image = compose(&view, size, |key| match key {
        (32, 48) => Some(&red),
        (33, 48) => Some(&green),
        (32, 49) => Some(&blue),
        _ => None,
    });
    assert_eq!(pixel(&image, size, 20, 20), RED, "north-west");
    assert_eq!(pixel(&image, size, 44, 20), GREEN, "north-east");
    assert_eq!(pixel(&image, size, 20, 44), BLUE, "south-west");
    assert_eq!(
        pixel(&image, size, 44, 44),
        MISSING_TILE_COLOR,
        "south-east"
    );
    assert_eq!(pixel(&image, size, 0, 0)[3], 0, "outside the round mask");
    let edge = pixel(&image, size, 63, 32)[3];
    assert!(edge > 0 && edge < 255, "soft mask edge alpha {edge}");
}

/// The Forever skin's square minimap: the composite fills its corners, and a blip in a
/// corner the round mask hides is on the map.
#[test]
fn square_mask_fills_the_corners_and_keeps_corner_blips() {
    const RED: [u8; 4] = [200, 0, 0, 255];
    let red = solid(RED);
    let round = MinimapView::new(NORTHSHIRE, 0);
    let square = round.masked(MapMask::Square);
    let size = 64;
    let corners = |view: &MinimapView| {
        let image = compose(view, size, |_| Some(&red));
        [(0, 0), (63, 0), (0, 63), (63, 63)].map(|(x, y)| pixel(&image, size, x, y))
    };
    assert_eq!(corners(&square), [RED; 4]);
    assert_eq!(corners(&round), [[0; 4]; 4]);
    // 200 yards north and 200 east of the player on the 466.67-yard view: 0.43 of the
    // map each way, 0.61 from the centre.
    let corner = [NORTHSHIRE[0] + 200.0, NORTHSHIRE[1] + 200.0];
    let offset = 200.0 / OUTDOOR_DIAMETERS[0];
    let [right, down] = square
        .blip_offset(corner)
        .expect("corner blip on the square map");
    assert!(
        (right - offset).abs() < 1e-4 && (down + offset).abs() < 1e-4,
        "{right}, {down} vs {offset}"
    );
    assert_eq!(round.blip_offset(corner), None);
    // 240 yards east is past the square's edge too.
    assert_eq!(
        square.blip_offset([NORTHSHIRE[0], NORTHSHIRE[1] + 240.0]),
        None
    );
}

#[test]
fn composite_samples_the_texel_under_the_player() {
    // A 2x2 tile, one colour per quadrant: north-west, north-east / south-west, south-east.
    let tile = TileImage {
        pixels: [
            [255, 0, 0, 255],
            [0, 255, 0, 255],
            [0, 0, 255, 255],
            [255, 255, 0, 255],
        ]
        .concat(),
        width: 2,
        height: 2,
    };
    // Player in the south-east quadrant of tile (32, 48): u 0.9, v 0.9.
    let player = [(32.0 - 48.9) * TILE_YARDS, (32.9 - 32.0) * TILE_YARDS];
    let size = 16;
    let image = compose(&MinimapView::new(player, 5), size, |key| {
        (key == (32, 48)).then_some(&tile)
    });
    assert_eq!(pixel(&image, size, 8, 8), [255, 255, 0, 255]);
}

#[test]
fn blips_offset_by_compass_direction_and_vanish_past_the_edge() {
    let view = MinimapView::new(NORTHSHIRE, 0);
    let diameter = OUTDOOR_DIAMETERS[0];
    let north = [NORTHSHIRE[0] + 100.0, NORTHSHIRE[1]];
    let east = [NORTHSHIRE[0], NORTHSHIRE[1] + 50.0];
    let close = |got: Option<[f32; 2]>, want: [f32; 2]| {
        let got = got.expect("blip inside the minimap");
        assert!(
            (got[0] - want[0]).abs() < 1e-4 && (got[1] - want[1]).abs() < 1e-4,
            "{got:?} vs {want:?}"
        );
    };
    close(view.blip_offset(north), [0.0, -100.0 / diameter]);
    close(view.blip_offset(east), [50.0 / diameter, 0.0]);
    assert_eq!(
        view.blip_offset([NORTHSHIRE[0] - 240.0, NORTHSHIRE[1]]),
        None
    );
    // Zoomed in to 133 yards, the 100-yard north blip is outside.
    assert_eq!(MinimapView::new(NORTHSHIRE, 5).blip_offset(north), None);
}

#[test]
fn zoom_buttons_clamp_to_the_six_levels() {
    assert_eq!(zoom_in(0), 1);
    assert_eq!(zoom_in(5), 5);
    assert_eq!(zoom_out(1), 0);
    assert_eq!(zoom_out(0), 0);
    assert_eq!(
        MinimapView::new(NORTHSHIRE, 9).diameter,
        OUTDOOR_DIAMETERS[5]
    );
}

#[test]
fn clock_uses_the_twelve_hour_ticker() {
    assert_eq!(clock_text(0, 5), "12:05");
    assert_eq!(clock_text(9, 30), "9:30");
    assert_eq!(clock_text(12, 0), "12:00");
    assert_eq!(clock_text(13, 7), "1:07");
}

#[test]
fn zone_text_names_the_subzone_and_colours_by_controlling_faction() {
    let csv = "ID,ZoneName,AreaName_lang,ParentAreaID,FactionGroupMask,Flags_0\n\
        12,ElwynnForest,\"Elwynn Forest\",0,2,16448\n\
        6170,Northshire,Northshire,12,2,16448\n\
        24,Northshireabbey,\"Northshire Abbey\",6170,0,1076904000\n\
        14,Durotar,Durotar,0,4,0\n\
        3,Badlands,Badlands,0,0,0\n\
        3487,SilvermoonCity,\"Silvermoon City\",0,4,2048\n";
    let areas = AreaCatalog::parse(Cursor::new(csv), Path::new("AreaTable.csv")).unwrap();
    assert_eq!(areas.name(24), Some("Northshire Abbey"));
    assert_eq!(areas.pvp(24, FACTION_GROUP_ALLIANCE), ZonePvp::Friendly);
    assert_eq!(areas.pvp(24, FACTION_GROUP_HORDE), ZonePvp::Hostile);
    assert_eq!(areas.pvp(14, FACTION_GROUP_ALLIANCE), ZonePvp::Hostile);
    assert_eq!(areas.pvp(3, FACTION_GROUP_ALLIANCE), ZonePvp::Normal);
    assert_eq!(areas.pvp(3487, FACTION_GROUP_ALLIANCE), ZonePvp::Sanctuary);
    assert_eq!(ZonePvp::Friendly.text_color(), [0.1, 1.0, 0.1, 1.0]);
}

#[test]
fn races_map_to_their_faction_group() {
    let csv = "ID,ClientPrefix,Alliance\n1,Hu,0\n2,Or,1\n24,Pa,2\n";
    let races = parse_race_faction_groups(Cursor::new(csv), Path::new("ChrRaces.csv")).unwrap();
    assert_eq!(races.get(&1), Some(&FACTION_GROUP_ALLIANCE));
    assert_eq!(races.get(&2), Some(&FACTION_GROUP_HORDE));
    assert_eq!(races.get(&24), None);
}

#[test]
fn quest_area_tints_only_inside_its_polygon_with_a_brighter_rim() {
    let view = MinimapView::new(NORTHSHIRE, 0);
    let grey = solid([100, 100, 100, 255]);
    let size = 256;
    let mut image = compose(&view, size, |_| Some(&grey));
    // A 40-yard square north-east of the player: engine +x is north, +z east.
    let [x, z] = NORTHSHIRE;
    let square = vec![
        [x + 10.0, z + 10.0],
        [x + 50.0, z + 10.0],
        [x + 50.0, z + 50.0],
        [x + 10.0, z + 50.0],
    ];
    let art = game_engine_core::quest_area_data::QuestAreaArt::load(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        false,
    )
    .unwrap();
    let tinted = tint_quest_areas(&view, size, &mut image, &[square], &art, false);
    let yards_per_pixel = view.diameter / size as f32;
    let expected = (40.0 / yards_per_pixel).powi(2);
    assert!(
        (tinted as f32 - expected).abs() <= 12.0 * 40.0 / yards_per_pixel,
        "{tinted} tinted pixels for a {expected} pixel square"
    );
    // The player's pixel (centre) is outside the square and untouched.
    assert_eq!(
        pixel(&image, size, size / 2, size / 2),
        [100, 100, 100, 255]
    );
    // The square's centre (30 yd north and east) is filled, its rim brighter.
    let centre = |yards_east: f32, yards_north: f32| {
        let px = (size as f32 / 2.0 + yards_east / yards_per_pixel) as u32;
        let py = (size as f32 / 2.0 - yards_north / yards_per_pixel) as u32;
        pixel(&image, size, px, py)
    };
    let fill = centre(30.0, 30.0);
    assert_eq!(fill[3], 255);
    assert!(
        fill[2] > 100 && fill[2] > fill[0],
        "fill {fill:?} is not blue-tinted"
    );
    let rim = centre(11.0, 30.0);
    assert!(
        rim[0] > fill[0],
        "rim {rim:?} is not brighter than the fill {fill:?}"
    );
}

/// Vignette.csv rows of build 12.1.0.69933: Doomwalker (ShowOnMap, HasTooltip), Gift
/// of the Brokenhearted (DontShowOnMinimap) and War Supply Crate (HideOnContinentMaps).
#[test]
fn vignette_flags_place_doomwalker_on_both_maps() {
    let csv = "ID,Name_lang,PlayerConditionID,VisibleTrackingQuestID,QuestFeedbackEffectID,Flags,MaxHeight,MinHeight,VignetteType,RewardQuestID,UiWidgetSetID,UiMapPinInfoID,ObjectiveType\n\
        6520,Doomwalker,0,60214,1152,263170,100,0,0,0,0,3,0\n\
        2665,\"Gift of the Brokenhearted\",0,50582,88,512,30,-30,0,0,0,0,0\n\
        2967,\"War Supply Crate\",0,0,1109,66630,0,0,0,0,0,0,0\n";
    let vignettes = parse_vignettes(Cursor::new(csv), Path::new("Vignette.csv")).unwrap();
    let doomwalker = &vignettes[&6520];
    assert_eq!(doomwalker.name, "Doomwalker");
    assert!(doomwalker.on_minimap() && doomwalker.on_world_map());
    assert!(!doomwalker.hide_on_continent_maps());
    let gift = &vignettes[&2665];
    assert_eq!(gift.name, "Gift of the Brokenhearted");
    assert!(!gift.on_minimap() && !gift.on_world_map());
    assert!(vignettes[&2967].hide_on_continent_maps());
}

#[test]
fn rotating_minimap_moves_north_east_with_the_map_and_keeps_inverse_projection() {
    let view = MinimapView::new(NORTHSHIRE, 0).rotated(std::f32::consts::FRAC_PI_2);
    let north = [NORTHSHIRE[0] + view.diameter * 0.25, NORTHSHIRE[1]];
    let offset = view.blip_offset(north).unwrap();
    assert!((offset[0] + 0.25).abs() < 0.0001 && offset[1].abs() < 0.0001);
    let position = view.pixel_position(16, 32, 64);
    let inverse = view.blip_offset(position).unwrap();
    assert!((inverse[0] - (16.5 / 64.0 - 0.5)).abs() < 0.0001);
    assert!((inverse[1] - (32.5 / 64.0 - 0.5)).abs() < 0.0001);
}
