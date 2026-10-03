//! `ShardTemplate` / `WarlockPowerFrame` (ShardBar.xml:5-173, ShardBar.lua), atlas 2138
//! `uiwarlocksoulshard.blp` FDID 4715163.

use super::super::inworld_unit_frames_art::{AtlasArt, FlipBook, art, key};
use super::anim::{AnimGroup, Layer, Looping, PointTemplate, PointVisual, group, layer};
use super::{BarLogic, ClassBarView, Row};
use crate::status::{ClassBar, ClassBarResource};

const fn shard(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_715_163, (512.0, 256.0), rect)
}

const FRAME_GLOW: usize = 1;
const ICON: usize = 2;
const DEPLETE_A: usize = 3;
const DEPLETE_B: usize = 4;
const DEPLETE_C: usize = 5;
const REFILL: usize = 6;
const REFILL_FX: usize = 7;
const ICON_FX2: usize = 8;
const ICON_GLOW: usize = 9;
const ICON_FX3: usize = 10;
const SOUL: usize = 11;
/// `FillIncrement.Fill` / `.Glow` for increment `n` (1..=9) are layers `FILL + 2 (n - 1)`
/// and the one after it.
const FILL: usize = 12;
const FX_TEXTURES: [usize; 6] = [FRAME_GLOW, REFILL, REFILL_FX, ICON_FX2, ICON_GLOW, ICON_FX3];
const FX_FLIPBOOKS: [usize; 4] = [DEPLETE_A, DEPLETE_B, DEPLETE_C, SOUL];

const EMPTY_TO_FULL: usize = 0;
const FILL_DONE: usize = 1;
const READY_LOOP: usize = 2;
const DEPLETE: [usize; 3] = [3, 4, 5];
/// `FillIncrement.FillAnim` on increment `n` is group `FILL_ANIM + n - 1`.
const FILL_ANIM: usize = 6;

/// `WarlockShardMixin.IncrementSettings` fill and glow of one increment
/// (ShardBar.lua:53-63): `UF-SoulShard-Inc<n>` at `fillYOffset`, `-Inc<n>Glow` at
/// `glowYOffset`, inside `FillIncrement`, drawn over the shard's own layers.
const fn increment(
    fill: (f32, f32, f32, f32),
    fill_y: f32,
    glow: (f32, f32, f32, f32),
    glow_y: f32,
) -> [Layer; 2] {
    [
        layer("FillIncrementFill", shard(fill), (0.0, fill_y)).hidden(),
        layer("FillIncrementGlow", shard(glow), (0.0, glow_y))
            .alpha(0.0)
            .hidden(),
    ]
}

const fn flipbook(columns: u32, rows: u32, frames: u32, duration: f32) -> FlipBook {
    FlipBook {
        columns,
        rows,
        frames,
        duration,
    }
}

const INCREMENTS: [[Layer; 2]; 9] = [
    // Inc1 (17273) / Inc1Glow (17274)
    increment(
        (167.0, 183.0, 220.0, 247.0),
        -5.5,
        (191.0, 210.0, 162.0, 189.0),
        -5.5,
    ),
    // Inc2 (17275) / Inc2Glow (17276)
    increment(
        (188.0, 204.0, 191.0, 218.0),
        -6.5,
        (212.0, 231.0, 162.0, 189.0),
        -6.5,
    ),
    // Inc3 (17277) / Inc3Glow (17278)
    increment(
        (188.0, 204.0, 220.0, 247.0),
        -6.5,
        (167.0, 186.0, 191.0, 218.0),
        -6.5,
    ),
    // Inc4 (17279) / Inc4Glow (17280)
    increment(
        (142.0, 164.0, 193.0, 220.0),
        -5.5,
        (115.0, 140.0, 162.0, 189.0),
        -5.5,
    ),
    // Inc5 (17281) / Inc5Glow (17282)
    increment(
        (142.0, 164.0, 222.0, 249.0),
        -5.5,
        (115.0, 140.0, 191.0, 218.0),
        -5.5,
    ),
    // Inc6 (17283) / Inc6Glow (17284)
    increment(
        (167.0, 189.0, 162.0, 189.0),
        -5.5,
        (115.0, 140.0, 220.0, 247.0),
        -5.5,
    ),
    // Inc7 (17285) / Inc7Glow (17286)
    increment(
        (90.0, 113.0, 194.0, 223.0),
        -5.0,
        (32.0, 59.0, 162.0, 194.0),
        -3.0,
    ),
    // Inc8 (17287) / Inc8Glow (17288)
    increment(
        (90.0, 113.0, 225.0, 254.0),
        -4.5,
        (32.0, 59.0, 196.0, 228.0),
        -3.0,
    ),
    // Inc9 (17289) / Inc9Glow (17290)
    increment(
        (142.0, 165.0, 162.0, 191.0),
        -4.5,
        (61.0, 88.0, 162.0, 194.0),
        -3.0,
    ),
];

