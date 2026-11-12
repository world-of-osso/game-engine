//! Counters of the expensive scene and UI rebuilds a steady frame must not repeat: each
//! category counts how often its operation ran and keeps the first call site since the
//! last `RebuildCounters.take()`, so a per-frame rebuild that slips in shows up in the
//! frame benchmarks' `rebuilds_per_frame` and fails `tests/rebuild_guard_live.gd`.
//! A per-frame path that touches materials, environment or meshes must be change-driven
//! and count here (AGENTS.md).

use std::{
    backtrace::Backtrace,
    panic::Location,
    sync::{
        Mutex,
        atomic::{AtomicU64, Ordering},
    },
};

use godot::{
    classes::{
        ArrayMesh, Control, Font, Object, RefCounted, RenderingServer, ShaderMaterial, StyleBox,
    },
    obj::NewGd,
    prelude::*,
};

#[derive(Clone, Copy)]
pub(crate) enum Rebuild {
    /// Sky cubemap faces built or re-uploaded.
    SkyCubemap,
    /// A new world light applied to the scene: environment, sky dome and sun.
    Environment,
    /// Materials whose light or fog parameters were rebound.
    MaterialRebind,
    /// Global shader parameters set (each rewrites the global uniform buffer).
    ShaderBroadcast,
    /// `ArrayMesh`es created or mesh surfaces rebuilt.
    MeshBuild,
    /// `ShaderMaterial`s created or duplicated.
    ShaderMaterial,
    /// Theme overrides set and style boxes created (each re-themes the control).
    Theme,
    /// UI and nameplate controls created, freed or re-parented by a rebuild.
    UiTree,
}

const CATEGORIES: [(Rebuild, &str); 8] = [
    (Rebuild::SkyCubemap, "sky_cubemap"),
    (Rebuild::Environment, "environment"),
    (Rebuild::MaterialRebind, "material_rebind"),
    (Rebuild::ShaderBroadcast, "shader_broadcast"),
    (Rebuild::MeshBuild, "mesh_build"),
    (Rebuild::ShaderMaterial, "shader_material"),
    (Rebuild::Theme, "theme"),
    (Rebuild::UiTree, "ui_tree"),
];

static COUNTS: [AtomicU64; CATEGORIES.len()] = [const { AtomicU64::new(0) }; CATEGORIES.len()];
static FIRST_SITES: Mutex<[Option<FirstSite>; CATEGORIES.len()]> =
    Mutex::new([const { None }; CATEGORIES.len()]);

/// Where a category first counted since the last take. The backtrace is captured
/// unresolved and only symbolized by `take`, outside measured frames.
struct FirstSite {
    location: &'static Location<'static>,
    detail: Option<String>,
    backtrace: Backtrace,
}

impl FirstSite {
    /// `file:line via caller < caller ...`, the nearest extension frames above the site.
    fn describe(&self) -> String {
        let trace = self.backtrace.to_string();
        let callers: Vec<&str> = trace
            .lines()
            .filter_map(|line| line.trim().split_once(": "))
            .map(|(_, symbol)| symbol)
            .filter(|symbol| {
                symbol.contains("game_engine_godot::") && !symbol.contains("rebuild_guard")
            })
            .take(6)
            .collect();
        let mut site = format!("{}:{}", self.location.file(), self.location.line());
        if let Some(detail) = &self.detail {
            site = format!("{site} ({detail})");
        }
        if callers.is_empty() {
            format!(
                "{site} RAW {}",
                trace.lines().take(12).collect::<Vec<_>>().join(" | ")
            )
        } else {
            format!("{site} via {}", callers.join(" < "))
        }
    }
}

/// Counts `amount` operations of `kind` at the caller's call site.
#[track_caller]
pub(crate) fn count(kind: Rebuild, amount: u64) {
    count_detail(kind, amount, || None);
}

/// `count`, naming what was rebuilt (a frame or unit) when it is the first since the take.
#[track_caller]
pub(crate) fn count_detail(kind: Rebuild, amount: u64, detail: impl FnOnce() -> Option<String>) {
    let index = kind as usize;
    if COUNTS[index].fetch_add(amount, Ordering::Relaxed) == 0 {
        // Read here: a closure does not inherit `#[track_caller]`.
        let location = Location::caller();
        let mut sites = FIRST_SITES.lock().expect("rebuild sites lock");
        sites[index].get_or_insert_with(|| FirstSite {
            location,
            detail: detail(),
            backtrace: Backtrace::force_capture(),
        });
    }
}

#[track_caller]
pub(crate) fn new_shader_material() -> Gd<ShaderMaterial> {
    count(Rebuild::ShaderMaterial, 1);
    <ShaderMaterial as NewGd>::new_gd()
}

#[track_caller]
pub(crate) fn new_array_mesh() -> Gd<ArrayMesh> {
    count(Rebuild::MeshBuild, 1);
    <ArrayMesh as NewGd>::new_gd()
}

#[track_caller]
pub(crate) fn new_style_box<T: NewGd + Inherits<StyleBox>>() -> Gd<T> {
    count(Rebuild::Theme, 1);
    T::new_gd()
}

#[track_caller]
pub(crate) fn set_global_shader_parameter(
    server: &mut Gd<RenderingServer>,
    name: &str,
    value: &Variant,
) {
    count(Rebuild::ShaderBroadcast, 1);
    server.global_shader_parameter_set(name, value);
}

