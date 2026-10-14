//! Test-only atlas census panel through the production RegistryUi projection.
use super::RegistryUi;
use game_engine_ui_model::quest_art::{DynName, named_atlas_texture};
use godot::prelude::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    rsx,
    screen::SharedContext,
    widget_def::Element,
};

#[derive(Clone)]
struct AtlasProofView {
    names: Vec<String>,
    skin: String,
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_db2_atlas_proof(&mut self, names: Array<GString>, forever: bool) -> GString {
        let skin = if forever {
            ActiveSkin::Forever
        } else {
            ActiveSkin::Modern
        };
        set_thread_skin(skin);
        let view = AtlasProofView {
            names: names.iter_shared().map(|name| name.to_string()).collect(),
            skin: if forever { "Forever" } else { "Modern" }.into(),
        };
        let result = self
            .set_ui_scale(1.0)
            .and_then(|()| self.show_quest_window(view, atlas_proof_screen));
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

fn atlas_proof_screen(context: &SharedContext) -> Element {
    let view = context.get::<AtlasProofView>().expect("atlas proof view");
    let mut samples = Element::default();
    for (index, name) in view.names.iter().enumerate() {
        let left = 24.0 + (index % 4) as f32 * 278.0;
        let top = 62.0 + (index / 4) as f32 * 110.0;
        let image = named_atlas_texture(
            format!("Db2AtlasProofImage{index}"),
            name,
            (0.0, 0.0, 48.0, 48.0),
        );
        samples.extend(rsx! { r#frame { name: {DynName(format!("Db2AtlasProofSample{index}"))},
            width: 266.0, height: 102.0, pos_type: "absolute", left, top,
            {image}
            fontstring { name: {DynName(format!("Db2AtlasProofName{index}"))}, text: {name.clone()},
                width: 260.0, height: 38.0, pos_type: "absolute", left: 0.0, top: 54.0, font_size: 12.0 }
        }});
    }
    let title = format!(
        "Installed DB2 atlas proof — {} — normal atlas path",
        view.skin
    );
    rsx! { r#frame { name: "Db2AtlasProof", width: 1140.0, height: 660.0,
        pos_type: "absolute", left: 30.0, top: 30.0, background_color: "0.04,0.04,0.04,1.0",
        fontstring { name: "Db2AtlasProofTitle", text: title, width: 1100.0, height: 30.0,
            pos_type: "absolute", left: 24.0, top: 14.0, font_size: 18.0 }
        {samples}
    }}
}
