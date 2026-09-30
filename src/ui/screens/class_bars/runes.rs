//! `RuneButtonIndividualTemplate` / `RuneFrame` (RuneFrame.xml, RuneFrame.lua), atlas
//! 2273 `uideathknightrunes.blp` FDID 4876501, art set by spec (`RuneArtSet`).
//!
//! Retail reads each rune's cooldown from `GetRuneCooldown`. The server replicates only
//! the rune pool (0..6, regenerating `power_type` RegenPeace/RegenCombat 0.1 per second:
//! one rune at a time, 10 s each), so the cooldowns are rebuilt from that: spent runes
//! queue behind the recharging one, which starts when the previous one came back or when
//! a rune is spent from full.

use super::super::inworld_unit_frames_art::{AtlasArt, FlipBook, art, key};
use super::anim::{AnimGroup, Layer, PointTemplate, PointVisual, group, layer, translate};
use super::{BarLogic, ClassBarView, Row, TextureView};
use crate::status::{ClassBar, ClassBarResource};

const fn rune(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_876_501, (512.0, 256.0), rect)
}

const BG_INACTIVE: usize = 1;
const BG_ACTIVE: usize = 2;
const RUNE_INACTIVE: usize = 3;
const GRAD: usize = 4;
const LINES: usize = 5;
const ACTIVE: usize = 6;
const MID: usize = 7;
const EYES: usize = 8;
const GLOW: usize = 9;
const GLOW2: usize = 10;
const SMOKE: usize = 11;
/// `DepleteVisuals` (RuneFrame.xml:93-132), a child frame over the rune.
const DEPLETE_INACTIVE: usize = 12;
const DEPLETE_LINES: usize = 13;
const DEPLETE_FLIPBOOK: usize = 14;
const DEPLETE_GLOW2: usize = 15;
const DEPLETE_VISUALS: std::ops::Range<usize> = 12..16;

const FILL_ANIM: usize = 0;
const ENDING_ANIM: usize = 1;
const EMPTY_ANIM: usize = 2;
const DEPLETE_ANIM: usize = 3;

/// One `RuneArtSet` (RuneFrame.lua:107-127): `UF-DKRunes-<set>-SkullGrad`, `-SkullLines`,
/// `-SkullActive`, `-SkullMid`, `-Eyes`, `-FilledGlwA`, `-FilledGlwB`, `-Smoke`,
/// `UF-DKRunes-<set>Deplete` and the `-LevelBar` cooldown swipe.
struct ArtSet {
    grad: (f32, f32, f32, f32),
    lines: (f32, f32, f32, f32),
    active: (f32, f32, f32, f32),
    mid: (f32, f32, f32, f32),
    eyes: (f32, f32, f32, f32),
    glow: (f32, f32, f32, f32),
    glow2: (f32, f32, f32, f32),
    smoke: (f32, f32, f32, f32),
    deplete: (f32, f32, f32, f32),
    level_bar: (f32, f32, f32, f32),
}

