//! Mythic Mentor's compact party key list (BossHelper UI/GroupFinderPanel.lua).
use crate::bank_art::{UNKNOWN_ICON, WHITE, cropped, label, texture};
use crate::quest_art::{DynName, flat_panel_chrome};
use shared::protocol::{PartyKeystoneMember, PartyKeystoneSnapshot};
use std::collections::BTreeMap;
use ui_toolkit::strata::FrameStrata;
use ui_toolkit::{rsx, screen::SharedContext, widget_def::Element};

pub const OPEN_ACTION: &str = "mentor:toggle";
pub const CLOSE_ACTION: &str = "mentor:close";
pub const TELEPORT_ACTION_PREFIX: &str = "mentor:teleport:";
/// All eight mapped spells: SpellCategories.Category=1407, local build12.1.0.69933.
/// SpellCooldowns has RecoveryTime0 / CategoryRecoveryTime28800000 for each.
pub const TELEPORT_CATEGORY: u32 = 1407;
pub const FRAME: &str = "MythicMentorPanel";
pub const PANEL_WIDTH: f32 = 176.0;
#[derive(Clone, Debug, PartialEq)]
pub struct MentorView {
    pub position: [f32; 2],
    pub snapshot: PartyKeystoneSnapshot,
    /// Spell-ID keyed text from the existing server-fed cooldown state.
    pub cooldowns: BTreeMap<u32, String>,
}

/// Installed MythicTeleport.lua / enUS.lua's unlearned presentation.
pub fn mentor_teleport_tooltip(
    mut tooltip: crate::game_tooltip::GameTooltip,
    learned: bool,
) -> crate::game_tooltip::GameTooltip {
    if !learned {
        tooltip.content.title_color = [0.6, 0.6, 0.6, 1.0];
        tooltip
            .content
            .lines
            .push(crate::tooltip_presentation::TooltipLineState::colored(
                "Spell not learned.",
                [0.53, 0.53, 0.53, 1.0],
            ));
    }
    tooltip
}
pub fn apply_mentor_postsetup(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, widgets::texture::TextureSource};
    for index in 0..5 {
        let Some(id) = registry.get_by_name(&format!("MentorRow{index}Teleport")) else {
            break;
        };
        let frame = registry.get_mut(id).unwrap();
        let mapped = frame
            .onclick
            .as_deref()
            .is_some_and(|action| action.starts_with(TELEPORT_ACTION_PREFIX));
        let Some(WidgetData::Button(button)) = frame.widget_data.as_mut() else {
            panic!("Mentor teleport frame is not a button")
        };
        button.highlight_texture =
            mapped.then_some(ui_toolkit::widgets::button::ButtonTexture::new(
                TextureSource::SolidColor([0.0, 0.0, 0.0, 0.5]),
            ));
    }
}

pub fn panel_height(members: usize) -> f32 {
    // Addon: header22 + padding8 + n*36 + (n-1)*4.
    30.0 + members as f32 * 40.0 - 4.0
}
pub fn mentor_screen(ctx: &SharedContext) -> Element {
    let _ = ctx.get::<ui_toolkit::atlas::ActiveSkin>();
    let view = ctx.get::<MentorView>().expect("MentorView");
    if view.snapshot.members.is_empty() {
        return Element::new();
    }
    let height = panel_height(view.snapshot.members.len());
    let rows: Element = view
        .snapshot
        .members
        .iter()
        .enumerate()
        .flat_map(|(index, member)| keystone_row(index, member, &view.cooldowns))
        .collect();
    let [left, top] = view.position;
    rsx! {
        r#frame { name: {DynName(FRAME.into())}, width: PANEL_WIDTH, height,
            strata: FrameStrata::Dialog, mouse_enabled: true,
            pos_type: "absolute", left, top,
            {flat_panel_chrome("MythicMentor", (PANEL_WIDTH, height), "Keys", CLOSE_ACTION)}
            button { name: "MentorDragHandle", width: {PANEL_WIDTH - 30.0}, height: 22.0,
                button_default_skin: false, onclick: "mentor:drag",
                pos_type: "absolute", left: 0.0, top: 0.0 }
            {rows}
        }
    }
}

