//! `EssencePointButtonTemplate` / `EssencePlayerFrame` (EssenceFramePlayer.xml:5-304,
//! EssenceFramePlayer.lua), atlas 2034 `uievokeressence.blp` FDID 4631562. Each point is
//! five overlaid child frames (Filling, FillDone, Empty, Full, Depleting) shown and hidden
//! by the mixin; their textures are drawn in that frame order.

use std::ops::Range;

use super::super::inworld_unit_frames_art::{AtlasArt, art, key};
use super::anim::{PointTemplate, PointVisual, Smoothing, group, layer, rotate, translate};
use super::{BarLogic, ClassBarView, Row};
use crate::status::{ClassBar, ClassBarResource};

const fn essence(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_631_562, (512.0, 256.0), rect)
}

const BG: (f32, f32, f32, f32) = (324.0, 348.0, 107.0, 131.0);
const BG_ACTIVE: (f32, f32, f32, f32) = (294.0, 318.0, 224.0, 248.0);
const ICON: (f32, f32, f32, f32) = (350.0, 374.0, 133.0, 157.0);
const ICON_PROG: (f32, f32, f32, f32) = (350.0, 374.0, 107.0, 131.0);

// EssenceFilling
const SPIN_OUT_BG: usize = 1;
const TRAIL_SPINNER_IN: usize = 2;
const ICON_PROG_B: usize = 3;
const ICON_PROG_C: usize = 4;
const ICON_PROG_A: usize = 5;
const TRAIL_SPINNER: usize = 6;
const SPINNER_OUT: usize = 7;
const SPIN_STAR: usize = 8;
const TIMER_SPINNER: usize = 9;
// EssenceFillDone
const FX_BURST: usize = 10;
const DONE_BG_ACTIVE: usize = 12;
const RIM_GLOW: usize = 14;
const DONE_ICON: usize = 15;
const ICON_GLOW: usize = 16;
// EssenceDepleting
const FX_DEP_BG: usize = 20;
const DEP_BG_ACTIVE: usize = 21;
const DEP_ICON: usize = 22;
const FX_RIM_GLOW: usize = 23;
const ICON_DEPLETE: usize = 24;
const FX_SMOKE: usize = 25;

const FILLING: Range<usize> = 0..10;
const FILL_DONE: Range<usize> = 10..17;
const EMPTY: Range<usize> = 17..18;
const FULL: Range<usize> = 18..19;
const DEPLETING: Range<usize> = 19..26;

const FILLING_ANIM: usize = 0;
const CIRCLE_ANIM: usize = 1;
const FILL_DONE_ANIM: usize = 2;
const DEPLETING_ANIM: usize = 3;