/// Default (21322..21328), Blood (19115..19126, 19256), Frost (19127..19139), Unholy
/// (19143..19155).
const ART_SETS: [ArtSet; 4] = [
    ArtSet {
        grad: (152.0, 168.0, 228.0, 245.0),
        lines: (336.0, 352.0, 147.0, 163.0),
        active: (123.0, 139.0, 234.0, 251.0),
        mid: (208.0, 224.0, 231.0, 248.0),
        eyes: (342.0, 360.0, 166.0, 177.0),
        glow: (30.0, 57.0, 176.0, 203.0),
        glow2: (30.0, 57.0, 205.0, 232.0),
        smoke: (208.0, 235.0, 176.0, 199.0),
        deplete: (123.0, 243.0, 1.0, 145.0),
        level_bar: (152.0, 175.0, 203.0, 226.0),
    },
    ArtSet {
        grad: (59.0, 75.0, 234.0, 251.0),
        lines: (227.0, 243.0, 239.0, 255.0),
        active: (30.0, 46.0, 234.0, 251.0),
        mid: (88.0, 104.0, 234.0, 251.0),
        eyes: (302.0, 320.0, 166.0, 177.0),
        glow: (1.0, 28.0, 205.0, 232.0),
        glow2: (30.0, 57.0, 147.0, 174.0),
        smoke: (179.0, 206.0, 176.0, 199.0),
        deplete: (1.0, 121.0, 1.0, 145.0),
        level_bar: (59.0, 86.0, 147.0, 174.0),
    },
    ArtSet {
        grad: (227.0, 243.0, 220.0, 237.0),
        lines: (264.0, 280.0, 166.0, 182.0),
        active: (227.0, 243.0, 201.0, 218.0),
        mid: (245.0, 261.0, 237.0, 254.0),
        eyes: (302.0, 320.0, 179.0, 190.0),
        glow: (88.0, 115.0, 176.0, 203.0),
        glow2: (59.0, 86.0, 205.0, 232.0),
        smoke: (179.0, 206.0, 201.0, 224.0),
        deplete: (245.0, 365.0, 1.0, 145.0),
        level_bar: (88.0, 115.0, 205.0, 232.0),
    },
    ArtSet {
        grad: (300.0, 316.0, 147.0, 164.0),
        lines: (264.0, 280.0, 184.0, 200.0),
        active: (282.0, 298.0, 147.0, 164.0),
        mid: (318.0, 334.0, 147.0, 164.0),
        eyes: (342.0, 360.0, 179.0, 190.0),
        glow: (123.0, 150.0, 205.0, 232.0),
        glow2: (152.0, 179.0, 147.0, 174.0),
        smoke: (179.0, 206.0, 226.0, 249.0),
        deplete: (367.0, 487.0, 1.0, 145.0),
        level_bar: (181.0, 208.0, 147.0, 174.0),
    },
];

/// UF-DKRunes-SkullDis (19140).
const SKULL_DIS: (f32, f32, f32, f32) = (264.0, 280.0, 147.0, 164.0);

/// The rune's textures (RuneFrame.xml:6-117) in one art set.
const fn layers(set: &ArtSet) -> [Layer; 16] {
    [
        // BACKGROUND 0: UF-DKRunes-BGShadow (19114) y -3; 1: -BGDis (19113), -BGActive
        // (19112).
        layer("BG_Shadow", rune((152.0, 177.0, 176.0, 201.0)), (0.0, -3.0)),
        layer("BG_Inactive", rune((1.0, 28.0, 176.0, 203.0)), (0.0, 0.0)),
        layer("BG_Active", rune((1.0, 28.0, 147.0, 174.0)), (0.0, 0.0)),
        // ARTWORK 0..3.
        layer("Rune_Inactive", rune(SKULL_DIS), (0.0, 0.0)),
        layer("Rune_Grad", rune(set.grad), (0.0, 0.0)),
        layer("Rune_Lines", rune(set.lines), (0.0, 0.0)),
        layer("Rune_Active", rune(set.active), (0.0, 0.0)),
        layer("Rune_Mid", rune(set.mid), (0.0, 0.0)),
        layer("Rune_Eyes", rune(set.eyes), (0.0, 1.0)),
        // OVERLAY 1: glows and the smoke at y -4.
        layer("Glow", rune(set.glow), (0.0, 0.0)),
        layer("Glow2", rune(set.glow2), (0.0, 0.0)),
        layer("Smoke", rune(set.smoke), (0.0, -4.0)),
        // DepleteVisuals, hidden until its DepleteAnim plays: skull, lines, the 20×36
        // deplete flipbook at y 10, and FilledGlwB.
        layer("DepleteRune_Inactive", rune(SKULL_DIS), (0.0, 0.0)).hidden(),
        layer("DepleteRune_Lines", rune(set.lines), (0.0, 0.0)).hidden(),
        layer("FB_RuneDeplete", rune(set.deplete), (0.0, 10.0))
            .sized((20.0, 36.0))
            .hidden(),
        layer("DepleteGlow2", rune(set.glow2), (0.0, 0.0)).hidden(),
    ]
}

static LAYERS: [[Layer; 16]; 4] = [
    layers(&ART_SETS[0]),
    layers(&ART_SETS[1]),
    layers(&ART_SETS[2]),
    layers(&ART_SETS[3]),
];