const fn layers() -> [Layer; 30] {
    let own = [
        // BACKGROUND: UF-SoulShard-Holder (17230) y -4.5, UF-SoulShard-FX-FrameGlow
        // (17224) y -3.5.
        layer(
            "Background",
            shard((90.0, 113.0, 162.0, 192.0)),
            (0.0, -4.5),
        ),
        layer("Frame_Glow", shard((1.0, 30.0, 162.0, 198.0)), (0.0, -3.5)),
        // ARTWORK: UF-SoulShard-Icon (17231) y -3 alpha 0; DepleteA (17250) 21×39 at
        // y 6, DepleteB (17251) 21×41 at y 8, DepleteC (17252) 29×35 at (-1, 4).
        layer(
            "Shard_Icon",
            shard((206.0, 221.0, 191.0, 211.0)),
            (0.0, -3.0),
        )
        .alpha(0.0),
        layer(
            "FB_DepleteA",
            shard((369.0, 495.0, 108.0, 225.0)),
            (0.0, 6.0),
        )
        .sized((21.0, 39.0)),
        layer(
            "FB_DepleteB",
            shard((241.0, 367.0, 108.0, 231.0)),
            (0.0, 8.0),
        )
        .sized((21.0, 41.0)),
        layer(
            "FB_DepleteC",
            shard((241.0, 415.0, 1.0, 106.0)),
            (-1.0, 4.0),
        )
        .sized((29.0, 35.0)),
        // OVERLAY: Refill (17233), RefillFX (17291), FX2 (17228) alpha 0, IconGlow
        // (17232) alpha 0, all y -3; FX3 (17229) y -1; Soul flipbook (17253) 34×53 at
        // y 15.
        layer(
            "Shard_Refill",
            shard((206.0, 221.0, 213.0, 233.0)),
            (0.0, -3.0),
        ),
        layer(
            "Shard_RefillFX",
            shard((223.0, 238.0, 213.0, 233.0)),
            (0.0, -3.0),
        ),
        layer(
            "Shard_IconFX2",
            shard((1.0, 17.0, 235.0, 255.0)),
            (0.0, -3.0),
        )
        .alpha(0.0),
        layer(
            "Shard_IconGlow",
            shard((223.0, 238.0, 191.0, 211.0)),
            (0.0, -3.0),
        )
        .alpha(0.0),
        layer(
            "Shard_IconFX3",
            shard((32.0, 48.0, 230.0, 253.0)),
            (0.0, -1.0),
        ),
        layer("Shard_Soul", shard((1.0, 239.0, 1.0, 160.0)), (0.0, 15.0)).sized((34.0, 53.0)),
    ];
    let mut all = [own[0]; 30];
    let mut index = 0;
    while index < own.len() {
        all[index] = own[index];
        index += 1;
    }
    let mut n = 0;
    while n < INCREMENTS.len() {
        all[FILL + 2 * n] = INCREMENTS[n][0];
        all[FILL + 2 * n + 1] = INCREMENTS[n][1];
        n += 1;
    }
    all
}

static LAYERS: [Layer; 30] = layers();

