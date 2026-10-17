//! Fixed supplied timer state through the production HUD host; no server clock proof.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::mirror_timer_data::{
    MirrorTimerKind, MirrorTimerStart, MirrorTimersData,
};
use godot::prelude::*;
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

fn start_paused_timers(timers: &mut MirrorTimersData) {
    for (kind, value) in [
        (MirrorTimerKind::Breath, 180_000),
        (MirrorTimerKind::Exhaustion, 90_000),
        (MirrorTimerKind::FeignDeath, 45_000),
    ] {
        timers.start(MirrorTimerStart {
            kind,
            value,
            max_value: 180_000,
            scale: -1.0,
            paused: true,
        });
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_mirror_timer_shadow_preview(&mut self, forever: bool) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.set_ui_scale(1.0)?;
            self.show_mirror_timers()?;
            self.update_mirror_timers(start_paused_timers)
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
