use std::path::PathBuf;

use game_engine_ui_model::portrait_party_frame_component::{
    PortraitPartyFrameState, PortraitPartyMemberView, portrait_party_frame_screen,
};
use ui_toolkit::atlas::{ActiveSkin, resolve_region};
use ui_toolkit::frame::{Frame, WidgetData};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};
use ui_toolkit::widgets::texture::TextureSource;

fn members() -> Vec<PortraitPartyMemberView> {
    ["Theron", "Jaina", "Valeera", "Uther"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| PortraitPartyMemberView {
            name: name.into(),
            health_fraction: [0.75, 0.5, 0.25, 0.0][index],
            power_fraction: [0.5, 0.25, 1.0, 0.0][index],
            class_rgb: [
                [0.96, 0.55, 0.73],
                [0.25, 0.78, 0.92],
                [1.0, 0.96, 0.41],
                [0.96, 0.55, 0.73],
            ][index],
            leader: index == 1,
            offline: index == 2,
            dead: index == 3,
            ..PortraitPartyMemberView::named(name)
        })
        .collect()
}

fn render(skin: ActiveSkin) -> FrameRegistry {
    render_state(
        skin,
        PortraitPartyFrameState {
            members: members(),
            ..Default::default()
        },
    )
}

fn render_state(skin: ActiveSkin, state: PortraitPartyFrameState) -> FrameRegistry {
    game_engine_ui_model::paths::set_data_root(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(skin);
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(portrait_party_frame_screen).sync(&shared, &mut registry);
    registry
}

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(registry.get_by_name(name).expect(name))
        .unwrap()
}

fn text(registry: &FrameRegistry, name: &str) -> String {
    match frame(registry, name).widget_data.as_ref().unwrap() {
        WidgetData::FontString(data) => data.text.clone(),
        other => panic!("{name}: {other:?}"),
    }
}

fn region(
    registry: &FrameRegistry,
    name: &str,
    skin: ActiveSkin,
) -> ui_toolkit::atlas::AtlasRegion {
    let Some(WidgetData::Texture(data)) = &frame(registry, name).widget_data else {
        panic!("{name}");
    };
    let TextureSource::Atlas(atlas) = &data.source else {
        panic!("{name}");
    };
    resolve_region(atlas, skin).expect(atlas)
}

#[test]
fn four_members_draw_state_bars_names_status_and_only_the_leader() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        let registry = render(skin);
        for (index, view) in members().iter().enumerate() {
            let root = format!("PartyMemberFrame{}", index + 1);
            assert_eq!(text(&registry, &format!("{root}Name")), view.name);
            assert_eq!(
                frame(&registry, &format!("{root}LeaderIcon")).hidden,
                !view.leader
            );
            assert_eq!(
                text(&registry, &format!("{root}HealthBarText")),
                if view.offline {
                    "Offline"
                } else if view.dead {
                    "Dead"
                } else {
                    ""
                }
            );
            let health = frame(&registry, &format!("{root}HealthBarFill"));
            let power = frame(&registry, &format!("{root}ManaBarFill"));
            assert_eq!(
                health.width,
                ui_toolkit::frame::Dimension::Fixed(70.0 * view.health_fraction)
            );
            let power_width = if skin == ActiveSkin::Forever {
                69.0
            } else {
                74.0
            };
            assert_eq!(
                power.width,
                ui_toolkit::frame::Dimension::Fixed(power_width * view.power_fraction)
            );
            assert!(registry.get_by_name(&format!("{root}Portrait")).is_some());
        }
        assert!(registry.get_by_name("PartyMemberFrame5").is_none());
    }
}

#[test]
fn forever_party_draws_camelot_character_art_not_retail_party_art() {
    let modern = render(ActiveSkin::Modern);
    let forever = render(ActiveSkin::Forever);
    let party = region(&forever, "PartyMemberFrame1Art", ActiveSkin::Forever);
    let camelot = resolve_region(
        "UI-HUD-UnitFrame-CharacterFrameOnParty-PortraitOn",
        ActiveSkin::Forever,
    )
    .unwrap();
    assert_eq!(party, camelot);
    assert_ne!(
        party,
        region(&modern, "PartyMemberFrame1Art", ActiveSkin::Modern)
    );
    assert_eq!(
        party.source,
        resolve_region("UI-HUD-UnitFrame-Player-PortraitOn", ActiveSkin::Forever)
            .unwrap()
            .source
    );
}

#[path = "fixtures/modern_portrait_party.rs"]
mod fixture;

fn dump_tree(registry: &FrameRegistry, id: u64, depth: usize, out: &mut String) {
    use std::fmt::Write;
    let frame = registry.get(id).unwrap();
    writeln!(
        out,
        "{:indent$}{:?} {:?} w={:?} h={:?} pos={:?} hidden={} data={:?}",
        "",
        frame.name,
        frame.widget_type,
        frame.width,
        frame.height,
        frame.position,
        frame.hidden,
        frame.widget_data,
        indent = depth * 2
    )
    .unwrap();
    for child in &frame.children {
        dump_tree(registry, *child, depth + 1, out);
    }
}

fn modern_capture() -> String {
    let registry = render(ActiveSkin::Modern);
    let mut out = String::new();
    dump_tree(
        &registry,
        registry.get_by_name("PartyFrame").unwrap(),
        0,
        &mut out,
    );
    out
}

/// Regeneration captures authored Registry output, not a hand-maintained layout.
#[test]
#[ignore = "fixture regeneration; run with --ignored --nocapture"]
fn capture_modern_portrait_party_fixture() {
    println!("GOLDEN_START\n{}GOLDEN_END", modern_capture());
}

#[test]
fn modern_portrait_party_matches_captured_golden() {
    assert_eq!(modern_capture(), fixture::MODERN_PORTRAIT_PARTY);
}
