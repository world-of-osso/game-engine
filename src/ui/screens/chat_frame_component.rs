//! Retail-style tabbed chat frame (`ChatFrame1`), bottom-left of the screen.

use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::rsx;
use ui_toolkit::screen::SharedContext;
use ui_toolkit::widget_def::{Attr, Element, WidgetChild};
use ui_toolkit::widgets::scroll_list::{ScrollList, scroll_list};
use ui_toolkit::widgets::tabs::{Tab, TabStrip, tab_strip};

use crate::ui::anchor::FrameName;
use crate::ui::chat_frame::{ChatRow, ChatRun, ChatTab};
use crate::ui::widgets::font_string::{FontColor, GameFont};

pub const CHAT_FRAME: FrameName = FrameName("ChatFrame1");
pub const CHAT_EDITBOX: FrameName = FrameName("ChatFrame1EditBox");
pub const CHAT_BACKGROUND: FrameName = FrameName("ChatFrame1Background");
pub const CHAT_TABS: &str = "ChatFrame1Tabs";
pub const CHAT_MESSAGES: &str = "ChatFrame1Messages";

/// Plan layout: 430x200 reference units, 16 from the left and bottom edges.
const FRAME_W: f32 = 430.0;
const FRAME_H: f32 = 200.0;
const FRAME_INSET: f32 = 16.0;
const TAB_W: f32 = 96.0;
const TAB_H: f32 = 22.0;
const TAB_GAP: f32 = 2.0;
const PAD: f32 = 4.0;
const INPUT_H: f32 = 22.0;
const MESSAGES_TOP: f32 = TAB_H + PAD;
const MESSAGES_W: f32 = FRAME_W - 2.0 * PAD;
const MESSAGES_H: f32 = FRAME_H - MESSAGES_TOP - INPUT_H - PAD;
const TRACK_W: f32 = 8.0;
pub const CHAT_ROW_H: f32 = 16.0;
pub const CHAT_FONT: GameFont = GameFont::ArialNarrow;
pub const CHAT_FONT_SIZE: f32 = 14.0;
/// Width available to message text; lines wrap to it.
pub const CHAT_TEXT_W: f32 = MESSAGES_W - TRACK_W - PAD;
/// Background alpha while the frame is in use; it fades to 0 when idle.
pub const CHAT_BACKGROUND_ALPHA: f32 = 0.4;
/// Dark plate behind the selected tab so its gold label stays legible over the world.
const SELECTED_TAB_BG: &str = "0,0,0,0.5";
const LINK_PREFIX: &str = "ChatFrame1Link";

struct DynName(String);

#[derive(Clone, Debug, PartialEq)]
pub struct ChatFrameView {
    pub tab: ChatTab,
    pub rows: Vec<ChatRow>,
    pub input_open: bool,
    pub background_alpha: f32,
}

pub fn chat_frame_screen(ctx: &SharedContext) -> Element {
    let view = ctx
        .get::<ChatFrameView>()
        .expect("ChatFrameView must be in SharedContext");
    let background = format!("0,0,0,{}", view.background_alpha);
    let hide_input = !view.input_open;
    rsx! {
        r#frame {
            name: CHAT_FRAME,
            width: FRAME_W,
            height: FRAME_H,
            pos_type: "absolute",
            left: FRAME_INSET,
            bottom: FRAME_INSET,
            r#frame {
                name: CHAT_BACKGROUND,
                width: FRAME_W,
                height: {FRAME_H - TAB_H},
                background_color: {background},
                pos_type: "absolute",
                left: 0.0,
                top: TAB_H,
            }
            {tabs(view.tab)}
            r#frame {
                name: "ChatFrame1MessagesAnchor",
                width: MESSAGES_W,
                height: MESSAGES_H,
                pos_type: "absolute",
                left: PAD,
                top: MESSAGES_TOP,
                {messages(ctx, &view.rows)}
            }
            editbox {
                name: CHAT_EDITBOX,
                width: MESSAGES_W,
                height: INPUT_H,
                font_size: CHAT_FONT_SIZE,
                hidden: hide_input,
                pos_type: "absolute",
                left: PAD,
                top: {FRAME_H - INPUT_H},
            }
        }
    }
}

fn tabs(selected: ChatTab) -> Element {
    let tabs = ChatTab::ALL.map(|tab| Tab {
        label: tab.label(),
        action: tab.action(),
        disabled: false,
    });
    let strip = tab_strip(TabStrip {
        name: CHAT_TABS,
        tabs: &tabs,
        selected: selected.index(),
        tab_width: TAB_W,
        tab_height: TAB_H,
        gap: TAB_GAP,
        art: None,
    });
    let strip = with_size(strip, FRAME_W, TAB_H);
    let highlight_x = selected.index() as f32 * (TAB_W + TAB_GAP);
    rsx! {
        r#frame {
            name: "ChatFrame1TabSelected",
            width: TAB_W,
            height: TAB_H,
            background_color: SELECTED_TAB_BG,
            pos_type: "absolute",
            left: {highlight_x},
            top: 0.0,
        }
        r#frame {
            name: "ChatFrame1TabsAnchor",
            width: FRAME_W,
            height: TAB_H,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
            {strip}
        }
    }
}

