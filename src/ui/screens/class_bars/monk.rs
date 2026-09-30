//! `MonkLightEnergyTemplate` / `MonkHarmonyBarFrame` (MonkHarmonyBar.xml:4-129,
//! MonkHarmonyBar.lua), atlas 2224 `uimonkchi.blp` FDID 4744156.

use super::super::inworld_unit_frames_art::{AtlasArt, FlipBook, art, key};
use super::anim::{PointTemplate, group, layer, rotate, translate};
use super::{ActivePoints, BarLogic, ClassBarView, Row, Set, SetActive};
use crate::status::{ClassBar, ClassBarResource};

const fn chi(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_744_156, (256.0, 256.0), rect)
}

const BG: usize = 0;
const BG_GLOW: usize = 1;
const BG_ACTIVE: usize = 2;
const WIND: usize = 3;
const FX_2: usize = 4;
const ICON: usize = 5;
const DEPLETE: usize = 6;
const OUTER_GLOW: usize = 7;
const SMOKE: usize = 8;

static TEMPLATE: PointTemplate = PointTemplate {
    layers: &[
        // BACKGROUND: uf-chi-bg (21139) y -2.5, uf-chi-fx-bgglow (21143) y -1,
        // uf-chi-bg-active (21138) y -2.5.
        layer("Chi_BG", chi((225.0, 250.0, 43.0, 68.0)), (0.0, -2.5)),
        layer("Chi_BG_Glow", chi((1.0, 36.0, 111.0, 146.0)), (0.0, -1.0)),
        layer(
            "Chi_BG_Active",
            chi((225.0, 250.0, 70.0, 95.0)),
            (0.0, -2.5),
        ),
        // ARTWORK: uf-chi-windfx (21149) 37×36 at (-1, 3), uf-chi-fx-2 (21141).
        layer("FB_Wind_FX", chi((1.0, 223.0, 1.0, 109.0)), (-1.0, 3.0)).sized((37.0, 36.0)),
        layer("Chi_FX_2", chi((17.0, 31.0, 191.0, 205.0)), (0.0, 0.0)),
        // ARTWORK 2: uf-chi-icon (21146), uf-chi-fx-deplete (21144).
        layer("Chi_Icon", chi((1.0, 15.0, 174.0, 189.0)), (0.0, 0.0)),
        layer("Chi_Deplete", chi((17.0, 31.0, 207.0, 221.0)), (0.0, 0.0)),
        // ARTWORK 3: uf-chi-outerglow (21148) y -0.5, uf-chi-fx-smoke (21145) y 6.
        layer("FX_OuterGlow", chi((1.0, 25.0, 148.0, 172.0)), (0.0, -0.5)),
        layer("FX_Smoke", chi((225.0, 245.0, 1.0, 41.0)), (0.0, 6.0)),
        // OVERLAY: uf-chi-orbgleam (21147).
        layer("Orb_Gleam", chi((17.0, 31.0, 174.0, 189.0)), (0.0, 0.0)),
    ],
    groups: &[
        // activate (MonkHarmonyBar.xml:70-88).
        group(
            &[
                (OUTER_GLOW, key(0.0, 1.0, 0.0, 0.17)),
                (OUTER_GLOW, key(1.0, 1.0, 0.17, 0.1)),
                (OUTER_GLOW, key(1.0, 0.0, 0.18, 0.65)),
                (ICON, key(0.0, 0.0, 0.0, 0.4)),
                (ICON, key(0.0, 1.0, 0.4, 0.43)),
                (FX_2, key(0.0, 1.0, 0.0, 0.4)),
                (FX_2, key(1.0, 0.0, 0.4, 0.43)),
                (BG_ACTIVE, key(0.0, 1.0, 0.0, 0.15)),
                (BG, key(1.0, 1.0, 0.0, 0.17)),
                (BG, key(1.0, 0.0, 0.17, 0.1)),
                (BG_GLOW, key(0.0, 1.0, 0.0, 0.17)),
                (BG_GLOW, key(1.0, 1.0, 0.17, 0.33)),
                (BG_GLOW, key(1.0, 0.0, 0.5, 0.33)),
            ],
            &[(
                WIND,
                FlipBook {
                    columns: 6,
                    rows: 3,
                    frames: 17,
                    duration: 0.57,
                },
            )],
        )
        .turning(&[(FX_2, rotate(-65.0, 0.0, 0.43))]),
        // deactivate (MonkHarmonyBar.xml:89-100).
        group(
            &[
                (OUTER_GLOW, key(1.0, 1.0, 0.0, 0.33)),
                (OUTER_GLOW, key(1.0, 0.0, 0.33, 0.17)),
                (SMOKE, key(1.0, 1.0, 0.0, 0.3)),
                (SMOKE, key(1.0, 0.0, 0.3, 0.2)),
                (ICON, key(1.0, 0.0, 0.0, 0.2)),
                (DEPLETE, key(1.0, 0.0, 0.0, 0.5)),
            ],
            &[],
        )
        .moving(&[(SMOKE, translate(0.0, 5.0, 0.0, 0.5))])
        .turning(&[(DEPLETE, rotate(-30.0, 0.0, 0.5))]),
    ],
};

/// `MonkLightEnergyMixin` (MonkHarmonyBar.lua:25-68): `ResetVisuals` zeroes `Chi_Icon`,
/// `Chi_BG_Active` and the `fxTextures`; activating first shows the wind flipbook,
/// deactivating first restores `Chi_BG`.
static MIXIN: SetActive = SetActive {
    template: &TEMPLATE,
    reset: &[
        Set::Alpha(ICON, 0.0),
        Set::Alpha(BG_ACTIVE, 0.0),
        Set::Alpha(BG_GLOW, 0.0),
        Set::Alpha(WIND, 0.0),
        Set::Alpha(FX_2, 0.0),
        Set::Alpha(DEPLETE, 0.0),
        Set::Alpha(OUTER_GLOW, 0.0),
        Set::Alpha(SMOKE, 0.0),
    ],
    activate: (0, &[Set::Alpha(WIND, 1.0)]),
    deactivate: (1, &[Set::Alpha(BG, 1.0)]),
};

/// Chi orbs 3 apart, 2 from six orbs on (MonkHarmonyBar.lua:1-22); 21×21, `topPadding` 7
/// (MonkHarmonyBar.xml:5,124).
fn row(max: u8) -> Row {
    Row {
        cell: (21.0, 21.0),
        spacing: if max >= 6 { 2.0 } else { 3.0 },
        padding: (7.0, 0.0),
        scale: 1.0,
    }
}

#[derive(Debug)]
pub struct Bar(ActivePoints);

impl Default for Bar {
    fn default() -> Self {
        Self(ActivePoints::new(&MIXIN))
    }
}

impl BarLogic for Bar {
    /// `MonkPowerBar:UpdatePower`: `SetActive(i <= numChi)`.
    fn power(&mut self, resource: &ClassBarResource, now: f64) {
        self.0.power(resource.current, resource.max, now);
    }

    fn tick(&mut self, now: f64) {
        self.0.tick(now);
    }

    fn view(&self, resource: &ClassBarResource, now: f64) -> ClassBarView {
        row(resource.max).view(ClassBar::Chi, self.0.visuals(), now)
    }
}
