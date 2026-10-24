//! Mainline Communities guild streams over the existing chat/history service.
use crate::guild_ranks::{GuildCommand, GuildRanksSession};
use crate::menu_primitives::{
    ContextMenu, ContextMenuItem, DropdownButton, DropdownState, MenuElementKind,
    context_menu_with_rows, dropdown_button_with_state, menu_element,
};
use crate::minimal_scroll_bar::{MinimalScrollBar, Unscrollable, pixel_geometry, scroll_list_attr};
use crate::quest_art::DynName;
use shared::protocol::{ChatHistoryResponse, ChatMessage, ChatType};
use std::collections::VecDeque;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element, widgets::font_string::GameFont};

pub const EDITOR: &str = "CommunitiesChatEditBox";
pub const SCROLL_BOX: &str = "CommunitiesChatMessageFrame";
pub const LINE_HEIGHT: f32 = 14.0;
pub const MAX_MESSAGE_BYTES: usize = 512;
const MAX_MESSAGES: usize = 5000;
const CHAT_RECT: (f32, f32, f32, f32) = (201.0, 67.0, 390.0, 303.0);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GuildChatLine {
    pub sender: String,
    pub text: String,
    pub officer: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GuildChatSession {
    pub officer: bool,
    pub menu_open: bool,
    pub waiting: bool,
    pub messages: VecDeque<GuildChatLine>,
    pub error: Option<String>,
}

impl GuildChatSession {
    pub fn query(&mut self) -> Option<GuildCommand> {
        if self.waiting {
            return None;
        }
        self.waiting = true;
        self.error = None;
        Some(GuildCommand::ChatHistory)
    }

    pub fn submit(&mut self, text: &str, can_speak: bool) -> Option<GuildCommand> {
        if !can_speak || text.trim().is_empty() {
            return None;
        }
        if text.len() > MAX_MESSAGE_BYTES {
            self.error = Some("Guild chat messages must contain at most 512 bytes.".into());
            return None;
        }
        self.error = None;
        Some(GuildCommand::Chat {
            officer: self.officer,
            text: text.into(),
        })
    }

    pub fn apply_history(&mut self, reply: ChatHistoryResponse, guild: bool, officer: bool) {
        if !self.waiting {
            return;
        }
        self.waiting = false;
        self.error = reply.error;
        self.messages = reply
            .messages
            .iter()
            .filter_map(|message| permitted_line(message, guild, officer))
            .collect();
    }

    pub fn receive(&mut self, message: &ChatMessage, guild: bool, officer: bool) {
        if let Some(line) = permitted_line(message, guild, officer) {
            self.messages.push_back(line);
            if self.messages.len() > MAX_MESSAGES {
                self.messages.pop_front();
            }
        }
    }

    pub fn retain_readable(&mut self, guild: bool, officer: bool) {
        self.messages
            .retain(|line| if line.officer { officer } else { guild });
    }
}

fn permitted_line(message: &ChatMessage, guild: bool, officer: bool) -> Option<GuildChatLine> {
    let is_officer = match message.channel {
        ChatType::Guild if guild => false,
        ChatType::Officer if officer => true,
        _ => return None,
    };
    Some(GuildChatLine {
        sender: message.sender.clone(),
        text: message.content.clone(),
        officer: is_officer,
    })
}

pub fn frame(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    use ui_toolkit::art_layout::{BorderLayout, BorderSpec, border_art};
    let mut children = border_art(
        "CommunitiesChatInset",
        BorderSpec::new(BorderLayout::Inset, [191.0, 60.0, 410.0, 338.0]),
    );
    children.extend(stream_picker(session));
    children.extend(transcript(session, shared));
    children.extend(composer(session.chat_can_speak()));
    children
}

fn stream_picker(session: &GuildRanksSession) -> Element {
    let text = if session.chat.officer {
        "Officer Chat"
    } else {
        "Guild Chat"
    };
    let mut children = dropdown_button_with_state(
        DropdownButton {
            frame_name: "CommunitiesStreamDropdown",
            label_name: "CommunitiesStreamDropdownText",
            arrow_name: "CommunitiesStreamDropdownArrow",
            text,
            width: 160.0,
            height: 25.0,
            x: 195.0,
            y: -31.0,
            background_color: "",
            text_color: "1,1,1,1",
            arrow_color: "",
            onclick: Some("guild:chat_stream_menu"),
        },
        DropdownState {
            open: session.chat.menu_open,
            ..Default::default()
        },
    );
    if session.chat.menu_open {
        children.extend(stream_menu(session));
    }
    children
}

fn stream_menu(session: &GuildRanksSession) -> Element {
    let mut items = vec![ContextMenuItem {
        name: "CommunitiesGuildChatChoice",
        label: "Guild Chat",
        action: "guild:chat_guild",
    }];
    if session.chat_can_officer() {
        items.push(ContextMenuItem {
            name: "CommunitiesOfficerChatChoice",
            label: "Officer Chat",
            action: "guild:chat_officer",
        });
    }
    let rows = items
        .iter()
        .enumerate()
        .flat_map(|(index, item)| {
            menu_element(
                *item,
                MenuElementKind::Radio {
                    checked: (index == 1) == session.chat.officer,
                },
                true,
                (8.0, 8.0 + index as f32 * 20.0, 144.0, 20.0),
            )
        })
        .collect();
    context_menu_with_rows(
        ContextMenu {
            frame_name: "CommunitiesStreamMenu",
            title_name: "CommunitiesStreamMenuTitle",
            divider_name: "CommunitiesStreamMenuDivider",
            hidden: false,
            title: "",
            width: 160.0,
            x: 195.0,
            y: 56.0,
            items: &items,
        },
        rows,
    )
}

fn transcript(session: &GuildRanksSession, shared: &SharedContext) -> Element {
    let lines = wrapped_lines(session);
    let (left, top, width, height) = CHAT_RECT;
    let geometry = pixel_geometry(height, lines.len() as f32 * LINE_HEIGHT, height - 26.0);
    let offset = geometry.clamp(shared.scroll_first_row(SCROLL_BOX));
    let rows: Element = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| {
            let bottom = (*index + 1) as f32 * LINE_HEIGHT;
            bottom >= offset as f32 && bottom <= offset as f32 + height + LINE_HEIGHT
        })
        .flat_map(|(index, text)| {
            let is_error = session.chat.error.is_some() && index + 1 == lines.len();
            let name = if is_error {
                "CommunitiesChatError".into()
            } else {
                format!("CommunitiesChatMessage{index}")
            };
            let color = if is_error {
                "1,1,0,1"
            } else if session.chat.officer {
                "0.25,0.75,0.25,1"
            } else {
                "0.25,1,0.25,1"
            };
            rsx! { fontstring { name: {DynName(name)}, text: {text.as_str()},
            font: GameFont::ArialNarrow, font_size: LINE_HEIGHT, font_color: color,
            left: 0.0, top: {index as f32 * LINE_HEIGHT}, width, height: LINE_HEIGHT,
            pos_type: "absolute", justify_h: "LEFT" } }
        })
        .collect();
    let config = scroll_list_attr(&geometry);
    let mut out = rsx! { r#frame { name: {DynName(SCROLL_BOX.into())}, left, top, width, height,
        pos_type: "absolute", mouse_enabled: true, scroll_list: config,
        r#frame { name: "CommunitiesChatMessageChild", left: 0.0, top: {-(offset as f32)}, width,
            height: {lines.len() as f32 * LINE_HEIGHT}, pos_type: "absolute", {rows} }
    } };
    out.extend(
        MinimalScrollBar {
            list: SCROLL_BOX,
            left: left + width + 15.0,
            top: top - 1.0,
            height: height - 26.0,
            geometry,
            offset,
            unscrollable: Unscrollable::HideThumb,
        }
        .element(),
    );
    out
}

