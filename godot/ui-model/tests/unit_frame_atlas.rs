//! Unit frame art names Blizzard atlas elements. Under Modern every element draws exactly
//! the Retail `UiTextureAtlasMember` crop the frames hard-coded before; under Forever the
//! FlareUI-shaped frames draw none and boss frames keep their Retail crops.

use std::path::PathBuf;

use game_engine_ui_model::compact_unit_frame_component::{
    CompactUnitView, UnitStatus, compact_unit_frame,
};
use game_engine_ui_model::faction_reaction::Reaction;
use game_engine_ui_model::group_state::ReadyMark;
use game_engine_ui_model::inworld_unit_frames_component::{
    InWorldUnitFramesState, PetFrameState, PowerBarState, SmallUnitFrameState, UnitFrameMenuState,
    UnitFrameState, inworld_unit_frames_screen,
};
use shared::components::{CreatureClassification, PowerType};
use shared::protocol::GroupRoleSnapshot;
use ui_toolkit::atlas::{ActiveSkin, AtlasSource, resolve_region};
use ui_toolkit::frame::WidgetData;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn load_atlas_tables() {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
}

/// `UiTextureAtlas` `AtlasWidth`×`AtlasHeight` of every sheet the unit frames draw from.
fn sheet_size(fdid: u32) -> [f32; 2] {
    match fdid {
        4_631_591 => [1024.0, 512.0], // uiunitframe
        4_659_635 => [512.0, 512.0],  // uiunitframerestingflipbook
        4_703_659 => [256.0, 256.0],  // uiunitframeboss
        8_036_204 => [256.0, 512.0],  // uiunitframec60
        8_244_541 => [256.0, 256.0],  // uiunitframebossc60
        5_410_910 | 5_410_916 | 5_410_922 | 1_237_599 => [128.0, 32.0],
        5_412_495 => [1024.0, 1024.0], // priest insanity
        7_658_229 | 7_539_072 => [32.0, 32.0],
        7_539_067 => [16.0, 64.0],
        7_526_019 => [256.0, 128.0],
        5_171_843 => [2048.0, 2048.0], // group finder
        other => panic!("no sheet size for {other}"),
    }
}

/// An atlas crop: sheet FileDataID and pixel `left, right, top, bottom`.
#[derive(Debug, PartialEq)]
struct Crop(u32, [f32; 4]);

/// The sheet and pixel crop texture frame `name` draws under `skin`: its atlas element's
/// region narrowed by the frame's `tex_coords`.
fn drawn_crop(registry: &FrameRegistry, name: &str, skin: ActiveSkin) -> Crop {
    let frame = registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap();
    let Some(WidgetData::Texture(texture)) = frame.widget_data.as_ref() else {
        panic!("{name} is not a Texture");
    };
    let TextureSource::Atlas(atlas) = &texture.source else {
        panic!("{name} draws {:?}, not an atlas name", texture.source);
    };
    let region = resolve_region(atlas, skin).unwrap_or_else(|| panic!("{atlas} under {skin:?}"));
    let AtlasSource::FileDataId(fdid) = region.source else {
        panic!("{atlas} is not a FileDataID sheet");
    };
    let [width, height] = sheet_size(fdid);
    let (left, top) = (region.left * width, region.top * height);
    let (span_x, span_y) = (
        (region.right - region.left) * width,
        (region.bottom - region.top) * height,
    );
    let [u0, u1, v0, v1] = texture.tex_coords;
    Crop(
        fdid,
        [
            left + u0 * span_x,
            left + u1 * span_x,
            top + v0 * span_y,
            top + v1 * span_y,
        ],
    )
}

fn assert_crop(registry: &FrameRegistry, name: &str, skin: ActiveSkin, expected: Crop) {
    let drawn = drawn_crop(registry, name, skin);
    let close = drawn.0 == expected.0
        && drawn
            .1
            .iter()
            .zip(expected.1)
            .all(|(a, b)| (a - b).abs() < 1e-3);
    assert!(close, "{name} under {skin:?}: {drawn:?} != {expected:?}");
}

fn unit(name: &str, power: PowerType) -> UnitFrameState {
    UnitFrameState {
        health_fraction: 1.0,
        reaction: Some(Reaction::Hostile),
        power: Some(PowerBarState {
            power,
            current: 100,
            max: 100,
        }),
        show_combat_icon: true,
        show_resting_icon: true,
        ..UnitFrameState::named(name)
    }
}

