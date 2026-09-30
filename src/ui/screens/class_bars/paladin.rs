//! `PaladinPowerBarFrame`: a rune holder with five authored runes (PaladinPowerBar.xml,
//! PaladinPowerBar.lua), atlas 2141 `uipaladinholypower.blp` FDID 4719247.

use super::super::inworld_unit_frames_art::{AtlasArt, FlipBook, art, key};
use super::anim::{
    AnimGroup, Layer, Looping, PointTemplate, PointVisual, Smoothing, group, layer, translate,
};
use super::{BarLogic, ClassBarView, pip_name, point_textures};
use crate::status::{ClassBar, ClassBarResource};

const fn holy(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_719_247, (512.0, 512.0), rect)
}

/// `HOLY_POWER_SPELL_READY` (PaladinPowerBar.lua:1).
const SPELL_READY: u8 = 3;

/// `PaladinPowerBar.VisualState` (PaladinPowerBar.lua:5-9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum VisualState {
    Inactive,
    Active,
    SpellReady,
}

// Holder layers and groups (PaladinPowerBar.xml:98-162).
const HOLDER_ACTIVE: usize = 1;
const HOLDER_GLOW: usize = 2;
const HOLDER_THIN_GLOW: usize = 3;
const HOLDER_ACTIVATE: usize = 0;
const HOLDER_READY: usize = 1;
const HOLDER_READY_LOOP: usize = 2;
const HOLDER_DEPLETE: usize = 3;

static HOLDER: PointTemplate = PointTemplate {
    layers: &[
        // BACKGROUND -5: uf-holypower-runeholder (17384); -3: -runeholder-active
        // (17380); -2: -runeholder-glow (17381), -runeholder-thinglow (17383).
        layer("Background", holy((1.0, 151.0, 440.0, 483.0)), (0.0, 0.0)),
        layer(
            "ActiveTexture",
            holy((165.0, 315.0, 430.0, 473.0)),
            (0.0, 0.0),
        ),
        layer("Glow", holy((323.0, 473.0, 213.0, 256.0)), (0.0, 0.0)),
        layer("ThinGlow", holy((323.0, 473.0, 303.0, 346.0)), (0.0, 0.0)),
    ],
    // `showAnim` is never played by PaladinPowerBar.lua.
    groups: &[
        // activateAnim
        group(
            &[
                (HOLDER_GLOW, key(0.0, 1.0, 0.0, 0.1)),
                (HOLDER_GLOW, key(1.0, 1.0, 0.1, 0.2)),
                (HOLDER_GLOW, key(1.0, 0.0, 0.3, 0.37)),
                (HOLDER_ACTIVE, key(0.0, 0.0, 0.0, 0.1)),
                (HOLDER_ACTIVE, key(0.0, 1.0, 0.1, 0.23)),
            ],
            &[],
        ),
        // readyAnim
        group(&[(HOLDER_THIN_GLOW, key(1.0, 0.7, 0.0, 0.67))], &[]),
        // readyLoopAnim, looping REPEAT
        group(
            &[
                (HOLDER_THIN_GLOW, key(0.7, 1.0, 0.0, 0.5)),
                (HOLDER_THIN_GLOW, key(1.0, 0.7, 0.5, 0.83)),
            ],
            &[],
        )
        .looping(Looping::Repeat),
        // depleteAnim
        group(
            &[
                (HOLDER_GLOW, key(1.0, 1.0, 0.0, 0.37)),
                (HOLDER_GLOW, key(1.0, 0.0, 0.37, 0.33)),
            ],
            &[],
        ),
    ],
};

// Rune layers and groups (PaladinPowerBarRuneTemplate, PaladinPowerBar.xml:6-94).
const FX: usize = 0;
const DEPLETE_FLIPBOOK: usize = 1;
const ACTIVE: usize = 2;
const GLOW: usize = 3;
const BLUR: usize = 4;
const ACTIVATE: usize = 0;
const READY: usize = 1;
const READY_LOOP: usize = 2;
const DEPLETE: usize = 3;