fn wrapped_lines(session: &GuildRanksSession) -> Vec<String> {
    use crate::chat_frame::{ChatLine, ChatSpan, parse_received_chat_links, wrap_chat_line};
    let mut lines: Vec<String> = session
        .chat
        .messages
        .iter()
        .filter(|line| line.officer == session.chat.officer)
        .flat_map(|message| {
            let mut spans = vec![ChatSpan::Text(format!("[{}]: ", message.sender))];
            spans.extend(parse_received_chat_links(&message.text));
            let line = ChatLine {
                color: [1.0; 4],
                spans,
            };
            wrap_chat_line(
                &line,
                |_| unreachable!("received spell links carry their own labels"),
                CHAT_RECT.2,
                |text| {
                    ui_toolkit::text_measure::measure_text(text, GameFont::ArialNarrow, LINE_HEIGHT)
                        .expect("Communities ChatFontNormal loaded")
                        .0
                },
            )
            .into_iter()
            .map(|row| row.runs.into_iter().map(|run| run.text).collect::<String>())
        })
        .collect();
    if let Some(error) = &session.chat.error {
        lines.push(error.clone());
    }
    lines
}

fn composer(enabled: bool) -> Element {
    let color = if enabled { "1,1,1,1" } else { "0.5,0.5,0.5,1" };
    rsx! {
        texture { name: "CommunitiesChatEditBoxLeft", texture_fdid: 375507,
            left: 187.0, top: 374.0, width: 32.0, height: 32.0, pos_type: "absolute" }
        texture { name: "CommunitiesChatEditBoxMid", texture_fdid: 389190,
            left: 219.0, top: 374.0, width: 353.0, height: 32.0, pos_type: "absolute", horiz_tile: true }
        texture { name: "CommunitiesChatEditBoxRight", texture_fdid: 375508,
            left: 572.0, top: 374.0, width: 32.0, height: 32.0, pos_type: "absolute" }
        editbox { name: {DynName(EDITOR.into())}, left: 197.0, top: 374.0, width: 397.0, height: 32.0,
            pos_type: "absolute", font: GameFont::ArialNarrow, font_size: LINE_HEIGHT,
            font_color: color, text_insets: "0,0,0,0", mouse_enabled: enabled }
    }
}
