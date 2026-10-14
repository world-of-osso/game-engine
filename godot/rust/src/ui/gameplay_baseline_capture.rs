//! Temporary retrospective capture of unmodified dc7c36424 control renderers.
//! Removed after PNG capture; archived source/provenance stays in the private proof folder.
use super::{RegistryUi, ScreenPostsetup, hud_edit_preview, party_preview};
use game_engine_ui_model::{
    gameplay_baseline_cooldown_settings_component as cooldown,
    gameplay_baseline_hud_edit_component as hud,
};
use godot::prelude::*;
use ui_toolkit::{atlas::ActiveSkin, screen::SharedContext, widget_def::Element};

#[derive(Clone, PartialEq)]
struct Baseline {
    skin: ActiveSkin,
    cooldown: bool,
}
fn screen(ctx: &SharedContext) -> Element {
    let baseline = ctx.get::<Baseline>().expect("Baseline");
    if baseline.cooldown {
        cooldown::settings_panel(&cooldown::CooldownSettingsView {
            open: true,
            entries: vec![
                cooldown::CooldownSettingEntry {
                    id: 26,
                    category: 0,
                    label: "Colossus Smash".into(),
                    selected: true,
                },
                cooldown::CooldownSettingEntry {
                    id: 27,
                    category: 0,
                    label: "Mortal Strike".into(),
                    selected: true,
                },
            ],
            ..Default::default()
        })
    } else {
        let mut shared = SharedContext::new();
        shared.insert(hud::EditModePanelState {
            layout_name: format!("{:?}", baseline.skin),
            layout_names: vec!["Modern".into(), "Forever".into()],
            preset: true,
            ..Default::default()
        });
        hud::edit_mode_panel_screen(&shared)
    }
}
#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_gameplay_baseline_preview(&mut self, forever: bool, cooldown: bool) -> GString {
        let result = (|| {
            party_preview::load_data_root()?;
            let skin = if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            };
            ui_toolkit::atlas::set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            let parent = self.hud_parent()?;
            let mut registry = parent.registry();
            hud_edit_preview::register_edit_panel_style(&mut registry);
            self.show_viewport_screen_in(
                Baseline { skin, cooldown },
                screen,
                ScreenPostsetup::None,
                registry,
                parent,
            )
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