static GROUPS: [AnimGroup; 4] = [
    // CooldownFillAnim, authored for an 8 s cooldown (RuneFrame.xml:142-152).
    group(
        &[
            (MID, key(0.0, 0.0, 0.0, 0.1)),
            (EYES, key(0.0, 0.0, 0.0, 0.1)),
            (SMOKE, key(0.0, 0.0, 0.0, 0.1)),
            (GLOW, key(0.0, 0.0, 0.0, 0.1)),
            (GRAD, key(0.0, 0.0, 0.0, 0.86)),
            (LINES, key(0.0, 0.0, 0.85, 0.1)),
            (LINES, key(0.0, 0.16, 0.86, 3.18)),
            (GRAD, key(0.0, 0.3, 0.86, 3.18)),
            (LINES, key(0.16, 0.3, 4.0, 2.83)),
        ],
        &[],
    ),
    // CooldownEndingAnim (RuneFrame.xml:154-172).
    group(
        &[
            (MID, key(0.0, 1.0, 0.0, 0.47)),
            (EYES, key(0.0, 1.0, 0.24, 0.43)),
            (SMOKE, key(0.0, 1.0, 0.34, 0.5)),
            (LINES, key(1.0, 0.0, 0.67, 0.17)),
            (GRAD, key(1.0, 0.0, 0.67, 0.01)),
            (RUNE_INACTIVE, key(1.0, 0.0, 0.67, 0.01)),
            (BG_ACTIVE, key(0.0, 1.0, 0.67, 0.01)),
            (BG_INACTIVE, key(1.0, 0.0, 0.67, 0.01)),
            (GLOW, key(1.0, 1.0, 0.67, 0.0)),
            (GLOW2, key(1.0, 1.0, 0.67, 0.0)),
            (ACTIVE, key(0.0, 1.0, 0.77, 0.01)),
            (MID, key(1.0, 0.0, 0.77, 0.4)),
            (SMOKE, key(1.0, 0.0, 0.83, 0.2)),
            (GLOW, key(1.0, 0.0, 0.84, 0.23)),
            (GLOW2, key(1.0, 0.0, 1.0, 0.17)),
            (EYES, key(1.0, 0.0, 1.14, 0.5)),
        ],
        &[],
    )
    .moving(&[(SMOKE, translate(0.0, 12.0, 0.34, 0.7))]),
    // EmptyAnim (RuneFrame.xml:173-179).
    group(
        &[
            (BG_ACTIVE, key(0.0, 0.0, 0.0, 0.1)),
            (BG_INACTIVE, key(1.0, 1.0, 0.0, 0.1)),
            (ACTIVE, key(0.0, 0.0, 0.0, 0.1)),
            (RUNE_INACTIVE, key(1.0, 0.4, 0.0, 0.1)),
            (LINES, key(1.0, 0.0, 0.0, 0.1)),
        ],
        &[],
    ),
    // DepleteVisuals.DepleteAnim (RuneFrame.xml:120-130).
    group(
        &[
            (DEPLETE_INACTIVE, key(1.0, 1.0, 0.0, 0.1)),
            (DEPLETE_FLIPBOOK, key(0.0, 1.0, 0.0, 0.1)),
            (DEPLETE_INACTIVE, key(1.0, 0.4, 0.0, 0.67)),
            (DEPLETE_GLOW2, key(1.0, 0.0, 0.0, 0.4)),
            (DEPLETE_LINES, key(1.0, 1.0, 0.0, 0.4)),
            (DEPLETE_LINES, key(1.0, 0.0, 0.4, 0.43)),
            (DEPLETE_FLIPBOOK, key(1.0, 0.0, 1.0, 0.1)),
            (DEPLETE_INACTIVE, key(0.4, 0.0, 1.0, 0.1)),
        ],
        &[(
            DEPLETE_FLIPBOOK,
            FlipBook {
                columns: 6,
                rows: 4,
                frames: 23,
                duration: 1.0,
            },
        )],
    ),
];

static TEMPLATES: [PointTemplate; 4] = [
    PointTemplate {
        layers: &LAYERS[0],
        groups: &GROUPS,
    },
    PointTemplate {
        layers: &LAYERS[1],
        groups: &GROUPS,
    },
    PointTemplate {
        layers: &LAYERS[2],
        groups: &GROUPS,
    },
    PointTemplate {
        layers: &LAYERS[3],
        groups: &GROUPS,
    },
];

