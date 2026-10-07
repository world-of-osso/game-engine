//! Unit frame art names Blizzard atlas elements. Under Modern every element resolves to
//! art, shared or distinct by role; under Forever the FlareUI-shaped frames draw none and
//! boss frames keep Modern's art.

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

/// An atlas crop: the sheet and the normalized `left, right, top, bottom` region.
#[derive(Debug, PartialEq)]
struct Crop(AtlasSource, [f32; 4]);

/// The sheet and crop texture frame `name` draws under `skin`: its atlas element's region
/// narrowed by the frame's `tex_coords`. The crop must be non-empty.
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
    let (span_x, span_y) = (region.right - region.left, region.bottom - region.top);
    let [u0, u1, v0, v1] = texture.tex_coords;
    let crop = [
        region.left + u0 * span_x,
        region.left + u1 * span_x,
        region.top + v0 * span_y,
        region.top + v1 * span_y,
    ];
    assert!(
        crop[1] > crop[0] && crop[3] > crop[2],
        "{name} under {skin:?}: empty crop {crop:?}"
    );
    Crop(region.source, crop)
}

/// Every crop in `crops` differs from every other.
fn assert_distinct<T: std::fmt::Debug>(crops: &[(T, Crop)]) {
    for (i, (a, crop_a)) in crops.iter().enumerate() {
        for (b, crop_b) in &crops[i + 1..] {
            assert_ne!(crop_a, crop_b, "{a:?} and {b:?} draw the same art");
        }
    }
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
        bosses: vec![UnitFrameState {
            classification,
            ..unit("Edwin VanCleef", PowerType::Mana)
        }],
        menu: UnitFrameMenuState::default(),
        personal_resource: None,
    });
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(inworld_unit_frames_screen).sync(&shared, &mut registry);
    registry
}