/// The groups of rune template layers `FX`, `DepleteFlipbook`, ... (its `depleteAnim`
/// flipbook is 6×5 of 26 frames over the rune's own crop).
const RUNE_GROUPS: [AnimGroup; 4] = [
    // activateAnim
    group(
        &[
            (FX, key(0.0, 1.0, 0.0, 0.3)),
            (FX, key(1.0, 0.0, 0.3, 0.5)),
            (BLUR, key(1.0, 1.0, 0.0, 0.27)),
            (BLUR, key(1.0, 0.0, 0.27, 0.53)),
            (GLOW, key(0.0, 1.0, 0.1, 0.23)),
            (GLOW, key(1.0, 0.0, 0.23, 0.57)),
            (ACTIVE, key(0.0, 0.0, 0.0, 0.1)),
            (ACTIVE, key(0.0, 1.0, 0.1, 0.23)),
        ],
        &[],
    )
    .moving(&[(BLUR, translate(0.0, 2.0, 0.0, 0.8).smooth(Smoothing::Out))]),
    // readyAnim
    group(
        &[
            (FX, key(0.0, 1.0, 0.0, 0.3)),
            (FX, key(1.0, 0.0, 0.3, 0.37)),
            (BLUR, key(1.0, 1.0, 0.0, 0.33)),
            (BLUR, key(1.0, 0.0, 0.3, 0.37)),
            (GLOW, key(0.0, 0.0, 0.0, 0.13)),
            (GLOW, key(0.0, 1.0, 0.13, 0.23)),
            (GLOW, key(1.0, 0.0, 0.23, 0.3)),
        ],
        &[],
    )
    .moving(&[(BLUR, translate(0.0, 3.0, 0.0, 0.67))]),
    // readyLoopAnim, looping REPEAT
    group(
        &[
            (GLOW, key(0.0, 1.0, 0.0, 0.5)),
            (GLOW, key(1.0, 0.0, 0.5, 0.83)),
        ],
        &[],
    )
    .looping(Looping::Repeat),
    // depleteAnim: no setToFinalAlpha
    group(
        &[],
        &[(
            DEPLETE_FLIPBOOK,
            FlipBook {
                columns: 6,
                rows: 5,
                frames: 26,
                duration: 0.87,
            },
        )],
    )
    .keep_alpha(false),
];

/// One rune's textures (`PaladinPowerBarRune:OnLoad` atlases,
/// `uf-holypower-rune<n>-fx/-active/-glow/-blur`, `UF-HolyPower-DepleteRune<n>`): FX at
/// `fxOffsetY` 5, the deplete flipbook at its per-rune size and y offset. `useBackground`
/// is false for every rune, so `uf-holypower-rune<n>` stays hidden and is not drawn.
const fn rune_layers(
    fx: (f32, f32, f32, f32),
    deplete: ((f32, f32, f32, f32), (f32, f32), f32),
    active: (f32, f32, f32, f32),
    glow: (f32, f32, f32, f32),
    blur: (f32, f32, f32, f32),
) -> [Layer; 5] {
    let (deplete_rect, deplete_size, deplete_y) = deplete;
    [
        layer("FX", holy(fx), (0.0, 5.0)),
        layer("DepleteFlipbook", holy(deplete_rect), (0.0, deplete_y)).sized(deplete_size),
        layer("ActiveTexture", holy(active), (0.0, 0.0)),
        layer("Glow", holy(glow), (0.0, 0.0)),
        layer("Blur", holy(blur), (0.0, 0.0)),
    ]
}

static RUNE_LAYERS: [[Layer; 5]; 5] = [
    // rune1: -fx 17348, DepleteRune1 17339 26×43 y 7, -active 17345, -glow 17349, -blur 17346
    rune_layers(
        (38.0, 68.0, 485.0, 511.0),
        ((165.0, 321.0, 213.0, 428.0), (26.0, 43.0), 7.0),
        (475.0, 498.0, 258.0, 281.0),
        (100.0, 126.0, 485.0, 511.0),
        (349.0, 364.0, 372.0, 392.0),
    ),
    // rune2: -fx 17355, DepleteRune2 17340 28×42 y 7, -active 17352, -glow 17356, -blur 17353
    rune_layers(
        (1.0, 36.0, 485.0, 509.0),
        ((165.0, 333.0, 1.0, 211.0), (28.0, 42.0), 7.0),
        (323.0, 347.0, 348.0, 370.0),
        (165.0, 192.0, 475.0, 500.0),
        (323.0, 339.0, 491.0, 510.0),
    ),
    // rune3: -fx 17362, DepleteRune3 17341 27×43 y 9, -active 17359, -glow 17363, -blur 17360
    rune_layers(
        (194.0, 222.0, 475.0, 499.0),
        ((1.0, 163.0, 223.0, 438.0), (27.0, 43.0), 9.0),
        (323.0, 346.0, 421.0, 442.0),
        (245.0, 271.0, 475.0, 500.0),
        (349.0, 364.0, 416.0, 435.0),
    ),
    // rune4: -fx 17369, DepleteRune4 17342 27×42 y 7, -active 17366, -glow 17370, -blur 17367
    rune_layers(
        (273.0, 297.0, 475.0, 500.0),
        ((335.0, 497.0, 1.0, 211.0), (27.0, 42.0), 7.0),
        (323.0, 344.0, 467.0, 489.0),
        (475.0, 499.0, 213.0, 238.0),
        (495.0, 508.0, 328.0, 346.0),
    ),
    // rune5: -fx 17376, DepleteRune5 17343 27×44 y 7, -active 17373, -glow 17377, -blur 17374
    rune_layers(
        (70.0, 98.0, 485.0, 510.0),
        ((1.0, 163.0, 1.0, 221.0), (27.0, 44.0), 7.0),
        (299.0, 321.0, 475.0, 498.0),
        (128.0, 154.0, 485.0, 511.0),
        (349.0, 364.0, 394.0, 414.0),
    ),
];