/// Current installed MythicTeleport.lua:17-26. No inferred teleport for Azure401.
pub fn teleport_spell(challenge_id: u32) -> Option<u32> {
    match challenge_id {
        587 => Some(1286809),
        588 => Some(1286812),
        586 => Some(1286807),
        584 => Some(1286801),
        399 => Some(393256),
        250 => Some(1286828),
        249 => Some(1286831),
        585 => Some(1286804),
        _ => None,
    }
}
fn keystone_row(
    index: usize,
    member: &PartyKeystoneMember,
    cooldowns: &BTreeMap<u32, String>,
) -> Element {
    let prefix = format!("MentorRow{index}");
    let (dungeon, level, timer) = match &member.key {
        Some(key) => (
            key.dungeon_name.clone(),
            format!("+{}", key.level),
            crate::challenge::clock(key.time_limit_ms),
        ),
        None => ("No key".into(), String::new(), String::new()),
    };
    let [r, g, b] = crate::damage_meter_data::class_color(member.class);
    let color = format!("{r},{g},{b},1.0");
    let mut children = row_icon(&prefix, member);
    let cooldown = member
        .key
        .as_ref()
        .and_then(|key| teleport_spell(key.challenge_id))
        .and_then(|spell| cooldowns.get(&spell))
        .map(String::as_str)
        .unwrap_or_default();
    children.extend(icon_overlay(&prefix, &level, member.is_leader, cooldown));
    children.extend(label(
        format!("{prefix}Dungeon"),
        &dungeon,
        (37.0, 1.0, 118.0, 17.0),
        (10.0, "0.65,0.65,0.85,1.0", "LEFT"),
    ));
    children.extend(label(
        format!("{prefix}Player"),
        &member.name,
        (37.0, 18.0, 78.0, 17.0),
        (10.0, &color, "LEFT"),
    ));
    children.extend(label(
        format!("{prefix}Timer"),
        &timer,
        (117.0, 18.0, 38.0, 17.0),
        (9.0, WHITE, "RIGHT"),
    ));
    rsx! {
        r#frame { name: {DynName(prefix)}, width: 160.0, height: 36.0,
            background_color: "0.06,0.07,0.11,0.95", border: "1px solid 0.3,0.3,0.3,1.0",
            pos_type: "absolute", left: 8.0, top: {22.0 + index as f32 * 40.0}, {children} }
    }
}
fn icon_overlay(prefix: &str, level: &str, leader: bool, cooldown: &str) -> Element {
    // KeystoneWidget.lua's topLayer is above the icon button and its hover layer.
    let mut children = label(
        format!("{prefix}Level"),
        level,
        (5.0, 19.0, 26.0, 14.0),
        (11.0, "1.0,0.82,0.0,1.0", "RIGHT"),
    );
    children.extend(label(
        format!("{prefix}Cooldown"),
        cooldown,
        (5.0, 5.0, 26.0, 12.0),
        (8.0, WHITE, "LEFT"),
    ));
    if leader {
        children.extend(texture(
            format!("{prefix}Leader"),
            132062,
            (11.0, -3.0, 14.0, 14.0),
            WHITE,
        ));
    }
    rsx! { r#frame { name: {DynName(format!("{prefix}TopLayer"))}, width: 36.0, height: 36.0,
    frame_level: 20.0, mouse_enabled: false, pos_type: "absolute", left: 0.0, top: 0.0, {children} } }
}
/// MapChallengeMode.MapID → JournalInstance.MapID / ButtonSmallFileDataID,
/// local adventure_guide.sqlite, build12.1.0.69933. These are square LFG icons.
fn dungeon_icon(challenge_id: u32) -> u32 {
    match challenge_id {
        249 => 2178730,
        250 => 2178734,
        399 => 4746639,
        401 => 4746634,
        584 => 7478534,
        585 => 7479112,
        586 => 7478536,
        587 => 7467179,
        588 => 7956176,
        // KeystoneWidget.lua uses the question mark when dungeon texture is unknown.
        _ => UNKNOWN_ICON,
    }
}
fn row_icon(prefix: &str, member: &PartyKeystoneMember) -> Element {
    let art = match member.key.as_ref().map(|key| key.challenge_id) {
        Some(challenge_id) => cropped(
            format!("{prefix}Icon"),
            dungeon_icon(challenge_id),
            "0.07,0.93,0.07,0.93",
            (0.0, 0.0, 26.0, 26.0),
        ),
        None => texture(
            format!("{prefix}Icon"),
            UNKNOWN_ICON,
            (0.0, 0.0, 26.0, 26.0),
            "0.5,0.5,0.5,1.0",
        ),
    };
    let action = member
        .key
        .as_ref()
        .and_then(|key| teleport_spell(key.challenge_id))
        .map(|spell| format!("{TELEPORT_ACTION_PREFIX}{spell}"))
        .unwrap_or_default();
    rsx! { button { name: {DynName(format!("{prefix}Teleport"))}, width: 26.0, height: 26.0,
    button_default_skin: false, onclick: {action.as_str()},
    pos_type: "absolute", left: 5.0, top: 5.0, {art} } }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shared::protocol::{PartyKeystone, PartyKeystoneMember};
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        frame::WidgetData,
        registry::FrameRegistry,
        screen::Screen,
    };

    fn text(registry: &FrameRegistry, name: &str) -> String {
        let frame = registry
            .get(registry.get_by_name(name).expect(name))
            .unwrap();
        let Some(WidgetData::FontString(font)) = &frame.widget_data else {
            panic!("not a label: {name}")
        };
        font.text.clone()
    }
    #[test]
    fn mythic_mentor_unlearned_teleport_tooltip_preserves_real_spell_text() {
        use crate::{
            game_tooltip::GameTooltip,
            tooltip_presentation::{TooltipLineState, TooltipPresentation},
        };
        let tooltip = GameTooltip::new(
            TooltipPresentation {
                title: "Path of the Clutch Defender".into(),
                lines: vec![TooltipLineState::new("8 hour cooldown")],
                ..Default::default()
            },
            None,
        );
        let unlearned = mentor_teleport_tooltip(tooltip.clone(), false);
        assert_eq!(unlearned.content.title, "Path of the Clutch Defender");
        assert_eq!(unlearned.content.lines[0].left_text, "8 hour cooldown");
        assert_eq!(unlearned.content.lines[1].left_text, "Spell not learned.");
        assert_eq!(unlearned.content.title_color, [0.6, 0.6, 0.6, 1.0]);
        assert_eq!(mentor_teleport_tooltip(tooltip.clone(), true), tooltip);
    }

    #[test]
    fn mythic_mentor_cooldown_countdown_clears_when_ready_and_key_replaced() {
        crate::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut view = MentorView {
                position: [510.0, 100.0],
                cooldowns: BTreeMap::from([(393256, "8h".into())]),
                snapshot: PartyKeystoneSnapshot {
                    members: vec![PartyKeystoneMember {
                        character_id: 17,
                        name: "Mentorone".into(),
                        class: 2,
                        is_leader: true,
                        key: Some(PartyKeystone {
                            challenge_id: 399,
                            level: 8,
                            dungeon_name: "Ruby Life Pools".into(),
                            time_limit_ms: 1_680_000,
                        }),
                    }],
                },
            };
            let mut shared = SharedContext::new();
            shared.insert(skin);
            shared.insert(view.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            let mut screen = Screen::new(mentor_screen);
            screen.sync(&shared, &mut registry);
            assert_eq!(text(&registry, "MentorRow0Cooldown"), "8h");
            view.cooldowns.clear();
            shared.insert(view.clone());
            screen.sync(&shared, &mut registry);
            assert_eq!(text(&registry, "MentorRow0Cooldown"), "");
            view.cooldowns.insert(393256, "8h".into());
            view.snapshot.members[0].key = None;
            shared.insert(view);
            screen.sync(&shared, &mut registry);
            assert_eq!(text(&registry, "MentorRow0Cooldown"), "");
        }
    }

    #[test]
    fn ui_template_behavior_services_mythic_mentor_current_dungeon_icons_teleport_actions_and_owner()
     {
        use ui_toolkit::widgets::texture::TextureSource;
        crate::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut shared = SharedContext::new();
            shared.insert(skin);
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            let mut screen = Screen::new(mentor_screen);
            for (challenge_id, icon, spell, name, time_limit_ms) in [
                (249, 2178730, Some(1286831), "Kings' Rest", 1_980_000),
                (
                    250,
                    2178734,
                    Some(1286828),
                    "Temple of Sethraliss",
                    1_920_000,
                ),
                (399, 4746639, Some(393256), "Ruby Life Pools", 1_680_000),
                (401, 4746634, None, "The Azure Vault", 2_250_000),
                (584, 7478534, Some(1286801), "The Blinding Vale", 1_800_000),
                (585, 7479112, Some(1286804), "Voidscar Arena", 1_800_000),
                (586, 7478536, Some(1286807), "Den of Nalorakk", 1_920_000),
                (587, 7467179, Some(1286809), "Murder Row", 2_040_000),
                (588, 7956176, Some(1286812), "Altar of Fangs", 1_800_000),
            ] {
                shared.insert(MentorView {
                    position: [510.0, 100.0],
                    cooldowns: BTreeMap::new(),
                    snapshot: PartyKeystoneSnapshot {
                        members: vec![PartyKeystoneMember {
                            character_id: 17,
                            name: "Mentorone".into(),
                            class: 2,
                            is_leader: true,
                            key: Some(PartyKeystone {
                                challenge_id,
                                level: 8,
                                dungeon_name: name.into(),
                                time_limit_ms,
                            }),
                        }],
                    },
                });
                screen.sync(&shared, &mut registry);
                let frame = registry
                    .get(
                        registry
                            .get_by_name("MentorRow0Icon")
                            .expect("Sourced dungeon icon"),
                    )
                    .unwrap();
                let Some(WidgetData::Texture(art)) = &frame.widget_data else {
                    panic!("Dungeon icon must render artwork")
                };
                assert_eq!(art.source, TextureSource::FileDataId(icon));
                let button = registry
                    .get(registry.get_by_name("MentorRow0Teleport").unwrap())
                    .unwrap();
                let action = spell
                    .map(|id| format!("mentor:teleport:{id}"))
                    .unwrap_or_default();
                assert_eq!(button.onclick.as_deref().unwrap_or_default(), action);
                let (owner, binding) = ui_toolkit::tooltip::tooltip_owner(&registry, button.id)
                    .expect("mentor live teleport owner");
                assert_eq!(owner.id, button.id);
                assert_eq!(binding.provider.as_deref(), Some("mentor_teleport"));
                assert_eq!(binding.anchor, ui_toolkit::tooltip::TooltipAnchor::Right);
                apply_mentor_postsetup(&mut registry);
                let button = registry
                    .get(registry.get_by_name("MentorRow0Teleport").unwrap())
                    .unwrap();
                let Some(WidgetData::Button(button)) = &button.widget_data else {
                    panic!("Keystone click target must be a button")
                };
                let highlight = spell.map(|_| TextureSource::SolidColor([0.0, 0.0, 0.0, 0.5]));
                assert_eq!(
                    button
                        .highlight_texture
                        .as_ref()
                        .map(|texture| texture.source.clone()),
                    highlight
                );
            }
        }
    }

    #[test]
    fn mythic_mentor_both_skins_display_replace_and_prune_authoritative_rows() {
        crate::paths::set_data_root(
            std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
        )
        .unwrap();
        for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
            set_thread_skin(skin);
            let mut view = MentorView {
                position: [510.0, 100.0],
                cooldowns: BTreeMap::new(),
                snapshot: PartyKeystoneSnapshot {
                    members: vec![
                        PartyKeystoneMember {
                            character_id: 17,
                            name: "Mentorone".into(),
                            class: 2,
                            is_leader: true,
                            key: Some(PartyKeystone {
                                challenge_id: 401,
                                level: 8,
                                dungeon_name: "The Azure Vault".into(),
                                time_limit_ms: 2250000,
                            }),
                        },
                        PartyKeystoneMember {
                            character_id: 18,
                            name: "Mentortwo".into(),
                            class: 8,
                            is_leader: false,
                            key: None,
                        },
                    ],
                },
            };
            let mut shared = SharedContext::new();
            shared.insert(skin);
            shared.insert(view.clone());
            let mut registry = FrameRegistry::new(1280.0, 720.0);
            let mut screen = Screen::new(mentor_screen);
            screen.sync(&shared, &mut registry);
            assert!(
                registry.get_by_name(FRAME).is_some(),
                "non-raid rows must show the native Keys panel"
            );
            assert_eq!(text(&registry, "MentorRow0Dungeon"), "The Azure Vault");
            assert_eq!(text(&registry, "MentorRow0Level"), "+8");
            assert_eq!(text(&registry, "MentorRow0Timer"), "37:30");
            assert_eq!(text(&registry, "MentorRow1Dungeon"), "No key");
            assert_eq!(text(&registry, "MentorRow1Level"), "");
            assert!(registry.get_by_name("MentorRow0Leader").is_some());
            view.snapshot.members[0].key.as_mut().unwrap().level = 11;
            view.snapshot.members[0].is_leader = false;
            view.snapshot.members.pop();
            shared.insert(view.clone());
            screen.sync(&shared, &mut registry);
            assert_eq!(text(&registry, "MentorRow0Level"), "+11");
            assert!(registry.get_by_name("MentorRow1Dungeon").is_none());
            assert!(registry.get_by_name("MentorRow0Leader").is_none());
            view.snapshot.members.clear();
            shared.insert(view);
            screen.sync(&shared, &mut registry);
            assert!(
                registry.get_by_name(FRAME).is_none(),
                "raid/empty replacement hides the panel"
            );
        }
    }
}