/// `FillIncrement.FillAnim` (ShardBar.xml:98-101) on increment `n`'s glow.
macro_rules! fill_anim {
    ($n:expr) => {
        group(
            &[
                (FILL + 2 * ($n - 1) + 1, key(0.0, 1.0, 0.0, 0.2)),
                (FILL + 2 * ($n - 1) + 1, key(1.0, 0.0, 0.2, 0.4)),
            ],
            &[],
        )
    };
}

/// `depleteAnimA/B/C` (ShardBar.xml:135-146).
macro_rules! deplete {
    ($layer:expr) => {
        group(
            &[(FRAME_GLOW, key(1.0, 0.0, 0.0, 0.5))],
            &[($layer, flipbook(6, 3, 15, 0.5))],
        )
    };
}

static GROUPS: [AnimGroup; 15] = [
    // emptyToFullAnim (ShardBar.xml:106-118).
    group(
        &[
            (ICON_FX3, key(0.0, 1.0, 0.17, 0.17)),
            (ICON_FX3, key(0.0, 0.0, 0.34, 0.33)),
            (ICON_GLOW, key(0.0, 1.0, 0.17, 0.17)),
            (ICON_GLOW, key(1.0, 0.0, 0.34, 0.33)),
            (FRAME_GLOW, key(0.0, 1.0, 0.17, 0.2)),
            (FRAME_GLOW, key(1.0, 0.0, 0.37, 0.3)),
            (ICON_FX2, key(0.0, 1.0, 0.0, 0.17)),
            (ICON_FX2, key(1.0, 0.0, 0.17, 0.35)),
            (ICON, key(0.0, 0.0, 0.0, 0.47)),
            (ICON, key(0.0, 1.0, 0.47, 0.2)),
        ],
        &[(SOUL, flipbook(7, 3, 18, 0.6))],
    ),
    // fillDoneAnim (ShardBar.xml:119-130).
    group(
        &[
            (ICON_FX3, key(1.0, 1.0, 0.0, 0.43)),
            (ICON_FX3, key(1.0, 0.0, 0.43, 0.17)),
            (REFILL_FX, key(0.0, 1.0, 0.0, 0.43)),
            (REFILL_FX, key(1.0, 0.0, 0.43, 0.17)),
            (REFILL, key(1.0, 1.0, 0.0, 0.43)),
            (REFILL, key(1.0, 0.0, 0.43, 0.17)),
            (ICON, key(0.0, 1.0, 0.0, 0.43)),
            (FRAME_GLOW, key(1.0, 1.0, 0.0, 0.27)),
            (FRAME_GLOW, key(1.0, 0.0, 0.27, 0.33)),
        ],
        &[(SOUL, flipbook(7, 3, 18, 0.6))],
    ),
    // readyLoopAnim, looping BOUNCE (ShardBar.xml:131-134).
    group(
        &[
            (ICON_GLOW, key(0.0, 1.0, 0.0, 0.65)),
            (FRAME_GLOW, key(0.25, 0.6, 0.0, 0.65)),
        ],
        &[],
    )
    .looping(Looping::Bounce),
    deplete!(DEPLETE_A),
    deplete!(DEPLETE_B),
    deplete!(DEPLETE_C),
    fill_anim!(1),
    fill_anim!(2),
    fill_anim!(3),
    fill_anim!(4),
    fill_anim!(5),
    fill_anim!(6),
    fill_anim!(7),
    fill_anim!(8),
    fill_anim!(9),
];

static TEMPLATE: PointTemplate = PointTemplate {
    layers: &LAYERS,
    groups: &GROUPS,
};

/// `WarlockShardMixin:ResetVisuals` (ShardBar.lua:145-163).
fn reset(visual: &mut PointVisual, now: f64) {
    visual.stop_all(now);
    for layer in FILL..LAYERS.len() {
        visual.set_shown(layer, false);
    }
    visual.set_alpha(ICON, 0.0);
    for layer in FX_TEXTURES {
        visual.set_alpha(layer, 0.0);
    }
    for layer in FX_FLIPBOOKS {
        visual.set_shown(layer, false);
    }
}