static RUNES: [PointTemplate; 5] = [
    PointTemplate {
        layers: &RUNE_LAYERS[0],
        groups: &RUNE_GROUPS,
    },
    PointTemplate {
        layers: &RUNE_LAYERS[1],
        groups: &RUNE_GROUPS,
    },
    PointTemplate {
        layers: &RUNE_LAYERS[2],
        groups: &RUNE_GROUPS,
    },
    PointTemplate {
        layers: &RUNE_LAYERS[3],
        groups: &RUNE_GROUPS,
    },
    PointTemplate {
        layers: &RUNE_LAYERS[4],
        groups: &RUNE_GROUPS,
    },
];

/// Each rune frame's LEFT anchor `(x, y)` on the holder and its size
/// (PaladinPowerBar.xml:165-224).
const RUNE_FRAMES: [((f32, f32), (f32, f32)); 5] = [
    ((18.0, -1.0), (18.0, 18.0)),
    ((41.6, 0.0), (20.0, 16.0)),
    ((68.0, 0.5), (17.0, 15.0)),
    ((92.0, 0.0), (15.0, 16.0)),
    ((112.2, -1.0), (18.0, 16.0)),
];

/// 150×43, `topPadding` -3, `leftPadding` 5 (PaladinPowerBar.xml:228-236).
const HOLDER_SIZE: (f32, f32) = (150.0, 43.0);
const PADDING: (f32, f32) = (-3.0, 5.0);

#[derive(Debug)]
struct Rune {
    visual: PointVisual,
    state: VisualState,
}

impl Rune {
    /// `PaladinPowerBarRune:OnLoad`: `SetVisualState(Inactive, skipTransitionAnimation)`.
    fn new(template: &'static PointTemplate, now: f64) -> Self {
        let mut rune = Self {
            visual: PointVisual::new(template),
            state: VisualState::Inactive,
        };
        rune.reset(now);
        rune.visual.set_alpha(DEPLETE_FLIPBOOK, 0.0);
        rune
    }

    /// `PaladinPowerBarRune:ResetAllVisuals` (PaladinPowerBar.lua:197-208).
    fn reset(&mut self, now: f64) {
        self.visual.stop_all(now);
        for layer in [ACTIVE, GLOW, FX, BLUR, DEPLETE_FLIPBOOK] {
            self.visual.set_alpha(layer, 0.0);
        }
    }

    /// `PaladinPowerBarRune:SetVisualState` (PaladinPowerBar.lua:157-195), never skipping
    /// the transition after `OnLoad`.
    fn set_state(&mut self, state: VisualState, now: f64) {
        let old = self.state;
        self.state = state;
        if state != old {
            self.reset(now);
        }
        let visual = &mut self.visual;
        match state {
            VisualState::Inactive => {
                if old != VisualState::Inactive && !visual.is_playing(DEPLETE) {
                    visual.set_alpha(DEPLETE_FLIPBOOK, 1.0);
                    visual.restart(DEPLETE, now);
                }
            }
            VisualState::Active if old == VisualState::Inactive => visual.restart(ACTIVATE, now),
            VisualState::Active => visual.set_alpha(ACTIVE, 1.0),
            VisualState::SpellReady => {
                visual.set_alpha(ACTIVE, 1.0);
                visual.stop(READY_LOOP, now);
                visual.restart(READY, now);
            }
        }
    }
}

/// The holder's `PaladinPowerBar` state (PaladinPowerBar.lua:52-117).
#[derive(Debug)]
struct Holder {
    visual: PointVisual,
    state: Option<VisualState>,
    last_power: Option<u8>,
    /// The group whose `OnFinished` is `PlayReadyLoopCallback`.
    then_loop: Option<usize>,
}

