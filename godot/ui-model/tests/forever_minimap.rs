//! The MinimapCluster under both skins. Modern is exactly the Retail cluster the component
//! built before it knew skins (master 5e9c5994). Forever is FlareUI's square minimap
//! (FlareUI 1.3 `Modules/Minimap.lua:32-47,154-208,301-303`): a 244×244 map without a mask in a
//! 260×260 cluster, a 17-high header with the zone name and the clock at its TOPRIGHT, and
//! bronze Blizzard tooltip-border chrome instead of the compass ring.

use std::fmt::Write;

#[path = "fixtures/modern_minimap_trees.rs"]
mod fixture;

use game_engine_ui_model::minimap::{
    BlipKind, MINIMAP_ARROW, MINIMAP_CLOCK_TEXT, MINIMAP_CLUSTER, MINIMAP_DISPLAY,
    MINIMAP_MAIL_FRAME, MINIMAP_ZONE_TEXT, MINIMAP_ZOOM_IN, MINIMAP_ZOOM_OUT, MinimapBlip,
    MinimapClusterState, minimap_cluster_screen,
};
use ui_toolkit::atlas::ActiveSkin;
use ui_toolkit::frame::{Dimension, Frame, WidgetData};
use ui_toolkit::layout_values::Val;
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn build(skin: ActiveSkin, state: MinimapClusterState) -> FrameRegistry {
    let mut shared = SharedContext::new();
    shared.insert(skin);
    // Keep the captured baseline stable when the user changes the runtime style pick.
    shared.insert(game_engine_ui_model::launcher::LauncherIconStyle::Outline);
    shared.insert(state);
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    Screen::new(minimap_cluster_screen).sync(&shared, &mut registry);
    registry
}

/// Every attribute the component sets, one line per frame in tree order.
fn dump(registry: &FrameRegistry) -> String {
    let mut out = String::new();
    let root = registry.get_by_name(MINIMAP_CLUSTER).expect("cluster");
    dump_frame(registry, root, 0, &mut out);
    out
}

fn dump_frame(registry: &FrameRegistry, id: u64, depth: usize, out: &mut String) {
    let f = registry.get(id).unwrap();
    writeln!(
        out,
        "{:indent$}{:?} {:?} w={:?} h={:?} pos={:?} {:?} margin={:?} translate={:?} \
         hidden={} alpha={} strata={:?} level={} layer={:?} bg={:?} mouse={} click={:?} \
         style={:?} data={:?}",
        "",
        f.name,
        f.widget_type,
        f.width,
        f.height,
        f.position,
        f.position_type,
        f.margin,
        f.translation,
        f.hidden,
        f.alpha,
        f.strata,
        f.frame_level,
        f.draw_layer,
        f.background_color,
        f.mouse_enabled,
        f.onclick,
        f.panel_style,
        f.widget_data,
        indent = depth * 2
    )
    .unwrap();
    for child in &f.children {
        dump_frame(registry, *child, depth + 1, out);
    }
}

/// A blip in the map's north-east corner, 0.64 of the map from its centre: on the square
/// map only, as `MinimapView::blip_offset` clips it.
const CORNER_BLIP: [f32; 2] = [0.45, -0.45];

/// Zone text in the hostile colour, clock, calendar day, mail, hover zoom buttons at the
/// top zoom, and one blip of each kind.
fn busy_state() -> MinimapClusterState {
    let blip = |unit, kind, offset| MinimapBlip { unit, kind, offset };
    MinimapClusterState {
        zone_text: "Northshire Valley".into(),
        zone_color: [1.0, 0.1, 0.1, 1.0],
        clock_text: "3:07".into(),
        calendar_day: Some(3),
        arrow_rotation: 0.5,
        zoom_buttons: true,
        zoom: 5,
        blips: vec![
            blip(7, BlipKind::QuestAvailable, [0.25, -0.25]),
            blip(8, BlipKind::QuestTurnIn, [-0.1, 0.3]),
            blip(9, BlipKind::Vignette { elite: false }, [0.0, 0.0]),
            blip(10, BlipKind::Vignette { elite: true }, CORNER_BLIP),
        ],
        has_mail: true,
        map_texture: None,
    }
}

fn trees(skin: ActiveSkin) -> String {
    let mut out = dump(&build(skin, MinimapClusterState::default()));
    out += &dump(&build(skin, busy_state()));
    out
}

