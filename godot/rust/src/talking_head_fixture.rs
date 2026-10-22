//! Offline native source-payload fixture. Actual UDP relay is proved separately;
//! this fixture calls the production state/RegistryUi projection, not a fake renderer.
use crate::ui::RegistryUi;
use game_engine_ui_model::talking_head::TalkingHeadState;
use godot::{
    classes::{InputEvent, Node, ProjectSettings},
    prelude::*,
};

#[derive(GodotClass)]
#[class(base=Node)]
struct TalkingHeadFixture {
    base: Base<Node>,
    state: TalkingHeadState,
    ui: Option<Gd<RegistryUi>>,
    voice: crate::talking_head::voice::TalkingHeadVoice,
    data_root: std::path::PathBuf,
}

#[godot_api]
impl INode for TalkingHeadFixture {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            state: Default::default(),
            ui: None,
            voice: Default::default(),
            data_root: ProjectSettings::singleton()
                .globalize_path("res://../data")
                .to_string()
                .into(),
        }
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if crate::talking_head::dismiss_pointer(&mut self.state, self.ui.as_ref(), &event) {
            if let Some(mut viewport) = self.base().get_viewport() {
                viewport.set_input_as_handled();
            }
        }
    }

    fn exit_tree(&mut self) {
        self.voice.stop();
    }
}

#[godot_api]
impl TalkingHeadFixture {
    #[func]
    fn initialize(&mut self, forever: bool, payload: GString) -> GString {
        let result =
            serde_json::from_str::<shared::protocol::ConversationUpdate>(&payload.to_string())
                .map_err(|error| error.to_string())
                .and_then(|update| self.initialize_native(forever, update));
        result
            .err()
            .map_or_else(GString::new, |error| error.as_str().into())
    }

    #[func]
    fn initialize_at_elapsed(
        &mut self,
        forever: bool,
        payload: GString,
        elapsed_ms: i64,
    ) -> GString {
        let result =
            serde_json::from_str::<shared::protocol::ConversationUpdate>(&payload.to_string())
                .map_err(|error| error.to_string())
                .and_then(|mut update| {
                    let elapsed = u64::try_from(elapsed_ms)
                        .map_err(|_| "negative fixture elapsed".to_owned())?;
                    for conversation in &mut update.conversations {
                        conversation.elapsed_ms = elapsed;
                    }
                    self.initialize_native(forever, update)
                });
        result
            .err()
            .map_or_else(GString::new, |error| error.as_str().into())
    }
    #[func]
    fn receive(&mut self, payload: GString, owner: i64) -> GString {
        let result =
            serde_json::from_str::<shared::protocol::ConversationUpdate>(&payload.to_string())
                .map_err(|error| error.to_string())
                .and_then(|update| {
                    let owner =
                        u64::try_from(owner).map_err(|_| "negative fixture owner".to_owned())?;
                    self.state.receive(update, owner)
                });
        result
            .err()
            .map_or_else(GString::new, |error| error.as_str().into())
    }

    #[func]
    fn tick(&mut self, delta: f64) -> GString {
        crate::talking_head::advance_source_presentation(
            &mut self.state,
            &mut self.voice,
            delta as f32,
        );
        let Some(ui) = &mut self.ui else {
            return "TalkingHead fixture not initialized".into();
        };
        crate::talking_head::consume_actions(&mut self.state, &mut ui.bind_mut());
        let result = ui.bind_mut().set_state(self.state.clone());
        let mut parent = self.base().clone().upcast();
        result
            .and_then(|()| {
                crate::talking_head::camera::sync_source_camera(
                    self.ui.as_ref().expect("initialized"),
                    self.state.current().as_ref(),
                    &self.data_root,
                )
            })
            .and_then(|()| {
                self.voice.sync(
                    self.state.current().as_ref(),
                    &mut parent,
                    &self.data_root,
                    &game_engine_core::client_options_data::SoundOptionsFile::default(),
                )
            })
            .err()
            .map_or_else(GString::new, |error| error.as_str().into())
    }
    #[func]
    fn camera_state(&self) -> VarDictionary {
        crate::talking_head::camera::camera_state(self.ui.as_ref())
    }

    #[func]
    fn voice_state(&self) -> VarDictionary {
        self.voice.snapshot()
    }
}

impl TalkingHeadFixture {
    fn initialize_native(
        &mut self,
        forever: bool,
        update: shared::protocol::ConversationUpdate,
    ) -> Result<(), String> {
        let path = ProjectSettings::singleton().globalize_path("res://../data");
        game_engine_ui_model::paths::set_data_root(std::path::PathBuf::from(path.to_string()))?;
        ui_toolkit::atlas::set_thread_skin(if forever {
            ui_toolkit::atlas::ActiveSkin::Forever
        } else {
            ui_toolkit::atlas::ActiveSkin::Modern
        });
        let owner = update
            .conversations
            .first()
            .ok_or("native source fixture needs a conversation")?
            .private_owner;
        self.state.receive(update, owner)?;
        let mut ui = RegistryUi::new_alloc();
        self.base_mut().add_child(&ui);
        let result = ui.bind_mut().show_talking_head(self.state.clone());
        self.ui = Some(ui);
        self.base_mut().set_process_input(true);
        result
    }
}