/// A flex strip without a size lays out 0 wide and shrinks every tab to nothing.
fn with_size(mut element: Element, width: f32, height: f32) -> Element {
    if let Some(WidgetChild::Widget(def)) = element.first_mut() {
        def.attrs
            .push(Attr::new_dynamic("width", width.to_string()));
        def.attrs
            .push(Attr::new_dynamic("height", height.to_string()));
    }
    element
}

fn messages(ctx: &SharedContext, rows: &[ChatRow]) -> Element {
    scroll_list(
        ctx,
        ScrollList {
            name: CHAT_MESSAGES,
            width: MESSAGES_W,
            height: MESSAGES_H,
            row_height: CHAT_ROW_H,
            row_count: rows.len(),
            track_width: TRACK_W,
            track_fdid: None,
            thumb_fdid: None,
        },
        |index| {
            rows[index]
                .runs
                .iter()
                .enumerate()
                .flat_map(|(run_index, run)| message_run(index, run_index, run))
                .collect()
        },
    )
}

fn message_run(row: usize, run_index: usize, run: &ChatRun) -> Element {
    let [r, g, b, a] = run.color;
    let is_link = run.spell_id.is_some();
    let name = match run.spell_id {
        Some(spell) => format!("{LINK_PREFIX}{row}_{run_index}_{spell}"),
        None => format!("{CHAT_MESSAGES}Row{row}Run{run_index}"),
    };
    rsx! {
        fontstring {
            name: {DynName(name)},
            width: {run.width.ceil() + 1.0},
            height: CHAT_ROW_H,
            text: {run.text.as_str()},
            font: CHAT_FONT,
            font_size: CHAT_FONT_SIZE,
            font_color: {FontColor::new(r, g, b, a)},
            justify_h: "LEFT",
            mouse_enabled: is_link,
            pos_type: "absolute",
            left: {run.x},
            top: 0.0,
        }
    }
}

/// Spell id of the chat link frame at or above `frame_id`.
pub fn chat_spell_link_at(registry: &FrameRegistry, mut frame_id: u64) -> Option<u32> {
    loop {
        let frame = registry.get(frame_id)?;
        if let Some(spell) = frame
            .name
            .as_deref()
            .and_then(|name| name.strip_prefix(LINK_PREFIX))
            .and_then(|rest| rest.rsplit('_').next())
            .and_then(|spell| spell.parse().ok())
        {
            return Some(spell);
        }
        frame_id = frame.parent_id?;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::chat_frame::SPELL_LINK_COLOR;
    use crate::ui::screens::screen_test_helpers::fontstring_text;
    use ui_toolkit::screen::Screen;

    fn run(text: &str, x: f32, spell_id: Option<u32>) -> ChatRun {
        ChatRun {
            text: text.to_string(),
            color: if spell_id.is_some() {
                SPELL_LINK_COLOR
            } else {
                [1.0; 4]
            },
            x,
            width: 60.0,
            spell_id,
        }
    }

    fn build(rows: Vec<ChatRow>) -> FrameRegistry {
        let mut reg = FrameRegistry::new(1920.0, 1080.0);
        let mut shared = SharedContext::new();
        shared.insert(ChatFrameView {
            tab: ChatTab::CombatLog,
            rows,
            input_open: false,
            background_alpha: 0.0,
        });
        Screen::new(chat_frame_screen).sync(&shared, &mut reg);
        reg
    }

    #[test]
    fn link_runs_resolve_to_their_spell_and_text_runs_do_not() {
        let reg = build(vec![ChatRow {
            runs: vec![
                run("Alice's ", 0.0, None),
                run("[Fireball]", 60.0, Some(133)),
            ],
        }]);
        let link = reg
            .get_by_name("ChatFrame1Link0_1_133")
            .expect("link frame");
        assert_eq!(fontstring_text(&reg, "ChatFrame1Link0_1_133"), "[Fireball]");
        assert_eq!(chat_spell_link_at(&reg, link), Some(133));
        let text = reg
            .get_by_name("ChatFrame1MessagesRow0Run0")
            .expect("text run");
        assert_eq!(chat_spell_link_at(&reg, text), None);
        assert!(
            reg.get(reg.get_by_name("ChatFrame1EditBox").unwrap())
                .unwrap()
                .hidden
        );
    }
}