impl Holder {
    fn new(now: f64) -> Self {
        let mut holder = Self {
            visual: PointVisual::new(&HOLDER),
            state: None,
            last_power: None,
            then_loop: None,
        };
        holder.reset(now);
        holder
    }

    /// `ResetAllVisuals`.
    fn reset(&mut self, now: f64) {
        self.visual.stop_all(now);
        for layer in [HOLDER_ACTIVE, HOLDER_THIN_GLOW, HOLDER_GLOW] {
            self.visual.set_alpha(layer, 0.0);
        }
    }

    /// Chains `readyLoopAnim` from the instant a callback group ended, in SpellReady.
    fn finish(&mut self, now: f64) {
        for (group, ended) in self.visual.finish(now) {
            if self.then_loop == Some(group) {
                self.then_loop = None;
                if self.state == Some(VisualState::SpellReady) {
                    self.visual
                        .play(HOLDER_READY_LOOP, now, (now - ended) as f32, 1.0);
                }
            }
        }
    }

    /// `PaladinPowerBar:UpdateVisualState`.
    fn update(&mut self, state: VisualState, power: u8, now: f64) {
        if self.state == Some(state) && self.last_power == Some(power) {
            return;
        }
        let old = self.state;
        let old_power = self.last_power.unwrap_or(0);
        self.state = Some(state);
        self.last_power = Some(power);
        if old != Some(state) {
            self.reset(now);
        }
        let visual = &mut self.visual;
        match state {
            VisualState::Inactive => {
                if old != Some(state) {
                    visual.restart(HOLDER_DEPLETE, now);
                }
            }
            VisualState::Active => {
                if old == Some(VisualState::SpellReady) || power < old_power {
                    visual.set_alpha(HOLDER_ACTIVE, 1.0);
                    visual.restart(HOLDER_DEPLETE, now);
                } else {
                    visual.restart(HOLDER_ACTIVATE, now);
                }
            }
            VisualState::SpellReady => {
                visual.set_alpha(HOLDER_ACTIVE, 1.0);
                visual.stop(HOLDER_READY, now);
                visual.stop(HOLDER_READY_LOOP, now);
                let group = if power < old_power {
                    HOLDER_DEPLETE
                } else {
                    HOLDER_READY
                };
                self.then_loop = Some(group);
                visual.restart(group, now);
            }
        }
    }
}

#[derive(Debug)]
pub struct Bar {
    holder: Holder,
    runes: Vec<Rune>,
}

impl Default for Bar {
    /// `OnLoad` of the holder and each rune.
    fn default() -> Self {
        Self {
            holder: Holder::new(0.0),
            runes: RUNES.iter().map(|rune| Rune::new(rune, 0.0)).collect(),
        }
    }
}

impl BarLogic for Bar {
    /// `PaladinPowerBar:UpdatePower` (PaladinPowerBar.lua:20-46): runes up to the max,
    /// each Inactive, Active or (from three holy power) SpellReady, and the holder.
    fn power(&mut self, resource: &ClassBarResource, now: f64) {
        let power = resource.current;
        let lit = if power >= SPELL_READY {
            VisualState::SpellReady
        } else {
            VisualState::Active
        };
        let max = usize::from(resource.max);
        for (index, rune) in (1u8..).zip(self.runes.iter_mut()).take(max) {
            let state = if index <= power {
                lit
            } else {
                VisualState::Inactive
            };
            rune.set_state(state, now);
        }
        let holder_state = if power > 0 {
            lit
        } else {
            VisualState::Inactive
        };
        self.holder.update(holder_state, power, now);
    }

    fn tick(&mut self, now: f64) {
        self.holder.finish(now);
        for rune in &mut self.runes {
            rune.visual.finish(now);
        }
    }

    fn view(&self, _: &ClassBarResource, now: f64) -> ClassBarView {
        let mut textures = point_textures(
            "PlayerSecondaryResourceHolder",
            &self.holder.visual,
            ((0.0, 0.0), 1.0),
            HOLDER_SIZE,
            now,
        );
        for (index, (rune, ((x, y), size))) in self.runes.iter().zip(RUNE_FRAMES).enumerate() {
            let origin = (x, HOLDER_SIZE.1 / 2.0 - y - size.1 / 2.0);
            let name = pip_name(index);
            textures.extend(point_textures(
                &name,
                &rune.visual,
                (origin, 1.0),
                size,
                now,
            ));
        }
        ClassBarView {
            bar: ClassBar::HolyPower,
            size: HOLDER_SIZE,
            padding: PADDING,
            textures,
        }
    }
}