/// Godot's theme overrides, set and counted only when the value changes: every set
/// notifies the control that its theme changed (scene/gui/control.cpp
/// `_notify_theme_override_changed`), and a label reshapes its text.
pub(crate) trait ThemeOverrides {
    fn change_theme_color_override(&mut self, name: &str, color: Color);
    fn change_theme_constant_override(&mut self, name: &str, constant: i32);
    fn change_theme_font_override<F: Inherits<Font>>(&mut self, name: &str, font: &Gd<F>);
    fn change_theme_font_size_override(&mut self, name: &str, size: i32);
    fn change_theme_stylebox_override<S: Inherits<StyleBox>>(
        &mut self,
        name: &str,
        stylebox: &Gd<S>,
    );
}

impl<T: Inherits<Control>> ThemeOverrides for Gd<T> {
    #[track_caller]
    fn change_theme_color_override(&mut self, name: &str, color: Color) {
        let control = self.upcast_mut();
        if control.has_theme_color_override(name) && control.get_theme_color(name) == color {
            return;
        }
        count_detail(Rebuild::Theme, 1, || {
            Some(format!("{} {name}", node_label(control)))
        });
        control.add_theme_color_override(name, color);
    }

    #[track_caller]
    fn change_theme_constant_override(&mut self, name: &str, constant: i32) {
        let control = self.upcast_mut();
        if control.has_theme_constant_override(name) && control.get_theme_constant(name) == constant
        {
            return;
        }
        count_detail(Rebuild::Theme, 1, || {
            Some(format!("{} {name}", node_label(control)))
        });
        control.add_theme_constant_override(name, constant);
    }

    #[track_caller]
    fn change_theme_font_override<F: Inherits<Font>>(&mut self, name: &str, font: &Gd<F>) {
        let control = self.upcast_mut();
        if control.has_theme_font_override(name)
            && control
                .get_theme_font(name)
                .map(|current| current.instance_id())
                == Some(font.instance_id())
        {
            return;
        }
        count_detail(Rebuild::Theme, 1, || {
            Some(format!("{} {name}", node_label(control)))
        });
        control.add_theme_font_override(name, font);
    }

    #[track_caller]
    fn change_theme_font_size_override(&mut self, name: &str, size: i32) {
        let control = self.upcast_mut();
        if control.has_theme_font_size_override(name) && control.get_theme_font_size(name) == size {
            return;
        }
        count_detail(Rebuild::Theme, 1, || {
            Some(format!("{} {name}", node_label(control)))
        });
        control.add_theme_font_size_override(name, size);
    }

    #[track_caller]
    fn change_theme_stylebox_override<S: Inherits<StyleBox>>(
        &mut self,
        name: &str,
        stylebox: &Gd<S>,
    ) {
        let control = self.upcast_mut();
        if control.has_theme_stylebox_override(name)
            && control
                .get_theme_stylebox(name)
                .map(|current| current.instance_id())
                == Some(stylebox.instance_id())
        {
            return;
        }
        count_detail(Rebuild::Theme, 1, || {
            Some(format!("{} {name}", node_label(control)))
        });
        control.add_theme_stylebox_override(name, stylebox);
    }
}

/// A node's scene path, or its name before it enters the tree (`get_path` errors then).
pub(crate) fn node_label(node: &Node) -> String {
    if node.is_inside_tree() {
        node.get_path().to_string()
    } else {
        node.get_name().to_string()
    }
}

/// Godot objects created so far: the ObjectDB validator counter (core/object/object.cpp
/// `ObjectDB::add_instance`) advances once per object and sits above the 24 slot bits of
/// every instance id (object.h `OBJECTDB_SLOT_MAX_COUNT_BITS`, `OBJECTDB_VALIDATOR_BITS`).
fn objects_created() -> u64 {
    let probe = RefCounted::new_gd();
    (probe.instance_id().to_i64() as u64 >> 24) & ((1 << 39) - 1)
}

static OBJECTS_AT_TAKE: AtomicU64 = AtomicU64::new(0);
static WORLD_EVENTS: AtomicU64 = AtomicU64::new(0);

/// A unit visual entered the scene: the world changed, so the frame was not steady and
/// its builds are not per-frame churn (the guard measures again).
pub(crate) fn world_event() {
    WORLD_EVENTS.fetch_add(1, Ordering::Relaxed);
}

/// Script access to the counters, for the frame benchmarks and the rebuild guard test.
#[derive(GodotClass)]
#[class(base = Object, init)]
pub struct RebuildCounters {
    base: Base<Object>,
}

#[godot_api]
impl RebuildCounters {
    /// Every category's count and first call site since the previous take, then resets
    /// them: `{counts: {category: n}, sites: {category: "file:line via callers"},
    /// world_events: n, objects_created: n}`.
    /// `objects_created` excludes the probe objects of the two takes.
    #[func]
    fn take() -> VarDictionary {
        let mut counts = VarDictionary::new();
        let mut sites = VarDictionary::new();
        let mut first = FIRST_SITES.lock().expect("rebuild sites lock");
        for (index, (_, name)) in CATEGORIES.iter().enumerate() {
            counts.set(*name, COUNTS[index].swap(0, Ordering::Relaxed) as i64);
            if let Some(site) = first[index].take() {
                sites.set(*name, site.describe());
            }
        }
        drop(first);
        let objects = objects_created();
        let previous = OBJECTS_AT_TAKE.swap(objects, Ordering::Relaxed);
        let mut taken = VarDictionary::new();
        taken.set("counts", &counts);
        taken.set("sites", &sites);
        taken.set(
            "world_events",
            WORLD_EVENTS.swap(0, Ordering::Relaxed) as i64,
        );
        taken.set(
            "objects_created",
            objects.saturating_sub(previous).saturating_sub(1) as i64,
        );
        taken
    }
}