fn small(name: &str) -> SmallUnitFrameState {
    SmallUnitFrameState {
        name: name.into(),
        level: None,
        health_fraction: 1.0,
        dead: false,
        reaction: Some(Reaction::Hostile),
        class_id: None,
        power: None,
    }
}

fn pet(power: PowerType) -> PetFrameState {
    PetFrameState {
        name: "Wolf".into(),
        health_fraction: 1.0,
        reaction: None,
        health_text: Default::default(),
        power: Some(PowerBarState {
            power,
            current: 100,
            max: 100,
        }),
        power_text: Default::default(),
    }
}

/// Every unit frame shown: player, an elite (`classification`) target, target of target,
/// focus, pet and one boss, the player and pet with `power`.
fn unit_frames(
    skin: ActiveSkin,
    classification: CreatureClassification,
    power: PowerType,
) -> FrameRegistry {
    load_atlas_tables();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(InWorldUnitFramesState {
        show_player_frame: true,
        show_target_frame: true,
        target_cast: None,
        player: unit("Fbatlas", power),
        target: Some(UnitFrameState {
            classification,
            ..unit("Hogger", PowerType::Rage)
        }),
        target_of_target: Some(small("Fbatlas")),
        focus: Some(small("Mother Fang")),
        pet: Some(pet(power)),
        bosses: vec![unit("Edwin VanCleef", PowerType::Mana)],
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}

fn party_member(role: GroupRoleSnapshot, ready: ReadyMark) -> FrameRegistry {
    load_atlas_tables();
    let view = CompactUnitView {
        name: "Fbparty".into(),
        class_rgb: [1.0, 1.0, 1.0],
        health_fraction: Some(1.0),
        power: Some((1.0, [0.0, 0.0, 1.0])),
        role,
        status: UnitStatus::Online,
        in_range: true,
        selected: true,
        ready: Some(ready),
        debuffs: Vec::new(),
    };
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(move |_| {
        compact_unit_frame("CompactPartyFrameMember1", &view, (0.0, 0.0, 72.0, 36.0))
    })
    .sync(&SharedContext::new(), &mut registry);
    registry
}

const UNIT_FRAME: u32 = 4_631_591;
const PORTRAIT_OFF: [f32; 4] = [195.0, 328.0, 160.0, 211.0];
const PLAYER_HEALTH: [f32; 4] = [705.0, 829.0, 213.0, 233.0];
const REACTION_STRIP: [f32; 4] = [195.0, 330.0, 235.0, 253.0];

/// The crops the frames hard-coded (as `AtlasArt`) before naming atlases, each a Retail
/// 12.1.0.69933 `UiTextureAtlasMember` on canvas 1x.
fn modern_frame_crops() -> Vec<(&'static str, Crop)> {
    vec![
        // UI-HUD-UnitFrame-Player-PortraitOn (16110)
        (
            "PlayerFrameArt",
            Crop(UNIT_FRAME, [1.0, 199.0, 87.0, 158.0]),
        ),
        ("PlayerHealthBarFill", Crop(UNIT_FRAME, PLAYER_HEALTH)),
        // UI-HUD-UnitFrame-Player-CombatIcon (16779)
        (
            "PlayerCombatIcon",
            Crop(UNIT_FRAME, [1007.0, 1023.0, 133.0, 149.0]),
        ),
        // First 60×60 cell of UI-HUD-UnitFrame-Player-Rest-Flipbook (16568)
        ("PlayerRestingIcon", Crop(4_659_635, [1.0, 61.0, 1.0, 61.0])),
        // UI-HUD-UnitFrame-Target-PortraitOn (16118)
        (
            "TargetFrameArt",
            Crop(UNIT_FRAME, [1.0, 193.0, 229.0, 296.0]),
        ),
        ("TargetReputationColor", Crop(UNIT_FRAME, REACTION_STRIP)),
        // UI-HUD-UnitFrame-Target-PortraitOn-Bar-Health (16111)
        (
            "TargetHealthBarFill",
            Crop(UNIT_FRAME, [195.0, 321.0, 213.0, 233.0]),
        ),
        // UI-HUD-UnitFrame-Player-PortraitOff-Bar-Rage (16868)
        (
            "TargetManaBarFill",
            Crop(UNIT_FRAME, [779.0, 903.0, 289.0, 299.0]),
        ),
        // UI-HUD-UnitFrame-Target-PortraitOn-Boss-Gold (17120)
        (
            "TargetBossPortraitFrameTexture",
            Crop(4_703_659, [1.0, 81.0, 84.0, 163.0]),
        ),
        // UI-HUD-UnitFrame-Target-PortraitOn-Boss-Rare-Star (17122)
        (
            "TargetBossIcon",
            Crop(4_703_659, [83.0, 109.0, 148.0, 174.0]),
        ),
        ("TargetOfTargetFrameArt", Crop(UNIT_FRAME, PORTRAIT_OFF)),
        (
            "TargetOfTargetHealthBarFill",
            Crop(UNIT_FRAME, PLAYER_HEALTH),
        ),
        ("FocusFrameArt", Crop(UNIT_FRAME, PORTRAIT_OFF)),
        ("FocusReputationColor", Crop(UNIT_FRAME, REACTION_STRIP)),
        ("FocusHealthBarFill", Crop(UNIT_FRAME, PLAYER_HEALTH)),
        // UI-HUD-UnitFrame-TargetofTarget-PortraitOn (16121)
        (
            "PetFrameArt",
            Crop(UNIT_FRAME, [330.0, 450.0, 160.0, 209.0]),
        ),
        // UI-HUD-UnitFrame-TargetofTarget-PortraitOn-Bar-Health (16119)
        (
            "PetFrameHealthBarFill",
            Crop(UNIT_FRAME, [950.0, 1020.0, 160.0, 170.0]),
        ),
        ("Boss1TargetFrameArt", Crop(UNIT_FRAME, PORTRAIT_OFF)),
        ("Boss1ReputationColor", Crop(UNIT_FRAME, REACTION_STRIP)),
        ("Boss1HealthBarFill", Crop(UNIT_FRAME, PLAYER_HEALTH)),
        // UI-HUD-UnitFrame-Player-PortraitOn-Bar-Mana (16109)
        (
            "Boss1ManaBarFill",
            Crop(UNIT_FRAME, [647.0, 771.0, 301.0, 311.0]),
        ),
    ]
}

/// Player power bar per power type, the old `power_bar_art` table.
fn modern_power_crops() -> Vec<(PowerType, Crop)> {
    vec![
        (
            PowerType::Mana,
            Crop(UNIT_FRAME, [647.0, 771.0, 301.0, 311.0]),
        ),
        (
            PowerType::Rage,
            Crop(UNIT_FRAME, [779.0, 903.0, 289.0, 299.0]),
        ),
        (
            PowerType::Focus,
            Crop(UNIT_FRAME, [653.0, 777.0, 289.0, 299.0]),
        ),
        (
            PowerType::Energy,
            Crop(UNIT_FRAME, [527.0, 651.0, 289.0, 299.0]),
        ),
        (
            PowerType::RunicPower,
            Crop(UNIT_FRAME, [269.0, 393.0, 301.0, 311.0]),
        ),
        (
            PowerType::LunarPower,
            Crop(5_410_916, [1.0, 127.0, 1.0, 11.0]),
        ),
        (
            PowerType::Maelstrom,
            Crop(5_410_922, [1.0, 127.0, 1.0, 11.0]),
        ),
        (
            PowerType::Insanity,
            Crop(5_412_495, [1.0, 127.0, 1.0, 11.0]),
        ),
        (PowerType::Fury, Crop(5_410_910, [1.0, 127.0, 1.0, 11.0])),
        (PowerType::Pain, Crop(1_237_599, [0.0, 128.0, 13.0, 23.0])),
    ]
}

/// Pet power bar per power type, the old `tot_power_bar_art` table.
fn modern_pet_power_crops() -> Vec<(PowerType, Crop)> {
    vec![
        (
            PowerType::Mana,
            Crop(UNIT_FRAME, [389.0, 463.0, 267.0, 274.0]),
        ),
        (
            PowerType::Rage,
            Crop(UNIT_FRAME, [661.0, 735.0, 267.0, 274.0]),
        ),
        (
            PowerType::Focus,
            Crop(UNIT_FRAME, [884.0, 958.0, 77.0, 84.0]),
        ),
        (
            PowerType::Energy,
            Crop(UNIT_FRAME, [808.0, 882.0, 77.0, 84.0]),
        ),
        (
            PowerType::RunicPower,
            Crop(UNIT_FRAME, [933.0, 1007.0, 255.0, 262.0]),
        ),
    ]
}

#[test]
fn modern_unit_frames_draw_the_retail_crops_they_hard_coded() {
    let skin = ActiveSkin::Modern;
    let registry = unit_frames(skin, CreatureClassification::Elite, PowerType::Mana);
    for (name, crop) in modern_frame_crops() {
        assert_crop(&registry, name, skin, crop);
    }
    let rare = unit_frames(skin, CreatureClassification::RareElite, PowerType::Mana);
    // ui-hud-unitframe-target-portraiton-boss-rare-silver (19019)
    let silver = Crop(4_703_659, [1.0, 81.0, 165.0, 244.0]);
    assert_crop(&rare, "TargetBossPortraitFrameTexture", skin, silver);
}

#[test]
fn modern_power_bars_draw_the_retail_crops_they_hard_coded() {
    let skin = ActiveSkin::Modern;
    for (power, crop) in modern_power_crops() {
        let registry = unit_frames(skin, CreatureClassification::Normal, power);
        assert_crop(&registry, "PlayerManaBarFill", skin, crop);
    }
    for (power, crop) in modern_pet_power_crops() {
        let registry = unit_frames(skin, CreatureClassification::Normal, power);
        assert_crop(&registry, "PetFrameManaBarFill", skin, crop);
    }
}

#[test]
fn modern_party_frames_draw_the_retail_crops_they_hard_coded() {
    let skin = ActiveSkin::Modern;
    let lfg = 5_171_843;
    let cases = [
        // UI-LFG-RoleIcon-Tank-Micro-GroupFinder (27953), UI-LFG-ReadyMark-Raid (23888)
        (
            GroupRoleSnapshot::Tank,
            ReadyMark::Ready,
            [2026.0, 2047.0, 47.0, 68.0],
            [1947.0, 2011.0, 391.0, 455.0],
        ),
        // ...-Healer-... (27950), UI-LFG-PendingMark-Raid (23887)
        (
            GroupRoleSnapshot::Healer,
            ReadyMark::Waiting,
            [2003.0, 2024.0, 24.0, 45.0],
            [1947.0, 2011.0, 325.0, 389.0],
        ),
        // ...-DPS-... (27948), UI-LFG-DeclineMark-Raid (23886)
        (
            GroupRoleSnapshot::Damage,
            ReadyMark::NotReady,
            [2003.0, 2024.0, 1.0, 22.0],
            [1947.0, 2011.0, 259.0, 323.0],
        ),
    ];
    for (role, ready, role_rect, ready_rect) in cases {
        let registry = party_member(role, ready);
        let member = |part: &str| format!("CompactPartyFrameMember1{part}");
        // raidframe-hp-bg-white (35719), RaidFrame-Hp-Fill (35362)
        assert_crop(
            &registry,
            &member("Background"),
            skin,
            Crop(7_658_229, [0.0, 32.0, 0.0, 32.0]),
        );
        assert_crop(
            &registry,
            &member("HealthBar"),
            skin,
            Crop(7_539_072, [0.0, 32.0, 0.0, 32.0]),
        );
        // _RaidFrame-Resource-Background (35359), _RaidFrame-Resource-Fill (35360)
        assert_crop(
            &registry,
            &member("PowerBarBackground"),
            skin,
            Crop(7_539_067, [0.0, 16.0, 43.0, 49.0]),
        );
        assert_crop(
            &registry,
            &member("PowerBar"),
            skin,
            Crop(7_539_067, [0.0, 16.0, 51.0, 57.0]),
        );
        // RaidFrame-TargetFrame (35354)
        assert_crop(
            &registry,
            &member("SelectionHighlight"),
            skin,
            Crop(7_526_019, [145.0, 215.0, 1.0, 35.0]),
        );
        assert_crop(&registry, &member("RoleIcon"), skin, Crop(lfg, role_rect));
        assert_crop(
            &registry,
            &member("ReadyCheckIcon"),
            skin,
            Crop(lfg, ready_rect),
        );
    }
}

/// Under Forever the player, target, target-of-target, focus and pet frames take FlareUI's
/// shape (`forever_flare_frames.rs`) and draw no portrait art; boss frames keep the
/// portrait-off names, which Forever does not re-skin (no set-1 member), so they draw
/// their Retail crops.
#[test]
fn forever_boss_frames_keep_retail_crops_and_flare_frames_draw_no_portrait_art() {
    let skin = ActiveSkin::Forever;
    let registry = unit_frames(skin, CreatureClassification::Elite, PowerType::Mana);
    for (name, crop) in modern_frame_crops() {
        if name.starts_with("Boss1") {
            assert_crop(&registry, name, skin, crop);
        }
    }
    for name in [
        "PlayerFrameArt",
        "TargetFrameArt",
        "TargetOfTargetFrameArt",
        "FocusFrameArt",
        "PetFrameArt",
        "TargetBossPortraitFrameTexture",
    ] {
        assert!(registry.get_by_name(name).is_none(), "{name} drawn");
    }
}