/// `ArtTypeBySpec`: Blood 250, Frost 251, Unholy 252; Default before a spec is chosen.
fn art_set(spec: Option<u32>) -> usize {
    match spec {
        Some(250) => 1,
        Some(251) => 2,
        Some(252) => 3,
        _ => 0,
    }
}

/// `cooldownFillAnimBasisSeconds`, `cooldownEndingOffsetSeconds` (RuneFrame.xml:136-138).
const FILL_BASIS: f32 = 8.0;
const ENDING_OFFSET: f64 = 0.67;

/// `RuneButtonMixin.VisualState`, ordered as `CompareRuneButtons` sorts them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum VisualState {
    Empty = 1,
    OnCooldown = 2,
    CooldownEnding = 3,
    Ready = 4,
}

/// `GetRuneCooldown(runeIndex)`.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RuneCooldown {
    start: f64,
    duration: f64,
    ready: bool,
}

#[derive(Debug)]
struct RuneButton {
    index: usize,
    visual: PointVisual,
    state: Option<VisualState>,
    last: Option<RuneCooldown>,
    cooldown_ending_start: Option<f64>,
    /// The Cooldown frame's `(start, duration)` while it tracks one.
    cooldown: Option<(f64, f64)>,
    newly_depleted: bool,
}

impl RuneButton {
    fn new(index: usize, spec: Option<u32>) -> Self {
        Self {
            index,
            visual: PointVisual::new(&TEMPLATES[art_set(spec)]),
            state: None,
            last: None,
            cooldown_ending_start: None,
            cooldown: None,
            newly_depleted: false,
        }
    }

    /// `SkipToFinalAnimState`: restart the group at its end.
    fn skip_to_end(&mut self, group: usize, now: f64) {
        let duration = GROUPS[group].duration();
        self.visual.play(group, now, duration, 1.0);
        self.visual.finish(now);
    }

    /// `RuneButtonMixin:UpdateState` (RuneFrame.lua:171-189).
    fn update_state(&mut self, cooldown: RuneCooldown, now: f64) {
        let previous = self.state;
        self.newly_depleted = false;
        self.last = Some(cooldown);
        if cooldown.ready {
            self.show_as_ready(previous, now);
        } else {
            self.show_as_on_cooldown(cooldown, previous, now);
        }
    }

    /// `ShowAsReady` (RuneFrame.lua:209-227).
    fn show_as_ready(&mut self, previous: Option<VisualState>, now: f64) {
        for group in [EMPTY_ANIM, FILL_ANIM] {
            if self.visual.is_playing(group) {
                self.skip_to_end(group, now);
            }
        }
        if !self.visual.is_playing(ENDING_ANIM) && previous != Some(VisualState::Ready) {
            self.skip_to_end(ENDING_ANIM, now);
        }
        self.state = Some(VisualState::Ready);
        self.cooldown_ending_start = None;
        self.cooldown = None;
    }

    /// `ShowAsEmpty`: no received cooldown start/duration is available.
    fn show_as_empty(&mut self, now: f64) {
        self.newly_depleted = false;
        if self.state != Some(VisualState::Empty) {
            self.visual.stop(FILL_ANIM, now);
            self.visual.stop(ENDING_ANIM, now);
            self.skip_to_end(EMPTY_ANIM, now);
        }
        self.state = Some(VisualState::Empty);
        self.cooldown = None;
        self.cooldown_ending_start = None;
    }

