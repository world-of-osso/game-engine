//! Offline concrete profession snapshot through the production screen; no network/catalog worker.
use super::{RegistryUi, party_preview};
use game_engine_ui_model::{
    professions::{ProfessionBook, Recipe},
    professions_frame::{ItemDisplay, ProfessionView},
};
use godot::prelude::*;
use shared::{profession::ProfessionSkillLine, protocol::ProfessionSnapshot};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_professions_preview(&mut self) -> GString {
        self.show_professions_preview_skin(ActiveSkin::Modern)
    }
    #[func]
    fn show_forever_professions_preview(&mut self) -> GString {
        self.show_professions_preview_skin(ActiveSkin::Forever)
    }
    fn show_professions_preview_skin(&mut self, skin: ActiveSkin) -> GString {
        let result = party_preview::load_data_root().and_then(|()| {
            set_thread_skin(skin);
            self.set_ui_scale(1.0)?;
            let view = preview_view();
            cache_preview_art(&view)?;
            self.show_professions(view)?;
            self.set_editbox_text("ProfessionsQuantity", "1")
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

fn cache_preview_art(view: &ProfessionView) -> Result<(), String> {
    use godot::obj::Singleton;
    let path = godot::classes::ProjectSettings::singleton().globalize_path("res://../data");
    let root = std::path::PathBuf::from(path.to_string());
    let resolver = crate::assets::creature::local_resolver(&root);
    let fdids = crate::quests::screen_texture_fdids(
        view.clone(),
        game_engine_ui_model::professions_frame::professions_screen,
    );
    for fdid in fdids
        .into_iter()
        .chain(game_engine_ui_model::panel_style_data::metal_sheet_fdids(
            game_engine_ui_model::panel_style_data::MetalTopLeft::Portrait,
        ))
        .chain([130924])
    {
        let path = root.join("textures").join(format!("{fdid}.blp"));
        if !path.exists() && resolver.ensure_cached(fdid, &path).is_none() {
            return Err(format!(
                "Profession preview FDID {fdid} missing from local CASC"
            ));
        }
    }
    Ok(())
}

fn preview_view() -> ProfessionView {
    let recipes = preview_recipes();
    let spells = recipes.iter().map(|recipe| recipe.spell_id).collect();
    ProfessionView {
        book: ProfessionBook {
            snapshot: ProfessionSnapshot {
                lines: vec![ProfessionSkillLine {
                    skill_line: 2540,
                    step: 1,
                    rank: 35,
                    max_rank: 300,
                }],
                spells,
            },
            recipes,
            visible: true,
            selected: Some(7623),
            quantity: 1,
            ..Default::default()
        },
        profession_name: "Tailoring".into(),
        profession_icon: 136249,
        skill_names: [(2540, "Classic Tailoring".into())].into(),
        items: [
            (
                6241,
                ItemDisplay {
                    name: "Brown Linen Robe".into(),
                    icon_fdid: 132662,
                    quality: 2,
                },
            ),
            (
                2996,
                ItemDisplay {
                    name: "Bolt of Linen Cloth".into(),
                    icon_fdid: 132890,
                    quality: 1,
                },
            ),
            (
                2320,
                ItemDisplay {
                    name: "Coarse Thread".into(),
                    icon_fdid: 132891,
                    quality: 1,
                },
            ),
        ]
        .into(),
        reagents: vec![(2996, 2, 3), (2320, 4, 1)],
        recipe_craftable: [(2963, 4), (3275, 8), (3276, 4)].into(),
        ..Default::default()
    }
}

fn preview_recipes() -> Vec<Recipe> {
    // Fixed threshold fixtures deliberately cover all four colours at rank 35.
    [
        (3275, "Bandages", "Linen Bandage", 1, 20, 30),
        (3276, "Bandages", "Heavy Linen Bandage", 20, 25, 40),
        (2963, "Cloth", "Bolt of Linen Cloth", 1, 25, 50),
        (2964, "Cloth", "Bolt of Woolen Cloth", 30, 45, 70),
        (7623, "Robes", "Brown Linen Robe", 1, 25, 50),
        (7624, "Robes", "White Linen Robe", 30, 40, 60),
    ]
    .into_iter()
    .map(
        |(spell_id, category, name, min_rank, trivial_low, trivial_high)| Recipe {
            spell_id,
            skill_line: 2540,
            profession: 197,
            category: category.into(),
            name: name.into(),
            min_rank,
            trivial_low,
            trivial_high,
            output: (6241, 1),
            reagents: vec![(2996, 3), (2320, 1)],
        },
    )
    .collect()
}
