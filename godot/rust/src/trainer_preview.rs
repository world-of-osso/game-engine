//! Offline mixed trainer snapshot through the production screen, without GameClient or networking.
use crate::ui::RegistryUi;
use game_engine_ui_model::{
    game_tooltip::{GameTooltip, GameTooltipView, TooltipScreen, place},
    trainer::{TrainerBook, TrainerDisplay},
    trainer_frame::{TrainerView, trainer_screen},
};
use godot::{classes::ProjectSettings, prelude::*};
use shared::{
    profession::ProfessionSkillLine,
    protocol::{TrainerList, TrainerService, TrainerServiceState},
};
use ui_toolkit::atlas::{ActiveSkin, set_thread_skin};

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_trainer_preview(&mut self) -> GString {
        self.show_trainer_preview_skin(ActiveSkin::Modern)
    }
    #[func]
    fn show_forever_trainer_preview(&mut self) -> GString {
        self.show_trainer_preview_skin(ActiveSkin::Forever)
    }
    /// Offline pointer hit through the same TrainerBook source and GameTooltip host as live UI.
    #[func]
    fn update_trainer_preview_tooltip(&self, mut host: Gd<RegistryUi>, at: Vector2) -> GString {
        let result = self.trainer_preview_tooltip_at(at).and_then(|view| {
            let mut host = host.bind_mut();
            host.set_ui_scale(1.0)?;
            // show_game_tooltip initializes a screen; subsequent pointer frames update its state.
            if host.registry().is_some() {
                host.set_state(view)
            } else {
                host.show_game_tooltip(view)
            }
        });
        GString::from(result.err().unwrap_or_default().as_str())
    }

    fn show_trainer_preview_skin(&mut self, skin: ActiveSkin) -> GString {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        let result =
            game_engine_ui_model::paths::set_data_root(std::path::PathBuf::from(path.to_string()))
                .and_then(|()| {
                    set_thread_skin(skin);
                    self.set_ui_scale(1.0)?;
                    cache_trainer_art()?;
                    self.show_quest_window(preview_view(), trainer_screen)?;
                    crate::unit_portraits::attach_trainer_preview(self.to_gd())
                });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

impl RegistryUi {
    fn trainer_preview_tooltip_at(&self, at: Vector2) -> Result<GameTooltipView, String> {
        let book = preview_view().book;
        let Some((owner, spell)) = self
            .pointer_frame_at(at)
            .and_then(|hit| book.hovered_service(self.registry()?, hit))
        else {
            return Ok(GameTooltipView::default());
        };
        let rect = self
            .frame_id_rect(owner)
            .ok_or("Trainer preview owner is not visible")?;
        let tooltip = load_preview_spell_tooltip(spell)?;
        let tooltip = book
            .service_tooltip(
                spell,
                tooltip,
                [rect.position.x, rect.position.y, rect.size.x, rect.size.y],
            )
            .ok_or("Trainer preview service is no longer visible")?;
        let screen = self.read_trainer_preview_screen(at)?;
        Ok(GameTooltipView {
            main: place(tooltip.for_skin(ui_toolkit::atlas::thread_skin()), screen),
            ..Default::default()
        })
    }

    fn read_trainer_preview_screen(&self, at: Vector2) -> Result<TooltipScreen, String> {
        let viewport = self
            .base()
            .get_viewport()
            .ok_or("Trainer preview lacks viewport")?;
        let size = viewport.get_visible_rect().size;
        Ok(TooltipScreen {
            size: [size.x, size.y],
            cursor: [at.x, at.y],
        })
    }
}

fn load_preview_spell_tooltip(id: u32) -> Result<GameTooltip, String> {
    use game_engine_core::spell_catalog::{
        SpellCatalogPaths, SpellTextContext, load_spell_catalog,
    };
    use game_engine_ui_model::game_tooltip::spell::{SpellTooltipInput, spell_tooltip};
    let path = ProjectSettings::singleton().globalize_path("res://../data");
    let paths = SpellCatalogPaths::for_data_dir(&std::path::PathBuf::from(path.to_string()));
    let catalog = load_spell_catalog(&paths)?;
    let spell = catalog
        .get(id)
        .ok_or_else(|| format!("Trainer preview spell {id} missing from local catalog"))?;
    let description = catalog
        .render_description(id, &SpellTextContext::default())
        .unwrap_or_default();
    Ok(spell_tooltip(
        spell,
        &SpellTooltipInput {
            description,
            ..Default::default()
        },
    ))
}

fn cache_trainer_art() -> Result<(), String> {
    let path = ProjectSettings::singleton().globalize_path("res://../data");
    let root = std::path::PathBuf::from(path.to_string());
    let resolver = crate::assets::creature::local_resolver(&root);
    let fdids = crate::quests::screen_texture_fdids(preview_view(), trainer_screen);
    for fdid in fdids.into_iter().chain([130924]) {
        let path = root.join("textures").join(format!("{fdid}.blp"));
        if !path.exists() && resolver.ensure_cached(fdid, &path).is_none() {
            return Err(format!(
                "Trainer preview FDID {fdid} missing from local CASC"
            ));
        }
    }
    Ok(())
}

fn preview_view() -> TrainerView {
    let three_coins = std::env::var("GODOT_TRAINER_THREE_COINS").as_deref() == Ok("1");
    let wool_cost = if three_coins { 12550 } else { 500 };
    let services = [
        (3908, 10, TrainerServiceState::Available, 1, true),
        (2963, 12550, TrainerServiceState::Available, 5, false),
        (2964, wool_cost, TrainerServiceState::Unavailable, 20, false),
        (3275, 100, TrainerServiceState::Known, 1, false),
        (7623, 0, TrainerServiceState::Available, 1, false),
        (7624, 25, TrainerServiceState::Unavailable, 30, false),
        (3276, 250, TrainerServiceState::Available, 5, false),
        (2385, 100, TrainerServiceState::Available, 5, false),
    ]
    .into_iter()
    .map(
        |(spell_id, cost, state, req_level, profession)| TrainerService {
            spell_id,
            cost,
            state,
            req_level,
            profession,
            req_skill_line: if profession { 0 } else { 2540 },
            req_skill_rank: if spell_id == 2964 { 75 } else { 1 },
            req_abilities: vec![],
        },
    )
    .collect();
    let mut book = TrainerBook {
        money: 12345,
        filter_menu: std::env::var_os("GODOT_TRAINER_FILTER_MENU").is_some(),
        ..Default::default()
    };
    book.receive_list(TrainerList {
        npc: 4201,
        trainer_id: 7,
        greeting: "Welcome, apprentice.".into(),
        services,
    });
    let default_selection = std::env::var("GODOT_TRAINER_DEFAULT").as_deref() == Ok("1");
    book.select(if default_selection { 3908 } else { 2963 });
    TrainerView {
        book,
        title: "Georgio Bolero".into(),
        display: TrainerDisplay {
            names: [
                (3908, "Tailoring"),
                (2963, "Bolt of Linen Cloth"),
                (2964, "Bolt of Woolen Cloth"),
                (3275, "Linen Bandage"),
                (7623, "Brown Linen Robe"),
                (7624, "White Linen Robe"),
                (3276, "Heavy Linen Bandage"),
                (2385, "Brown Linen Vest"),
            ]
            .into_iter()
            .map(|(id, name)| (id, name.into()))
            .collect(),
            icons: [
                (3908, 136249),
                (2963, 132890),
                (2964, 132894),
                (3275, 133681),
                (7623, 132662),
                (7624, 132662),
                (3276, 133681),
                (2385, 132715),
            ]
            .into(),
            skills: [(2540, "Classic Tailoring".into())].into(),
        },
        player_level: 10,
        known_spells: [3275].into(),
        ranks: vec![ProfessionSkillLine {
            skill_line: 2540,
            step: 1,
            rank: 35,
            max_rank: 300,
        }],
    }
}