/// The Modern trees as the component built them before it knew skins (master 5e9c5994).
#[test]
fn modern_cluster_is_exactly_the_retail_cluster() {
    let trees = trees(ActiveSkin::Modern);
    println!("<<<MODERN_TREES\n{trees}MODERN_TREES>>>");
    for (line, (actual, expected)) in trees.lines().zip(fixture::MODERN_TREES.lines()).enumerate() {
        assert_eq!(actual, expected, "line {}", line + 1);
    }
    assert_eq!(trees.lines().count(), fixture::MODERN_TREES.lines().count());
}

// ---- Forever ----

use game_engine_core::minimap_data::MapMask;
use game_engine_ui_model::minimap::{cluster_style, minimap_texture_fdids};
use ui_toolkit::widgets::font_string::JustifyH;

fn frame<'a>(registry: &'a FrameRegistry, name: &str) -> &'a Frame {
    registry
        .get(
            registry
                .get_by_name(name)
                .unwrap_or_else(|| panic!("no {name}")),
        )
        .unwrap()
}

/// `(left, top, width, height)` of a frame placed with fixed pixels.
fn fixed_rect(registry: &FrameRegistry, name: &str) -> (f32, f32, f32, f32) {
    let f = frame(registry, name);
    let px = |value: Val| match value {
        Val::Px(px) => px,
        other => panic!("{name}: {other:?} is not pixels"),
    };
    let fixed = |dimension: Dimension| match dimension {
        Dimension::Fixed(px) => px,
        other => panic!("{name}: {other:?} is not fixed"),
    };
    (
        px(f.position.left),
        px(f.position.top),
        fixed(f.width),
        fixed(f.height),
    )
}

fn assert_rect(registry: &FrameRegistry, name: &str, expected: (f32, f32, f32, f32)) {
    let actual = fixed_rect(registry, name);
    let close = [
        (actual.0, expected.0),
        (actual.1, expected.1),
        (actual.2, expected.2),
        (actual.3, expected.3),
    ]
    .iter()
    .all(|(a, b)| (a - b).abs() < 1e-3);
    assert!(close, "{name}: {actual:?} != {expected:?}");
}

/// FlareUI Modules/Minimap.lua:32-34,301-304: 244 map centred in a 260 cluster.
#[test]
fn forever_cluster_is_a_square_244_map_in_a_260_cluster() {
    let registry = build(ActiveSkin::Forever, busy_state());
    let cluster = frame(&registry, MINIMAP_CLUSTER);
    assert_eq!(cluster.width, Dimension::Fixed(260.0));
    assert_eq!(cluster.height, Dimension::Fixed(260.0));
    assert_rect(&registry, MINIMAP_DISPLAY, (8.0, 8.0, 244.0, 244.0));
    let style = cluster_style(ActiveSkin::Forever);
    assert_eq!(style.mask, MapMask::Square);
    assert_eq!((style.cluster_size, style.map_size), (260.0, 244.0));
    // Retail: the round 198×198 `Minimap` in the 256×256 cluster.
    let modern = cluster_style(ActiveSkin::Modern);
    assert_eq!(modern.mask, MapMask::Round);
    assert_eq!((modern.cluster_size, modern.map_size), (256.0, 198.0));
    assert_rect(
        &build(ActiveSkin::Modern, busy_state()),
        MINIMAP_DISPLAY,
        (39.0, 44.0, 198.0, 198.0),
    );
}