    /// `ShowAsOnCooldown` (RuneFrame.lua:245-301).
    fn show_as_on_cooldown(
        &mut self,
        RuneCooldown {
            start, duration, ..
        }: RuneCooldown,
        previous: Option<VisualState>,
        now: f64,
    ) {
        const EPSILON: f64 = 0.01;
        let same_end = self.cooldown.is_some_and(|(old_start, old_duration)| {
            ((old_start + old_duration) - (start + duration)).abs() <= EPSILON
        });
        if same_end && self.visual.is_playing(FILL_ANIM) {
            return;
        }
        let ending_start = start + duration - ENDING_OFFSET;
        self.cooldown_ending_start = Some(ending_start);
        let before_ending = now.floor() < ending_start.floor();
        if previous.is_none() || (before_ending && self.visual.is_playing(ENDING_ANIM)) {
            self.skip_to_end(ENDING_ANIM, now);
        }
        let restart_empty = match previous {
            None | Some(VisualState::Ready) => true,
            Some(VisualState::CooldownEnding) => before_ending,
            _ => false,
        };
        if restart_empty {
            self.visual.restart(EMPTY_ANIM, now);
            self.state = Some(VisualState::Empty);
            self.newly_depleted = true;
        }
        self.cooldown = Some((start, duration));
        if before_ending && now.floor() >= start.floor() {
            let speed = FILL_BASIS / duration as f32;
            let offset = (now - start) as f32;
            let current = self.visual.elapsed(FILL_ANIM, now);
            let in_step = current.is_some_and(|elapsed| (elapsed - offset * speed).abs() <= 0.01);
            if !in_step {
                self.visual.play(FILL_ANIM, now, offset * speed, speed);
            }
            self.state = Some(VisualState::OnCooldown);
        }
    }

    /// The Cooldown frame's `OnUpdate` → `OnCooldownUpdate` (RuneFrame.lua:303-320).
    fn on_cooldown_update(&mut self, now: f64) {
        let Some(ending_start) = self.cooldown_ending_start else {
            return;
        };
        if now >= ending_start {
            self.visual.stop(FILL_ANIM, now);
            self.visual
                .play(ENDING_ANIM, now, (now - ending_start) as f32, 1.0);
            self.cooldown_ending_start = None;
            self.state = Some(VisualState::CooldownEnding);
        }
    }

    /// `PlayDepleteVisuals`: DepleteVisuals shows while its group plays.
    fn play_deplete(&mut self, now: f64) {
        for layer in DEPLETE_VISUALS {
            self.visual.set_shown(layer, true);
        }
        self.visual.restart(DEPLETE_ANIM, now);
    }

    fn stop_deplete(&mut self, now: f64) {
        self.visual.stop(DEPLETE_ANIM, now);
        for layer in DEPLETE_VISUALS {
            self.visual.set_shown(layer, false);
        }
    }

    fn tick(&mut self, now: f64) {
        for (group, _) in self.visual.finish(now) {
            if group == DEPLETE_ANIM {
                for layer in DEPLETE_VISUALS {
                    self.visual.set_shown(layer, false);
                }
            }
        }
        self.on_cooldown_update(now);
    }

    /// `CompareRuneButtons`: does `self` sort before `other`.
    fn sorts_before(&self, other: &Self, now: f64) -> bool {
        let (Some(a), Some(b)) = (self.state, other.state) else {
            if self.state.is_none() && other.state.is_none() {
                return self.index > other.index;
            }
            return self.state.is_some();
        };
        if a != b {
            return a > b;
        }
        if a == VisualState::Ready {
            let (a_end, b_end) = (
                self.visual.is_playing(ENDING_ANIM),
                other.visual.is_playing(ENDING_ANIM),
            );
            if a_end != b_end {
                return !a_end;
            }
            let (a_progress, b_progress) = (
                self.visual.progress(ENDING_ANIM, now),
                other.visual.progress(ENDING_ANIM, now),
            );
            if a_progress != b_progress {
                return a_progress > b_progress;
            }
        }
        let start = |button: &Self| button.last.map_or(0.0, |last| last.start);
        if start(self) != start(other) {
            return start(self) < start(other);
        }
        self.index > other.index
    }
}

/// `GetRuneCooldown`: the server supplies each rune's own remaining time. Never infer
/// rune identity or cooldowns from the aggregate ready count.
fn received_cooldown(resource: &ClassBarResource, index: usize) -> Option<RuneCooldown> {
    let runes = resource.dynamics.runes.as_ref()?;
    let ready_in_ms = *runes.ready_in_ms.get(index)?;
    let duration = f64::from(runes.duration_ms) / 1000.0;
    Some(RuneCooldown {
        start: resource.dynamics.received_at + f64::from(ready_in_ms) / 1000.0 - duration,
        duration,
        ready: ready_in_ms == 0,
    })
}

