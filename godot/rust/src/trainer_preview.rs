//! Offline mixed trainer snapshot through the production screen, without GameClient or networking.
use crate::ui::RegistryUi;
use game_engine_ui_model::{
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
    fn show_trainer_preview_skin(&mut self, skin: ActiveSkin) -> GString {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        let result =
            game_engine_ui_model::paths::set_data_root(std::path::PathBuf::from(path.to_string()))
                .and_then(|()| {
                    set_thread_skin(skin);
                    self.set_ui_scale(1.0)?;
                    self.show_quest_window(preview_view(), trainer_screen)
                });
        GString::from(result.err().unwrap_or_default().as_str())
    }
}

fn preview_view() -> TrainerView {
    let services = [
        (3908, 10, TrainerServiceState::Available, 1, true),
        (2963, 12550, TrainerServiceState::Available, 5, false),
        (2964, 500, TrainerServiceState::Unavailable, 20, false),
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
        filter_menu: true,
        ..Default::default()
    };
    book.receive_list(TrainerList {
        npc: 4201,
        trainer_id: 7,
        greeting: "Welcome, apprentice.".into(),
        services,
    });
    book.select(2963);
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
        ranks: vec![ProfessionSkillLine {
            skill_line: 2540,
            step: 1,
            rank: 35,
            max_rank: 300,
        }],
    }
}