#[test]
fn bossframes_classification_portrait_is_drawn_for_engaged_elite() {
    let registry = unit_frames(
        ActiveSkin::Modern,
        CreatureClassification::Elite,
        PowerType::Mana,
    );
    let id = registry
        .get_by_name("Boss1BossPortraitFrameTexture")
        .expect("engaged boss must draw its classification portrait");
    assert!(!registry.get(id).unwrap().hidden);
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

/// Every texture part of the frames `unit_frames` shows under Modern.
const MODERN_FRAME_PARTS: [&str; 22] = [
    "PlayerFrameArt",
    "PlayerHealthBarFill",
    "PlayerCombatIcon",
    "PlayerRestingIcon",
    "TargetFrameArt",
    "TargetReputationColor",
    "TargetHealthBarFill",
    "TargetManaBarFill",
    "TargetBossPortraitFrameTexture",
    "TargetBossIcon",
    "TargetOfTargetFrameArt",
    "TargetOfTargetHealthBarFill",
    "FocusFrameArt",
    "FocusReputationColor",
    "FocusHealthBarFill",
    "PetFrameArt",
    "PetFrameHealthBarFill",
    "Boss1TargetFrameArt",
    "Boss1ReputationColor",
    "Boss1HealthBarFill",
    "Boss1ManaBarFill",
    "PlayerManaBarFill",
];

const POWERS: [PowerType; 10] = [
    PowerType::Mana,
    PowerType::Rage,
    PowerType::Focus,
    PowerType::Energy,
    PowerType::RunicPower,
    PowerType::LunarPower,
    PowerType::Maelstrom,
    PowerType::Insanity,
    PowerType::Fury,
    PowerType::Pain,
];

/// Every Modern part resolves to art; target of target and focus share portrait-off art;
/// encounter portraits use target art; a bar draws its power type's art whichever frame it is on; elite
/// and rare-elite dragons differ.
#[test]
fn modern_unit_frames_resolve_and_share_art_by_role() {
    let skin = ActiveSkin::Modern;
    let registry = unit_frames(skin, CreatureClassification::Elite, PowerType::Mana);
    let crop = |name: &str| drawn_crop(&registry, name, skin);
    for name in MODERN_FRAME_PARTS {
        crop(name);
    }
    for (a, b) in [
        ("TargetOfTargetFrameArt", "FocusFrameArt"),
        ("TargetReputationColor", "FocusReputationColor"),
        ("TargetReputationColor", "Boss1ReputationColor"),
        ("TargetOfTargetHealthBarFill", "FocusHealthBarFill"),
    ] {
        assert_eq!(crop(a), crop(b), "{a} and {b}");
    }
    assert_eq!(crop("PlayerHealthBarFill"), crop("FocusHealthBarFill"));
    assert_eq!(crop("TargetFrameArt"), crop("Boss1TargetFrameArt"));
    assert_eq!(crop("TargetHealthBarFill"), crop("Boss1HealthBarFill"));
    // The boss and the player both have mana; the target has rage.
    assert_eq!(crop("Boss1ManaBarFill"), crop("PlayerManaBarFill"));
    let raging = unit_frames(skin, CreatureClassification::Elite, PowerType::Rage);
    assert_eq!(
        crop("TargetManaBarFill"),
        drawn_crop(&raging, "PlayerManaBarFill", skin)
    );
    let rare = unit_frames(skin, CreatureClassification::RareElite, PowerType::Mana);
    assert_ne!(
        crop("TargetBossPortraitFrameTexture"),
        drawn_crop(&rare, "TargetBossPortraitFrameTexture", skin)
    );
}

/// Each power type draws its own bar art on the player frame and on the pet frame.
#[test]
fn modern_power_bars_draw_distinct_art_per_power_type() {
    let skin = ActiveSkin::Modern;
    let player: Vec<_> = POWERS
        .iter()
        .map(|&power| {
            let registry = unit_frames(skin, CreatureClassification::Normal, power);
            (power, drawn_crop(&registry, "PlayerManaBarFill", skin))
        })
        .collect();
    assert_distinct(&player);
    let pet: Vec<_> = POWERS[..5]
        .iter()
        .map(|&power| {
            let registry = unit_frames(skin, CreatureClassification::Normal, power);
            (power, drawn_crop(&registry, "PetFrameManaBarFill", skin))
        })
        .collect();
    assert_distinct(&pet);
}

/// Every party member part resolves to art; each role and each ready-check mark draws its
/// own icon.
#[test]
fn modern_party_frames_draw_an_icon_per_role_and_ready_mark() {
    let skin = ActiveSkin::Modern;
    let cases = [
        (GroupRoleSnapshot::Tank, ReadyMark::Ready),
        (GroupRoleSnapshot::Healer, ReadyMark::Waiting),
        (GroupRoleSnapshot::Damage, ReadyMark::NotReady),
    ];
    let mut roles = Vec::new();
    let mut marks = Vec::new();
    for (role, ready) in cases {
        let registry = party_member(role, ready);
        let member = |part: &str| format!("CompactPartyFrameMember1{part}");
        for part in [
            "Background",
            "HealthBar",
            "PowerBarBackground",
            "PowerBar",
            "SelectionHighlight",
        ] {
            drawn_crop(&registry, &member(part), skin);
        }
        roles.push((role, drawn_crop(&registry, &member("RoleIcon"), skin)));
        marks.push((
            ready,
            drawn_crop(&registry, &member("ReadyCheckIcon"), skin),
        ));
    }
    assert_distinct(&roles);
    assert_distinct(&marks);
}

/// Under Forever the player, target, target-of-target, focus and pet frames take FlareUI's
/// shape (`forever_flare_frames.rs`) and draw no portrait art; boss frames keep the
/// target portrait art, resolved under the active skin.
#[test]
fn bossframes_portrait_art_resolves_under_both_skins_and_flare_roots_keep_their_shape() {
    let forever = unit_frames(
        ActiveSkin::Forever,
        CreatureClassification::Elite,
        PowerType::Mana,
    );
    let modern = unit_frames(
        ActiveSkin::Modern,
        CreatureClassification::Elite,
        PowerType::Mana,
    );
    for name in MODERN_FRAME_PARTS {
        if name.starts_with("Boss1") {
            drawn_crop(&forever, name, ActiveSkin::Forever);
            drawn_crop(&modern, name, ActiveSkin::Modern);
        }
    }
    // Concrete committed atlas members: portrait-on art follows the active skin.
    assert_eq!(
        drawn_crop(&modern, "Boss1TargetFrameArt", ActiveSkin::Modern).0,
        AtlasSource::FileDataId(4_631_591)
    );
    assert_eq!(
        drawn_crop(&forever, "Boss1TargetFrameArt", ActiveSkin::Forever).0,
        AtlasSource::FileDataId(8_036_204)
    );
    for name in [
        "PlayerFrameArt",
        "TargetFrameArt",
        "TargetOfTargetFrameArt",
        "FocusFrameArt",
        "PetFrameArt",
        "TargetBossPortraitFrameTexture",
    ] {
        assert!(forever.get_by_name(name).is_none(), "{name} drawn");
    }
}