/// `RuneFrame`: 24×24 runes 1 px overlapped at scale 0.95, `topPadding` 6, `leftPadding`
/// -5 (RuneFrame.xml:5,188,233-238).
const ROW: Row = Row {
    cell: (24.0, 24.0),
    spacing: -1.0,
    padding: (6.0, -5.0),
    scale: 0.95,
};

#[derive(Debug)]
pub struct Bar {
    runes: Vec<RuneButton>,
    /// `runeIndices` in layout order.
    order: Vec<usize>,
    spec: Option<Option<u32>>,
}

impl Default for Bar {
    fn default() -> Self {
        Self {
            runes: (0..6).map(|index| RuneButton::new(index, None)).collect(),
            order: (0..6).collect(),
            spec: None,
        }
    }
}

impl BarLogic for Bar {
    /// `RuneFrameMixin:UpdateRunes` (RuneFrame.lua:45-85) on `RUNE_POWER_UPDATE`, with
    /// `UpdateSpec` on a spec change.
    fn power(&mut self, resource: &ClassBarResource, now: f64) {
        if self.spec != Some(resource.spec) {
            self.spec = Some(resource.spec);
            for rune in &mut self.runes {
                rune.visual.set_template(&TEMPLATES[art_set(resource.spec)]);
            }
        }
        let mut depleted = 0;
        for (index, rune) in self.runes.iter_mut().enumerate() {
            match received_cooldown(resource, index) {
                Some(cooldown) if cooldown.ready || cooldown.duration > 0.0 => {
                    rune.update_state(cooldown, now);
                }
                _ => rune.show_as_empty(now),
            }
            depleted += usize::from(rune.newly_depleted);
        }
        let runes = &self.runes;
        let mut order: Vec<usize> = self.order.clone();
        order.sort_by(|&a, &b| {
            if runes[a].sorts_before(&runes[b], now) {
                std::cmp::Ordering::Less
            } else if runes[b].sorts_before(&runes[a], now) {
                std::cmp::Ordering::Greater
            } else {
                std::cmp::Ordering::Equal
            }
        });
        for (position, &rune) in order.iter().enumerate() {
            let moved = self.order.iter().position(|&r| r == rune) != Some(position);
            let button = &mut self.runes[rune];
            if moved && button.visual.is_playing(DEPLETE_ANIM) {
                button.stop_deplete(now);
            }
            if depleted > 0 && button.state != Some(VisualState::Ready) {
                button.play_deplete(now);
                depleted -= 1;
            }
        }
        self.order = order;
    }

    fn tick(&mut self, now: f64) {
        for rune in &mut self.runes {
            rune.tick(now);
        }
    }

    fn view(&self, _: &ClassBarResource, now: f64) -> ClassBarView {
        let visuals = self.order.iter().map(|&rune| &self.runes[rune].visual);
        let mut view = ROW.view(ClassBar::Runes, visuals, now);
        view.textures.extend(self.swipes(now));
        view
    }
}

impl Bar {
    /// Each tracking Cooldown frame: 27×27, `reverse`, swiping the spec's `-LevelBar`.
    fn swipes(&self, now: f64) -> Vec<TextureView> {
        let set = &ART_SETS[art_set(self.spec.flatten())];
        let (cell, spacing, scale) = (ROW.cell.0, ROW.spacing, ROW.scale);
        self.order
            .iter()
            .enumerate()
            .filter_map(|(position, &index)| {
                let (start, duration) = self.runes[index].cooldown?;
                let progress = ((now - start) / duration).clamp(0.0, 1.0) as f32;
                let x = position as f32 * (cell + spacing) + (cell - 27.0) / 2.0;
                let y = (ROW.cell.1 - 27.0) / 2.0;
                Some(TextureView {
                    name: format!("{}Cooldown", super::pip_name(position)),
                    art: rune(set.level_bar),
                    rect: (x * scale, y * scale, 27.0 * scale, 27.0 * scale),
                    alpha: 1.0,
                    rotation: 0.0,
                    shown: now >= start,
                    swipe: Some(progress),
                })
            })
            .collect()
    }
}
