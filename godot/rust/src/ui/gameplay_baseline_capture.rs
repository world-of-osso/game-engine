//! Temporary before capture: 642fcee41 renderers, source dimensions/anchors unchanged.
use super::{RegistryUi, ScreenPostsetup, gameplay_panels_preview, party_preview};
use game_engine_ui_model::{
    gameplay_baseline_battleground as pvp, gameplay_baseline_raid_info as raid,
};
use godot::prelude::*;
use ui_toolkit::{atlas::ActiveSkin, screen::SharedContext, widget_def::Element};

#[derive(Clone, PartialEq)]
struct Baseline {
    page: String,
    pvp: pvp::PvpView,
    raid: raid::RaidInfoState,
}
fn screen(ctx: &SharedContext) -> Element {
    let baseline = ctx.get::<Baseline>().expect("Baseline");
    let mut shared = SharedContext::new();
    if baseline.page == "raid" {
        shared.insert(baseline.raid.clone());
        raid::screen(&shared)
    } else {
        shared.insert(baseline.pvp.clone());
        pvp::pvp_screen(&shared)
    }
}
fn baseline(page: String) -> Result<Baseline, String> {
    let current = gameplay_panels_preview::raid_state()?;
    let raid = raid::RaidInfoState {
        visible: true,
        rows: current
            .rows
            .into_iter()
            .map(|row| raid::SavedInstanceRow {
                lock: row.lock,
                name: row.name,
                difficulty: row.difficulty,
                extend_disabled: row.extend_disabled,
            })
            .collect(),
        ..Default::default()
    };
    let mut pvp = pvp::PvpView {
        visible: page != "scoreboard",
        scoreboard: page == "scoreboard",
        viewport: [1920.0, 1080.0],
        snapshot: gameplay_panels_preview::scoreboard_view().snapshot,
        ..Default::default()
    };
    if page.starts_with("arena") {
        pvp.snapshot
            .as_mut()
            .expect("Baseline snapshot")
            .battleground = None;
        pvp.category = if page == "arena-rated" {
            pvp::PvpCategory::Rated
        } else {
            pvp::PvpCategory::QuickMatch
        };
    }
    Ok(Baseline { page, pvp, raid })
}
#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_gameplay_baseline_preview(&mut self, forever: bool, page: GString) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            ui_toolkit::atlas::set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            self.show_viewport_screen(baseline(page.to_string())?, screen, ScreenPostsetup::None)
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
