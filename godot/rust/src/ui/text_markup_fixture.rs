//! Isolated FontString projection contract; no account, catalog or realm injection.
use super::super::RegistryUi;
use godot::prelude::*;
use ui_toolkit::{
    atlas::{ActiveSkin, set_thread_skin},
    rsx,
    screen::SharedContext,
    widget_def::Element,
};

#[derive(Clone, PartialEq)]
struct TextMarkupState(String);

fn text_markup_screen(shared: &SharedContext) -> Element {
    let text = &shared.get::<TextMarkupState>().unwrap().0;
    rsx! {
        r#frame { name: "TextMarkupPanel", width: 900., height: 300.,
            fontstring { name: "TextMarkupValue", text: text.as_str(),
                left: 100., top: 100., width: 700., height: 80., pos_type: "absolute",
                font_size: 20., font_color: "1,1,1,1", justify_h: "LEFT",
                fontstring { name: "TextMarkupChild", text: "Child preserved", left: 0., top: 50.,
                    width: 200., height: 25., pos_type: "absolute", font_size: 12.
                }
            }
        }
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_text_markup_fixture(&mut self, forever: bool) -> GString {
        let result = (|| {
            super::super::party_preview::load_data_root()?;
            set_thread_skin(if forever {
                ActiveSkin::Forever
            } else {
                ActiveSkin::Modern
            });
            self.show_standalone_screen(
                TextMarkupState("Earth Breakers continually cast [Skeletal Fractures].".into()),
                text_markup_screen,
            )
        })();
        result.err().unwrap_or_default().as_str().into()
    }

    #[func]
    fn text_markup_fixture_step(&mut self, text: GString) -> GString {
        self.set_state(TextMarkupState(text.to_string()))
            .err()
            .unwrap_or_default()
            .as_str()
            .into()
    }
}