static TEMPLATE: PointTemplate = PointTemplate {
    layers: &[
        // EssenceFilling: BACKGROUND UF-Essence-BG (16259); BORDER -Spin-OutBG (16277);
        // ARTWORK -TimerSpin-Trail (16284), -Icon-ProgB (16273), -Icon-ProgC (16274),
        // -Icon-Prog (16272), -Spinner (16281); ARTWORK 2 -SpinnerOut (16283),
        // -Spin-Stars (16280); OVERLAY -TimerSpin (16285). All but the BG start at alpha 0.
        layer("FillingEssenceBG", essence(BG), (0.0, 0.0)).hidden(),
        layer(
            "FillingSpinOut_BG",
            essence((249.0, 277.0, 216.0, 244.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        layer(
            "FillingTrailSpinnerIn",
            essence((294.0, 322.0, 137.0, 165.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        layer(
            "FillingIconProg_B",
            essence((480.0, 508.0, 60.0, 88.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        layer(
            "FillingIconProg_C",
            essence((249.0, 277.0, 156.0, 184.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        layer("FillingIconProg", essence(ICON_PROG), (0.0, 0.0))
            .alpha(0.0)
            .hidden(),
        layer(
            "FillingTrailSpinner",
            essence((350.0, 374.0, 159.0, 183.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        layer(
            "FillingSpinnerOut",
            essence((294.0, 322.0, 107.0, 135.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        layer(
            "FillingSpinStar",
            essence((416.0, 448.0, 60.0, 92.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        layer(
            "FillingTimerSpinner",
            essence((350.0, 374.0, 185.0, 209.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        // EssenceFillDone: BACKGROUND 0 -FX-Burst (16263), 1 -BG, 2 -BG-Active (16258)
        // alpha 0; ARTWORK 0 -Icon-Prog, -FX-RimGlw (16266) alpha 0; ARTWORK 1 -Icon
        // (16276) alpha 0; ARTWORK 2 -Icon-Glw (16271) alpha 0.
        layer(
            "FillDoneFXBurst",
            essence((249.0, 292.0, 107.0, 154.0)),
            (0.0, 0.0),
        )
        .hidden(),
        layer("FillDoneCircBG", essence(BG), (0.0, 0.0)).hidden(),
        layer("FillDoneCircBGActive", essence(BG_ACTIVE), (0.0, 0.0))
            .alpha(0.0)
            .hidden(),
        layer("FillDoneEssenceIconProg", essence(ICON_PROG), (0.0, 0.0)).hidden(),
        layer(
            "FillDoneRimGlow",
            essence((324.0, 348.0, 133.0, 157.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        layer("FillDoneEssenceIcon", essence(ICON), (0.0, 0.0))
            .alpha(0.0)
            .hidden(),
        layer(
            "FillDoneEssenceIconGlow",
            essence((324.0, 348.0, 211.0, 235.0)),
            (0.0, 0.0),
        )
        .alpha(0.0)
        .hidden(),
        // EssenceEmpty: -BG.
        layer("EmptyEssenceBG", essence(BG), (0.0, 0.0)).hidden(),
        // EssenceFull: -Icon-Active (16269).
        layer(
            "FullEssenceIconActive",
            essence((324.0, 348.0, 185.0, 209.0)),
            (0.0, 0.0),
        )
        .hidden(),
        // EssenceDepleting: BACKGROUND -BG; BORDER -FX-DepletedBG (16264), -BG-Active;
        // BORDER 1 -Icon; ARTWORK -FX-RimGlwDep (16267), -Icon-Dep (16270), -FX-DepSmoke
        // (16265) at y 10, alpha 0.
        layer("DepletingEssenceBG", essence(BG), (0.0, 0.0)).hidden(),
        layer(
            "DepletingFXDepBG",
            essence((294.0, 320.0, 196.0, 222.0)),
            (0.0, 0.0),
        )
        .hidden(),
        layer("DepletingCircBGActive", essence(BG_ACTIVE), (0.0, 0.0)).hidden(),
        layer("DepletingEssenceIcon", essence(ICON), (0.0, 0.0)).hidden(),
        layer(
            "DepletingFXRimGlow",
            essence((324.0, 348.0, 159.0, 183.0)),
            (0.0, 0.0),
        )
        .hidden(),
        layer(
            "DepletingIconDeplete",
            essence((450.0, 478.0, 60.0, 88.0)),
            (0.0, 0.0),
        )
        .hidden(),
        layer(
            "DepletingFXSmoke",
            essence((350.0, 364.0, 211.0, 237.0)),
            (0.0, 10.0),
        )
        .alpha(0.0)
        .hidden(),
    ],
    groups: &[
        // EssenceFilling.FillingAnim, no setToFinalAlpha (EssenceFramePlayer.xml:74-99).
        group(
            &[
                (TIMER_SPINNER, key(1.0, 0.0, 2.8, 0.1)),
                (TIMER_SPINNER, key(0.0, 1.0, 0.0, 0.1)),
                (TRAIL_SPINNER, key(0.0, 1.0, 0.5, 2.0)),
                (TRAIL_SPINNER, key(1.0, 0.0, 4.8, 0.2)),
                (TRAIL_SPINNER_IN, key(0.0, 1.0, 0.5, 1.0)),
                (TRAIL_SPINNER_IN, key(1.0, 0.0, 4.8, 0.2)),
                (ICON_PROG_B, key(0.0, 0.8, 2.2, 0.5)),
                (ICON_PROG_C, key(0.0, 0.8, 3.0, 0.5)),
                (ICON_PROG_A, key(0.0, 1.0, 4.3, 0.6)),
            ],
            &[],
        )
        .moving(&[(TIMER_SPINNER, translate(0.0, 20.0, 4.9, 0.1))])
        .turning(&[
            (
                TIMER_SPINNER,
                rotate(-360.0, 0.0, 5.0).smooth(Smoothing::InOut),
            ),
            (
                TRAIL_SPINNER,
                rotate(-360.0, 0.0, 5.0).smooth(Smoothing::InOut),
            ),
            (
                TRAIL_SPINNER_IN,
                rotate(-360.0, 0.0, 5.0).smooth(Smoothing::InOut),
            ),
        ])
        .keep_alpha(false),
        // EssenceFilling.CircleAnim, no setToFinalAlpha (EssenceFramePlayer.xml:100-113).
        group(
            &[
                (SPIN_OUT_BG, key(0.0, 1.0, 4.0, 0.3)),
                (SPIN_OUT_BG, key(1.0, 0.0, 4.7, 0.2)),
                (SPINNER_OUT, key(0.0, 1.0, 4.3, 0.3)),
                (SPINNER_OUT, key(1.0, 0.0, 4.6, 0.2)),
                (SPIN_STAR, key(0.0, 1.0, 4.3, 0.3)),
                (SPIN_STAR, key(1.0, 0.0, 4.6, 0.2)),
            ],
            &[],
        )
        .turning(&[
            (
                SPINNER_OUT,
                rotate(-260.0, 4.3, 0.6).smooth(Smoothing::InOut),
            ),
            (SPIN_STAR, rotate(-260.0, 4.3, 0.7).smooth(Smoothing::InOut)),
        ])
        .keep_alpha(false),
        // EssenceFillDone.AnimIn (EssenceFramePlayer.xml:170-185).
        group(
            &[
                (ICON_GLOW, key(0.0, 1.0, 0.0, 0.25)),
                (ICON_GLOW, key(1.0, 0.0, 0.5, 0.5)),
                (DONE_ICON, key(0.0, 1.0, 0.0, 0.25)),
                (RIM_GLOW, key(0.0, 1.0, 0.0, 0.25)),
                (RIM_GLOW, key(1.0, 0.0, 0.5, 0.3)),
                (DONE_BG_ACTIVE, key(0.0, 1.0, 0.0, 0.5)),
                (FX_BURST, key(0.0, 0.8, 0.0, 0.2)),
                (FX_BURST, key(0.8, 0.0, 0.2, 0.4)),
            ],
            &[],
        )
        .turning(&[
            (FX_BURST, rotate(-30.0, 0.0, 0.2)),
            (FX_BURST, rotate(-10.0, 0.2, 0.5)),
        ]),
        // EssenceDepleting.AnimIn (EssenceFramePlayer.xml:264-274).
        group(
            &[
                (FX_RIM_GLOW, key(1.0, 0.0, 0.3, 0.5)),
                (ICON_DEPLETE, key(1.0, 0.0, 0.3, 0.5)),
                (FX_DEP_BG, key(1.0, 0.0, 0.3, 0.5)),
                (DEP_BG_ACTIVE, key(1.0, 0.0, 0.0, 0.8)),
                (DEP_ICON, key(1.0, 0.0, 0.0, 0.5)),
                (FX_SMOKE, key(0.0, 1.0, 0.0, 0.3)),
                (FX_SMOKE, key(1.0, 0.0, 0.4, 0.5)),
            ],
            &[],
        )
        .moving(&[
            (FX_SMOKE, translate(0.0, 3.0, 0.3, 0.7)),
            (FX_SMOKE, translate(0.0, 2.0, 0.0, 0.3)),
        ]),
    ],
};

/// `FillingAnimationTime` (EssenceFramePlayer.lua:1).
const FILLING_TIME: f32 = 5.0;

#[derive(Debug)]
struct Point {
    visual: PointVisual,
    shown: [bool; 5],
    fill_speed: f32,
}

const FRAMES: [Range<usize>; 5] = [FILLING, FILL_DONE, EMPTY, FULL, DEPLETING];

impl Point {
    fn new() -> Self {
        Self {
            visual: PointVisual::new(&TEMPLATE),
            shown: [false; 5],
            fill_speed: 0.0,
        }
    }

    /// `Frame:Show()` / `Hide()` of child frame `frame`; showing EssenceFillDone plays its
    /// `AnimIn` (`OnShow`).
    fn set_frame(&mut self, frame: usize, shown: bool, now: f64) {
        if self.shown[frame] == shown {
            return;
        }
        self.shown[frame] = shown;
        for layer in FRAMES[frame].clone() {
            self.visual.set_shown(layer, shown);
        }
        if frame == 1 && shown {
            self.visual.restart(FILL_DONE_ANIM, now);
        }
    }

    fn is_filling(&self) -> bool {
        self.visual.is_playing(FILLING_ANIM)
    }

    /// `FillingAnim`'s `OnFinished`: show EssenceFillDone from the instant it ended.
    fn finish(&mut self, now: f64) {
        for (group, ended) in self.visual.finish(now) {
            if group == FILLING_ANIM {
                self.set_frame(1, true, ended);
            }
        }
    }

    /// `EssencePointButtonMixin:AnimIn(speed, elapsedPortion)`.
    fn anim_in(&mut self, speed: f32, portion: f32, now: f64) {
        self.fill_speed = speed;
        self.set_frame(0, true, now);
        self.visual
            .play(FILLING_ANIM, now, portion * FILLING_TIME, speed);
        self.visual
            .play(CIRCLE_ANIM, now, portion * FILLING_TIME, speed);
        for frame in [4, 2, 1, 3] {
            self.set_frame(frame, false, now);
        }
    }

    /// `EssencePointButtonMixin:AnimOut`.
    fn anim_out(&mut self, now: f64) {
        if !(self.shown[3] || self.shown[0] || self.shown[1]) {
            return;
        }
        self.set_frame(4, true, now);
        if self.is_filling() {
            self.visual.stop(FILLING_ANIM, now);
            self.visual.stop(CIRCLE_ANIM, now);
            self.visual.play(
                DEPLETING_ANIM,
                now,
                TEMPLATE.groups[DEPLETING_ANIM].duration(),
                1.0,
            );
        } else {
            self.visual.restart(DEPLETING_ANIM, now);
        }
        for frame in [0, 2, 1, 3] {
            self.set_frame(frame, false, now);
        }
    }

    /// `EssencePointButtonMixin:SetEssennceFull`.
    fn set_full(&mut self, now: f64) {
        self.visual.stop(FILLING_ANIM, now);
        self.visual.stop(CIRCLE_ANIM, now);
        self.set_frame(1, true, now);
        self.set_frame(2, false, now);
    }
}

/// 24×24 points 1 px overlapped, `topPadding` 5 (EssenceFramePlayer.xml:6,289,298).
const ROW: Row = Row {
    cell: (24.0, 24.0),
    spacing: -1.0,
    padding: (5.0, 0.0),
    scale: 1.0,
};

#[derive(Debug, Default)]
pub struct Bar {
    points: Vec<Point>,
}

impl BarLogic for Bar {
    /// `EssencePowerBar:UpdatePower` (EssenceFramePlayer.lua:4-46).
    fn power(&mut self, resource: &ClassBarResource, now: f64) {
        if self.points.len() != usize::from(resource.max) {
            self.points = (0..resource.max).map(|_| Point::new()).collect();
        }
        let (current, max) = (usize::from(resource.current), usize::from(resource.max));
        let elapsed = (now - resource.dynamics.received_at).max(0.0) as f32;
        // `GetPowerRegenForPowerType`: no or zero regen reads as 0.2 per second
        // (EssenceFramePlayer.lua:33-36, 63-66).
        let rate = match resource.dynamics.regen_per_sec {
            rate if rate > 0.0 => rate,
            _ => 0.2,
        };
        let portion =
            (f32::from(resource.dynamics.partial) / 1000.0 + elapsed * rate).clamp(0.0, 1.0);
        for point in self.points.iter_mut().take(current.min(max)) {
            point.set_full(now);
        }
        for point in self.points.iter_mut().skip(current + 1) {
            point.anim_out(now);
        }
        if current != max
            && let Some(point) = self.points.get_mut(current)
        {
            let filling = point.is_filling() || point.shown[3];
            let outdated =
                filling && (portion - point.visual.progress(FILLING_ANIM, now)).abs() > 0.1;
            let speed = FILLING_TIME * rate;
            if !filling || outdated || point.fill_speed != speed {
                if !filling {
                    point.visual.stop(FILLING_ANIM, now);
                    point.visual.stop(CIRCLE_ANIM, now);
                }
                point.anim_in(speed, portion, now);
            }
        }
    }

    fn tick(&mut self, now: f64) {
        for point in &mut self.points {
            point.finish(now);
        }
    }

    fn view(&self, _: &ClassBarResource, now: f64) -> ClassBarView {
        let visuals = self.points.iter().map(|point| &point.visual);
        ROW.view(ClassBar::Essence, visuals, now)
    }
}