/// One shard and its fill in tenths, `None` after `Setup`.
#[derive(Debug)]
struct Shard {
    visual: PointVisual,
    fill: Option<u16>,
}

impl Shard {
    fn new(now: f64) -> Self {
        let mut visual = PointVisual::new(&TEMPLATE);
        reset(&mut visual, now);
        Self { visual, fill: None }
    }

    /// `WarlockShardMixin:Update(powerAmount, isBarFull)` (ShardBar.lua:76-131), `fill`
    /// in tenths of this shard.
    fn update(&mut self, fill: u16, bar_full: bool, now: f64) {
        if self.fill == Some(fill) {
            self.update_full_power(bar_full, now);
            return;
        }
        let old = self.fill.unwrap_or(0);
        self.fill = Some(fill);
        let visual = &mut self.visual;
        reset(visual, now);
        if fill == 0 {
            let book = match old {
                7.. => Some(2),
                4.. => Some(1),
                1.. => Some(0),
                0 => None,
            };
            if let Some(book) = book {
                visual.set_shown(FX_FLIPBOOKS[book], true);
                visual.restart(DEPLETE[book], now);
            }
        } else if fill >= 10 {
            if !bar_full {
                visual.set_shown(SOUL, true);
                let group = if old == 0 { EMPTY_TO_FULL } else { FILL_DONE };
                visual.restart(group, now);
            }
        } else {
            let n = usize::from(fill);
            visual.set_shown(FILL + 2 * (n - 1), true);
            visual.set_shown(FILL + 2 * (n - 1) + 1, true);
            visual.restart(FILL_ANIM + n - 1, now);
        }
        if bar_full {
            self.update_full_power(bar_full, now);
        }
    }

    /// `UpdateFullPowerVisuals` (ShardBar.lua:133-143).
    fn update_full_power(&mut self, bar_full: bool, now: f64) {
        let visual = &mut self.visual;
        let looping = visual.is_playing(READY_LOOP);
        if bar_full && !looping {
            visual.stop(EMPTY_TO_FULL, now);
            visual.stop(FILL_DONE, now);
            visual.set_shown(SOUL, false);
            visual.set_alpha(ICON, 1.0);
            visual.restart(READY_LOOP, now);
        } else if !bar_full && looping {
            visual.stop(READY_LOOP, now);
        }
    }
}

/// `SPEC_WARLOCK_DESTRUCTION`.
const DESTRUCTION: u32 = 267;

/// 23×30 shards 1 apart, `leftPadding` 5, `topPadding` -2 (ShardBar.xml:6,159,162-163).
const ROW: Row = Row {
    cell: (23.0, 30.0),
    spacing: 1.0,
    padding: (-2.0, 5.0),
    scale: 1.0,
};

#[derive(Debug, Default)]
pub struct Bar {
    shards: Vec<Shard>,
}

impl BarLogic for Bar {
    /// `WarlockPowerBar:UpdatePower` (ShardBar.lua:19-41), also run on combat changes:
    /// partial shards only for Destruction; "full power" visuals only in combat with every
    /// shard full.
    fn power(&mut self, resource: &ClassBarResource, now: f64) {
        if self.shards.len() != usize::from(resource.max) {
            self.shards = (0..resource.max).map(|_| Shard::new(now)).collect();
        }
        let power = if resource.spec == Some(DESTRUCTION) {
            resource.tenths
        } else {
            resource.tenths / 10 * 10
        };
        let bar_full = resource.in_combat && power >= u16::from(resource.max) * 10;
        for (index, shard) in (0u16..).zip(&mut self.shards) {
            let fill = power.saturating_sub(index * 10).min(10);
            shard.update(fill, bar_full, now);
        }
    }

    fn tick(&mut self, now: f64) {
        for shard in &mut self.shards {
            shard.visual.finish(now);
        }
    }

    fn view(&self, _: &ClassBarResource, now: f64) -> ClassBarView {
        let visuals = self.shards.iter().map(|shard| &shard.visual);
        ROW.view(ClassBar::SoulShards, visuals, now)
    }
}