/// FlareUI Modules/Minimap.lua:42-47,154-167,178-184,200-208;
/// Forever Blizzard_Minimap/Mainline/GameTime.xml:4 supplies the 19×18 calendar.
#[test]
fn forever_header_holds_the_zone_name_and_the_clock_at_its_top_right() {
    let registry = build(ActiveSkin::Forever, busy_state());
    // Left to right on one 17-high header row: tracking, zone name, clock, calendar.
    let rect = |name: &str| fixed_rect(&registry, name);
    let right = |r: (f32, f32, f32, f32)| r.0 + r.2;
    let background = rect("MinimapClusterTrackingBackground");
    let button = rect("MinimapClusterTrackingButton");
    let zone = rect(MINIMAP_ZONE_TEXT);
    let clock = rect(MINIMAP_CLOCK_TEXT);
    let calendar = rect("GameTimeFrame");
    assert!(
        background.0 <= button.0
            && right(button) <= right(background)
            && background.1 <= button.1
            && button.1 + button.3 <= background.1 + background.3,
        "tracking button {button:?} outside its background {background:?}"
    );
    for (name, r) in [("tracking", background), ("zone", zone), ("clock", clock)] {
        assert_eq!(
            (r.1, r.3),
            (background.1, 17.0),
            "{name} off the header row"
        );
    }
    assert!(right(background) <= zone.0, "zone overlaps tracking");
    assert!(right(zone) <= clock.0, "clock overlaps zone");
    assert!(right(clock) <= calendar.0, "calendar overlaps clock");
    assert_eq!((calendar.2, calendar.3), (19.0, 18.0));
    assert!(right(calendar) <= 260.0, "calendar past the cluster");
    let modern = build(ActiveSkin::Modern, busy_state());
    for name in [
        "MinimapClusterTrackingBackground",
        "MinimapClusterTrackingButton",
        "GameTimeFrame",
    ] {
        assert_eq!(
            frame(&registry, name).widget_data,
            frame(&modern, name).widget_data
        );
        assert_eq!(frame(&registry, name).onclick, frame(&modern, name).onclick);
        assert_eq!(
            frame(&registry, name).mouse_enabled,
            frame(&modern, name).mouse_enabled
        );
        assert!(!frame(&registry, name).hidden);
    }
    let text = |name: &str| match frame(&registry, name).widget_data.as_ref() {
        Some(WidgetData::FontString(text)) => text.clone(),
        other => panic!("{name}: {other:?}"),
    };
    let zone = text(MINIMAP_ZONE_TEXT);
    assert_eq!(zone.text, "Northshire Valley");
    assert_eq!(zone.color, [1.0, 0.82, 0.0, 1.0]);
    assert_eq!(zone.justify_h, JustifyH::Left);
    assert_eq!(
        frame(&registry, MINIMAP_ZONE_TEXT).onclick.as_deref(),
        Some("minimap:world_map")
    );
    let clock = text(MINIMAP_CLOCK_TEXT);
    assert_eq!(clock.text, "3:07");
    assert_eq!(clock.font_size, 12.0); // GameFontHighlight, Fonts.xml:275-277.
    assert_eq!(zone.font_size, 12.0);
    assert!(!zone.word_wrap);
    assert_eq!(clock.justify_h, JustifyH::Right);
    assert_eq!(clock.color, [1.0, 1.0, 1.0, 1.0]);
}

/// Reuse FlareUI bronze backdrop border cells without a centre covering the map.
#[test]
fn forever_border_is_bronze_tooltip_art_instead_of_the_metal_frame() {
    let registry = build(ActiveSkin::Forever, busy_state());
    assert_rect(
        &registry,
        "MinimapClusterFlareBorder",
        (0.0, 0.0, 260.0, 260.0),
    );
    // 16-px tooltip-border cells: corners at the cluster's corners, edges between them;
    // the top and bottom edges are the side cell rotated a quarter turn onto the edge.
    let edge = 16.0;
    let far = 260.0 - edge;
    let span = 260.0 - 2.0 * edge;
    let mid = 130.0;
    let quarter = -std::f32::consts::FRAC_PI_2;
    let mut border_source = None;
    for (part, rect, rotation) in [
        ("TopLeft", (0.0, 0.0, edge, edge), 0.0),
        ("TopRight", (far, 0.0, edge, edge), 0.0),
        ("BottomLeft", (0.0, far, edge, edge), 0.0),
        ("BottomRight", (far, far, edge, edge), 0.0),
        ("Left", (0.0, edge, edge, span), 0.0),
        ("Right", (far, edge, edge, span), 0.0),
        // Unrotated rects centred on the top and bottom rows.
        (
            "Top",
            (mid - edge / 2.0, edge / 2.0 - span / 2.0, edge, span),
            quarter,
        ),
        (
            "Bottom",
            (mid - edge / 2.0, far + edge / 2.0 - span / 2.0, edge, span),
            quarter,
        ),
    ] {
        let name = format!("MinimapClusterBorder{part}");
        assert_rect(&registry, &name, rect);
        let Some(WidgetData::Texture(art)) = &frame(&registry, &name).widget_data else {
            panic!("{name} is not a texture");
        };
        let source = border_source.get_or_insert_with(|| art.source.clone());
        assert_eq!(&art.source, source, "{name} uses another texture");
        assert_eq!(art.vertex_color, [0.65, 0.49, 0.27, 1.0]);
        assert_eq!(art.rotation, rotation);
        assert!(!frame(&registry, &name).hidden);
    }
    let Some(ui_toolkit::widgets::texture::TextureSource::FileDataId(border_fdid)) = border_source
    else {
        panic!("border is not file art: {border_source:?}")
    };
    assert!(registry.get_by_name("MinimapClusterNineSlice").is_none());
    let retail_only = [
        "MinimapCompassTexture",
        "MinimapClusterBorderTopTopLeftCorner",
    ];
    let modern = build(ActiveSkin::Modern, busy_state());
    for name in retail_only {
        assert!(registry.get_by_name(name).is_none(), "{name} under Forever");
        assert!(modern.get_by_name(name).is_some(), "{name} under Modern");
    }
    assert!(modern.get_by_name("MinimapClusterNineSlice").is_none());
    // The host also caches the reused bronze border and calendar art.
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let fdids = minimap_texture_fdids(&busy_state());
    for fdid in [border_fdid, 4_618_663] {
        assert!(fdids.contains(&fdid), "{fdid}");
    }
}

