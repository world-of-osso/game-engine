//! `DruidComboPointTemplate` / `DruidComboPointBarFrame` (DruidComboPointBar.xml:5-113,
//! DruidComboPointBar.lua), atlas 2285 `uidruidcombopoints.blp` FDID 4883273.

use super::super::inworld_unit_frames_art::{AtlasArt, FlipBook, art, key};
use super::anim::{PointTemplate, group, layer, translate};
use super::{ActivePoints, BarLogic, ClassBarView, Row, Set, SetActive};
use crate::status::{ClassBar, ClassBarResource};

const fn druid(rect: (f32, f32, f32, f32)) -> AtlasArt {
    art(4_883_273, (256.0, 256.0), rect)
}

const BG_ACTIVE: usize = 1;
const BG_INACTIVE: usize = 2;
const BG_GLOW: usize = 3;
const DEPLETE: usize = 4;
const ICON: usize = 5;
const RING_GLOW: usize = 6;
const SLASH: usize = 7;
const SMOKE: usize = 8;

static TEMPLATE: PointTemplate = PointTemplate {
    layers: &[
        // BACKGROUND 1: UF-DruidCP-BG-Shadow (19528) y -2, -BG-Active (19525), -BG-Dis
        // (19526).
        layer("BG_Shadow", druid((1.0, 26.0, 126.0, 151.0)), (0.0, -2.0)),
        layer("BG_Active", druid((211.0, 233.0, 89.0, 111.0)), (0.0, 0.0)),
        layer("BG_Inactive", druid((1.0, 23.0, 178.0, 200.0)), (0.0, 0.0)),
        // BACKGROUND 2: UF-DruidCP-BG-Glow (19527).
        layer("BG_Glow", druid((211.0, 247.0, 1.0, 37.0)), (0.0, 0.0)),
        // ARTWORK: UF-DruidCP-Deplete (19529), UF-DruidCP-Icon (19530).
        layer(
            "Point_Deplete",
            druid((235.0, 251.0, 39.0, 54.0)),
            (0.0, 0.0),
        ),
        layer("Point_Icon", druid((235.0, 249.0, 73.0, 87.0)), (0.0, 0.0)),
        // OVERLAY: UF-DruidCP-Ring-Glow (19532), UF-DruidCP-Slash (19533) 26×41 at
        // (1, 3), UF-DruidCP-Smoke (19534) at y 15.
        layer("FX_RingGlow", druid((1.0, 24.0, 153.0, 176.0)), (0.0, 0.0)),
        layer("FB_Slash", druid((1.0, 209.0, 1.0, 124.0)), (1.0, 3.0)).sized((26.0, 41.0)),
        layer("Smoke", druid((211.0, 233.0, 39.0, 87.0)), (0.0, 15.0)),
    ],
    groups: &[
        // activateAnim (DruidComboPointBar.xml:64-77).
        group(
            &[
                (ICON, key(0.0, 0.5, 0.0, 0.1)),
                (ICON, key(0.5, 1.0, 0.47, 0.2)),
                (RING_GLOW, key(0.0, 1.0, 0.0, 0.27)),
                (RING_GLOW, key(1.0, 0.0, 0.27, 0.47)),
                (BG_ACTIVE, key(0.0, 0.0, 0.0, 0.27)),
                (BG_ACTIVE, key(0.0, 1.0, 0.27, 0.01)),
                (BG_INACTIVE, key(1.0, 1.0, 0.0, 0.27)),
                (BG_INACTIVE, key(1.0, 0.0, 0.27, 0.01)),
                (BG_GLOW, key(0.0, 0.0, 0.0, 0.17)),
                (BG_GLOW, key(0.0, 1.0, 0.17, 0.13)),
                (BG_GLOW, key(1.0, 0.0, 0.3, 0.4)),
            ],
            &[(
                SLASH,
                FlipBook {
                    columns: 8,
                    rows: 3,
                    frames: 20,
                    duration: 1.0,
                },
            )],
        ),
        // deactivateAnim (DruidComboPointBar.xml:78-87).
        group(
            &[
                (SMOKE, key(1.0, 1.0, 0.0, 0.33)),
                (SMOKE, key(1.0, 0.0, 0.33, 0.23)),
                (RING_GLOW, key(1.0, 1.0, 0.0, 0.43)),
                (RING_GLOW, key(1.0, 0.0, 0.43, 0.23)),
                (ICON, key(1.0, 0.0, 0.0, 0.2)),
                (DEPLETE, key(1.0, 1.0, 0.0, 0.23)),
                (DEPLETE, key(1.0, 0.0, 0.23, 0.2)),
            ],
            &[],
        )
        .moving(&[(SMOKE, translate(0.0, 7.0, 0.0, 0.56))]),
    ],
};

/// `DruidComboPointMixin` (DruidComboPointBar.lua:22-60): `ResetVisuals` hides `FB_Slash`
/// and zeroes the `fxTextures`; activating shows `FB_Slash` again.
static MIXIN: SetActive = SetActive {
    template: &TEMPLATE,
    reset: &[
        Set::Shown(SLASH, false),
        Set::Alpha(BG_GLOW, 0.0),
        Set::Alpha(DEPLETE, 0.0),
        Set::Alpha(RING_GLOW, 0.0),
        Set::Alpha(SMOKE, 0.0),
    ],
    activate: (0, &[Set::Shown(SLASH, true)]),
    deactivate: (1, &[]),
};

/// 20×20 points 4 apart, `topPadding` 7 (DruidComboPointBar.xml:6,101,110).
const ROW: Row = Row {
    cell: (20.0, 20.0),
    spacing: 4.0,
    padding: (7.0, 0.0),
    scale: 1.0,
};

#[derive(Debug)]
pub struct Bar(ActivePoints);

impl Default for Bar {
    fn default() -> Self {
        Self(ActivePoints::new(&MIXIN))
    }
}

impl BarLogic for Bar {
    /// `DruidComboPointBarMixin:UpdatePower`: `SetActive(i <= comboPoints)`.
    fn power(&mut self, resource: &ClassBarResource, now: f64) {
        self.0.power(resource.current, resource.max, now);
    }

    fn tick(&mut self, now: f64) {
        self.0.tick(now);
    }

    fn view(&self, _: &ClassBarResource, now: f64) -> ClassBarView {
        ROW.view(ClassBar::DruidComboPoints, self.0.visuals(), now)
    }
}