/// Blips, arrow and hover zoom offsets unchanged; centred map now has centre (130, 130).
#[test]
fn forever_map_marks_follow_the_square_map() {
    let registry = build(ActiveSkin::Forever, busy_state());
    // `(width, height)` centred on `(x, y)`.
    let centred = |(x, y): (f32, f32), (w, h): (f32, f32)| (x - w / 2.0, y - h / 2.0, w, h);
    let (map_x, map_y, map_size, _) = fixed_rect(&registry, MINIMAP_DISPLAY);
    let centre = (map_x + map_size / 2.0, map_y + map_size / 2.0);
    assert_rect(&registry, MINIMAP_ARROW, centred(centre, (32.0, 32.0)));
    // 16×16 blips centred at origin + map size × (0.5 + offset).
    let on_map = |[x, y]: [f32; 2]| (map_x + map_size * (0.5 + x), map_y + map_size * (0.5 + y));
    for (name, offset) in [
        ("MinimapBlip7", [0.25, -0.25]),
        ("MinimapBlip8", [-0.1, 0.3]),
        ("MinimapVignette9", [0.0, 0.0]),
        ("MinimapVignette10", CORNER_BLIP),
    ] {
        assert_rect(&registry, name, centred(on_map(offset), (16.0, 16.0)));
    }
    // ZoomIn 17×17 at CENTER (+88, −68), ZoomOut 17×9 at (+72, −84) (Minimap.xml:190-219).
    let from_centre = |dx: f32, dy: f32| (centre.0 + dx, centre.1 - dy);
    assert_rect(
        &registry,
        MINIMAP_ZOOM_IN,
        centred(from_centre(88.0, -68.0), (17.0, 17.0)),
    );
    assert_rect(
        &registry,
        MINIMAP_ZOOM_OUT,
        centred(from_centre(72.0, -84.0), (17.0, 9.0)),
    );
    // Modules/Minimap.lua:41,44,50,226: mail below the 22-high title band + 3 gap.
    assert_rect(&registry, MINIMAP_MAIL_FRAME, (14.0, 33.0, 20.0, 15.0));
}

/// One Screen re-synced with the other skin rebuilds the cluster, then Modern's frames
/// again. A re-sync appends the frames it creates, so the host builds another skin's
/// cluster on a fresh canvas to keep the draw order (`GameClient::sync_minimap`).
#[test]
fn switching_skin_reshapes_the_live_cluster() {
    let frames = |registry: &FrameRegistry| {
        let tree = dump(registry);
        let mut lines: Vec<String> = tree.lines().map(str::to_owned).collect();
        lines.sort();
        lines
    };
    let mut shared = SharedContext::new();
    shared.insert(ActiveSkin::Modern);
    shared.insert(busy_state());
    let mut registry = FrameRegistry::new(1920.0, 1080.0);
    let mut screen = Screen::new(minimap_cluster_screen);
    screen.sync(&shared, &mut registry);
    let modern = frames(&registry);
    assert_eq!(fixed_rect(&registry, MINIMAP_DISPLAY).2, 198.0);

    shared.insert(ActiveSkin::Forever);
    screen.sync(&shared, &mut registry);
    assert_eq!(fixed_rect(&registry, MINIMAP_DISPLAY).2, 244.0);
    assert!(registry.get_by_name("MinimapCompassTexture").is_none());

    shared.insert(ActiveSkin::Modern);
    screen.sync(&shared, &mut registry);
    assert_eq!(frames(&registry), modern);
}
