use std::collections::HashMap;

use ui_toolkit::rsx;
use ui_toolkit::screen::Screen;
use ui_toolkit::widget_def::WidgetChild;

use crate::ui::input::find_frame_at;
use crate::ui::layout::LayoutRect;
use crate::ui::registry::FrameRegistry;
use crate::ui::spellbook_data::{SPELLBOOK_TABS, SpellbookSpell};
use crate::ui::spellbook_frames::{
    FrameBuilder, SpellRowExtrasParams, TabRowParams, create_header_panels,
    create_header_search, create_header_title, create_spell_cooldown_frames,
    create_spell_list_header, create_spell_passive_badge, create_spell_row_base,
    create_tab_row, spell_row_color,
};

const SPELLBOOK_ROOT_NAME: &str = "SpellBookRoot";
const SPELLBOOK_ROOT_SIZE: (f32, f32) = (620.0, 720.0);
const SPELLS_PER_PAGE: usize = 14;
const FALLBACK_COOLDOWN_SECONDS: f32 = 8.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitTarget {
    Tab(usize),
    Spell(u32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellbookAction {
    CastSpell { spell_id: u32, spell_name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellbookKeyInput {
    PreviousTab,
    NextTab,
    PreviousPage,
    NextPage,
    Backspace,
    Clear,
    Character(char),
}

/// Drives a Screen and applies its mutations into the frame registry.
pub struct SpellbookUiRuntime {
    shared_ctx: ui_toolkit::screen::SharedContext,
    screen: Screen,
    spellbook_seeded: bool,
    active_tab_index: usize,
    page_index: usize,
    search_query: String,
    has_keyboard_focus: bool,
    hovered_target: Option<HitTarget>,
    pressed_target: Option<HitTarget>,
    cooldowns: HashMap<u32, f32>,
    generated_frame_ids: Vec<u64>,
    click_targets: HashMap<u64, HitTarget>,
    next_raise_order: i32,
}

impl Default for SpellbookUiRuntime {
    fn default() -> Self {
        Self::new()
    }
}

impl SpellbookUiRuntime {
    pub fn new() -> Self {
        Self {
            shared_ctx: ui_toolkit::screen::SharedContext::new(),
            screen: Screen::new(game_ui_root),
            spellbook_seeded: false,
            active_tab_index: 2,
            page_index: 0,
            search_query: String::new(),
            has_keyboard_focus: false,
            hovered_target: None,
            pressed_target: None,
            cooldowns: HashMap::new(),
            generated_frame_ids: Vec::new(),
            click_targets: HashMap::new(),
            next_raise_order: 1,
        }
    }

    pub fn sync(&mut self, registry: &mut FrameRegistry) {
        self.screen.sync(&self.shared_ctx, registry);

        if !self.spellbook_seeded {
            let _ = self.rebuild_spellbook(registry);
            self.spellbook_seeded = true;
        }
    }

    pub fn has_focus(&self) -> bool {
        self.has_keyboard_focus
    }

    pub fn handle_pointer_move(&mut self, registry: &mut FrameRegistry, x: f32, y: f32) {
        let hovered = self.hit_target_at(registry, x, y);
        if hovered != self.hovered_target {
            self.hovered_target = hovered;
            let _ = self.rebuild_spellbook(registry);
        }
    }

    pub fn handle_pointer_button(
        &mut self,
        registry: &mut FrameRegistry,
        pressed: bool,
        x: f32,
        y: f32,
    ) -> Option<SpellbookAction> {
        let hovered = self.hit_target_at(registry, x, y);
        if hovered != self.hovered_target {
            self.hovered_target = hovered;
        }

        if pressed {
            self.has_keyboard_focus = hovered.is_some();
            if self.pressed_target != hovered {
                self.pressed_target = hovered;
                let _ = self.rebuild_spellbook(registry);
            }
            return None;
        }

        let clicked = self.pressed_target;
        self.pressed_target = None;
        let action = if clicked == hovered {
            self.activate_target(clicked)
        } else {
            None
        };
        let _ = self.rebuild_spellbook(registry);
        action
    }

    pub fn handle_key_input(
        &mut self,
        registry: &mut FrameRegistry,
        key: SpellbookKeyInput,
    ) -> bool {
        if !self.has_keyboard_focus {
            return false;
        }

        let changed = match key {
            SpellbookKeyInput::PreviousTab => self.change_tab(false),
            SpellbookKeyInput::NextTab => self.change_tab(true),
            SpellbookKeyInput::PreviousPage => self.change_page(false),
            SpellbookKeyInput::NextPage => self.change_page(true),
            SpellbookKeyInput::Backspace => {
                if self.search_query.pop().is_some() {
                    self.page_index = 0;
                    true
                } else {
                    false
                }
            }
            SpellbookKeyInput::Clear => {
                let had = !self.search_query.is_empty();
                self.search_query.clear();
                self.page_index = 0;
                had
            }
            SpellbookKeyInput::Character(ch) => {
                if !is_search_character(ch) {
                    false
                } else {
                    self.search_query.push(ch.to_ascii_lowercase());
                    self.page_index = 0;
                    true
                }
            }
        };

        if changed {
            self.hovered_target = None;
            self.pressed_target = None;
            let _ = self.rebuild_spellbook(registry);
        }
        changed
    }

    pub fn advance_cooldowns(&mut self, registry: &mut FrameRegistry, delta_seconds: f32) {
        if self.cooldowns.is_empty() || delta_seconds <= 0.0 {
            return;
        }

        let mut changed = false;
        self.cooldowns.retain(|_, remaining| {
            let next = (*remaining - delta_seconds).max(0.0);
            changed |= (next - *remaining).abs() > f32::EPSILON;
            *remaining = next;
            next > 0.0
        });

        if changed {
            let _ = self.rebuild_spellbook(registry);
        }
    }

    pub fn handle_click(
        &mut self,
        registry: &mut FrameRegistry,
        x: f32,
        y: f32,
    ) -> Option<SpellbookAction> {
        self.handle_pointer_move(registry, x, y);
        let _ = self.handle_pointer_button(registry, true, x, y);
        self.handle_pointer_button(registry, false, x, y)
    }

    fn change_tab(&mut self, forward: bool) -> bool {
        if SPELLBOOK_TABS.is_empty() {
            return false;
        }
        let old = self.active_tab_index;
        if forward {
            self.active_tab_index = (self.active_tab_index + 1) % SPELLBOOK_TABS.len();
        } else {
            self.active_tab_index = if self.active_tab_index == 0 {
                SPELLBOOK_TABS.len() - 1
            } else {
                self.active_tab_index - 1
            };
        }
        if self.active_tab_index != old {
            self.page_index = 0;
            true
        } else {
            false
        }
    }

    fn change_page(&mut self, forward: bool) -> bool {
        let total_pages = self.total_pages_for_active_tab();
        if total_pages <= 1 {
            return false;
        }
        let old = self.page_index;
        if forward {
            self.page_index = (self.page_index + 1) % total_pages;
        } else {
            self.page_index = if self.page_index == 0 {
                total_pages - 1
            } else {
                self.page_index - 1
            };
        }
        self.page_index != old
    }

    fn hit_target_at(&self, registry: &FrameRegistry, x: f32, y: f32) -> Option<HitTarget> {
        let frame_id = find_frame_at(registry, x, y)?;
        self.click_targets.get(&frame_id).copied()
    }

    fn activate_target(&mut self, target: Option<HitTarget>) -> Option<SpellbookAction> {
        match target? {
            HitTarget::Tab(tab_index) => {
                if self.active_tab_index != tab_index {
                    self.active_tab_index = tab_index;
                    self.page_index = 0;
                }
                None
            }
            HitTarget::Spell(spell_id) => {
                let (spell_name, cooldown_seconds, passive) = find_spell(spell_id)
                    .map(|spell| (spell.name, spell.cooldown_seconds, spell.passive))
                    .unwrap_or(("Unknown spell", 0.0, false));
                if !passive {
                    let cooldown = if cooldown_seconds > 0.0 {
                        cooldown_seconds
                    } else {
                        FALLBACK_COOLDOWN_SECONDS
                    };
                    self.cooldowns.insert(spell_id, cooldown);
                }
                Some(SpellbookAction::CastSpell {
                    spell_id,
                    spell_name: spell_name.to_string(),
                })
            }
        }
    }

    fn rebuild_spellbook(&mut self, registry: &mut FrameRegistry) -> Option<u64> {
        let root_id = root_frame_id(registry)?;
        self.clear_generated_frames(registry);
        self.click_targets.clear();
        self.next_raise_order = 1;
        position_root_frame(registry, root_id);
        self.create_header_frames(registry, root_id);
        self.create_tab_frames(registry, root_id);
        self.create_spell_list_frames(registry, root_id);
        Some(root_id)
    }

    fn make_builder<'a>(&'a mut self, registry: &'a mut FrameRegistry) -> FrameBuilder<'a> {
        FrameBuilder {
            registry,
            generated_frame_ids: &mut self.generated_frame_ids,
            next_raise_order: &mut self.next_raise_order,
        }
    }

    fn create_header_frames(&mut self, registry: &mut FrameRegistry, root_id: u64) {
        let search_text = format!(
            "Search: {}",
            if self.search_query.is_empty() {
                "(type to filter)"
            } else {
                &self.search_query
            }
        );
        let mut builder = self.make_builder(registry);
        create_header_title(&mut builder, root_id);
        create_header_panels(&mut builder, root_id);
        create_header_search(&mut builder, root_id, &search_text);
    }

    fn create_tab_frames(&mut self, registry: &mut FrameRegistry, root_id: u64) {
        let mut tab_y = 116.0;
        for (index, tab) in SPELLBOOK_TABS.iter().enumerate() {
            let target = HitTarget::Tab(index);
            let params = TabRowParams {
                index,
                tab,
                target,
                tab_y,
                is_active: index == self.active_tab_index,
                is_hover: self.hovered_target == Some(target),
                is_pressed: self.pressed_target == Some(target),
            };
            let mut builder = self.make_builder(registry);
            let (panel_id, name_id, count_id) = create_tab_row(&mut builder, root_id, params);
            self.click_targets.insert(panel_id, target);
            self.click_targets.insert(name_id, target);
            self.click_targets.insert(count_id, target);
            tab_y += 50.0;
        }
    }

    fn create_spell_list_frames(&mut self, registry: &mut FrameRegistry, root_id: u64) {
        let active_tab = SPELLBOOK_TABS
            .get(self.active_tab_index)
            .unwrap_or(&SPELLBOOK_TABS[0]);
        let filtered = self.filtered_spells(active_tab);
        let total_pages = total_pages(filtered.len());
        if self.page_index >= total_pages {
            self.page_index = total_pages.saturating_sub(1);
        }
        let page_start = self.page_index * SPELLS_PER_PAGE;
        let page_end = (page_start + SPELLS_PER_PAGE).min(filtered.len());
        {
            let page_index = self.page_index;
            let tab_name = active_tab.name;
            let mut builder = self.make_builder(registry);
            create_spell_list_header(&mut builder, root_id, tab_name, page_index, total_pages);
        }
        let mut row_y = 148.0;
        for (index, spell) in filtered[page_start..page_end].iter().enumerate() {
            self.create_spell_row(registry, root_id, index, spell, row_y);
            row_y += 31.0;
        }
    }

    fn create_spell_row(
        &mut self,
        registry: &mut FrameRegistry,
        root_id: u64,
        index: usize,
        spell: &SpellbookSpell,
        row_y: f32,
    ) {
        let target = HitTarget::Spell(spell.id);
        let is_hover = self.hovered_target == Some(target);
        let is_pressed = self.pressed_target == Some(target);
        let cooldown = self.cooldowns.get(&spell.id).copied().unwrap_or(0.0);
        let color = spell_row_color(index, is_hover, is_pressed);
        let mut builder = self.make_builder(registry);
        let (row_id, icon_id, name_id, spell_id_id) =
            create_spell_row_base(&mut builder, root_id, index, spell, row_y, color);
        drop(builder);
        self.click_targets.insert(row_id, target);
        self.click_targets.insert(icon_id, target);
        self.click_targets.insert(name_id, target);
        self.click_targets.insert(spell_id_id, target);
        let extras = SpellRowExtrasParams {
            index,
            spell,
            row_y,
            target,
            cooldown,
        };
        self.create_spell_row_extras(registry, root_id, extras);
    }

    fn create_spell_row_extras(
        &mut self,
        registry: &mut FrameRegistry,
        root_id: u64,
        params: SpellRowExtrasParams<'_>,
    ) {
        let target = params.target;
        if params.spell.passive {
            let mut builder = self.make_builder(registry);
            let badge_id =
                create_spell_passive_badge(&mut builder, root_id, params.index, params.row_y);
            drop(builder);
            self.click_targets.insert(badge_id, target);
        }
        if params.cooldown > 0.0 {
            let mut builder = self.make_builder(registry);
            let (overlay_id, text_id) = create_spell_cooldown_frames(
                &mut builder,
                root_id,
                params.index,
                params.row_y,
                params.cooldown,
            );
            drop(builder);
            self.click_targets.insert(overlay_id, target);
            self.click_targets.insert(text_id, target);
        }
    }

    fn filtered_spells<'a>(
        &self,
        tab: &'a crate::ui::spellbook_data::SpellbookTab,
    ) -> Vec<&'a SpellbookSpell> {
        if self.search_query.is_empty() {
            return tab.spells.iter().collect();
        }
        tab.spells
            .iter()
            .filter(|spell| spell.name.to_ascii_lowercase().contains(&self.search_query))
            .collect()
    }

    fn total_pages_for_active_tab(&self) -> usize {
        let active_tab = SPELLBOOK_TABS
            .get(self.active_tab_index)
            .unwrap_or(&SPELLBOOK_TABS[0]);
        total_pages(self.filtered_spells(active_tab).len())
    }

    fn clear_generated_frames(&mut self, registry: &mut FrameRegistry) {
        for frame_id in self.generated_frame_ids.drain(..).rev() {
            registry.remove_frame(frame_id);
        }
    }
}

fn game_ui_root(_ctx: &ui_toolkit::screen::SharedContext) -> Vec<WidgetChild> {
    rsx! {
        frame {
            name: "SpellBookRoot",
            width: 620.0,
            height: 720.0,
            background_color: "0.16,0.12,0.08,0.96",
            strata: "DIALOG",
        }
    }
}

fn position_root_frame(registry: &mut FrameRegistry, root_id: u64) {
    if let Some(root) = registry.get_mut(root_id) {
        root.layout_rect = Some(LayoutRect {
            x: 80.0,
            y: 120.0,
            width: SPELLBOOK_ROOT_SIZE.0,
            height: SPELLBOOK_ROOT_SIZE.1,
        });
    }
}

fn total_pages(total_spells: usize) -> usize {
    if total_spells == 0 {
        1
    } else {
        total_spells.div_ceil(SPELLS_PER_PAGE)
    }
}

fn is_search_character(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, ' ' | '-' | '\'')
}

fn find_spell(spell_id: u32) -> Option<&'static SpellbookSpell> {
    for tab in SPELLBOOK_TABS {
        if let Some(spell) = tab.spells.iter().find(|spell| spell.id == spell_id) {
            return Some(spell);
        }
    }
    None
}

fn root_frame_id(registry: &FrameRegistry) -> Option<u64> {
    if let Some(id) = registry.get_by_name(SPELLBOOK_ROOT_NAME) {
        return Some(id);
    }
    registry
        .frames_iter()
        .find(|frame| {
            frame.parent_id.is_none()
                && (frame.width.value() - SPELLBOOK_ROOT_SIZE.0).abs() < f32::EPSILON
                && (frame.height.value() - SPELLBOOK_ROOT_SIZE.1).abs() < f32::EPSILON
        })
        .map(|frame| frame.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::frame::{Dimension, WidgetData};
    use crate::ui::strata::FrameStrata;

    #[test]
    fn sync_builds_frames_from_virtual_dom() {
        let mut runtime = SpellbookUiRuntime::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        runtime.sync(&mut registry);
        let root_id = root_frame_id(&registry).expect("spellbook root frame exists");
        let root = registry.get(root_id).expect("spellbook root is present");
        assert_eq!(root.width, Dimension::Fixed(620.0));
        assert_eq!(root.height, Dimension::Fixed(720.0));
        assert_eq!(root.strata, FrameStrata::Dialog);
        assert_eq!(root.background_color, Some([0.16, 0.12, 0.08, 0.96]));
        let sample_spell_id = registry
            .get_by_name("SpellBookSpellName1")
            .expect("spell row label exists");
        let sample_spell =
            get_fontstring_text(&registry, sample_spell_id).expect("spell row is a text label");
        assert_eq!(sample_spell, "Avenger's Shield");
    }

    #[test]
    fn clicking_tab_rebuilds_active_spell_list() {
        let mut runtime = SpellbookUiRuntime::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        runtime.sync(&mut registry);
        let holy_tab = registry
            .get_by_name("SpellBookTabPanel4")
            .expect("holy tab panel exists");
        let rect = registry
            .get(holy_tab)
            .and_then(|f| f.layout_rect.clone())
            .expect("holy tab rect exists");
        let _ = runtime.handle_click(
            &mut registry,
            rect.x + rect.width * 0.5,
            rect.y + rect.height * 0.5,
        );
        let sample_spell_id = registry
            .get_by_name("SpellBookSpellName1")
            .expect("spell row label exists");
        let sample_spell =
            get_fontstring_text(&registry, sample_spell_id).expect("spell row is a text label");
        assert_eq!(sample_spell, "Holy Shock");
    }

    #[test]
    fn clicking_spell_returns_cast_action() {
        let mut runtime = SpellbookUiRuntime::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        runtime.sync(&mut registry);
        let spell_name = registry
            .get_by_name("SpellBookSpellName1")
            .expect("first spell name exists");
        let rect = registry
            .get(spell_name)
            .and_then(|f| f.layout_rect.clone())
            .expect("first spell rect exists");
        let action = runtime.handle_click(
            &mut registry,
            rect.x + rect.width * 0.5,
            rect.y + rect.height * 0.5,
        );
        assert_eq!(
            action,
            Some(SpellbookAction::CastSpell {
                spell_id: 31935,
                spell_name: "Avenger's Shield".to_string(),
            })
        );
    }

    #[test]
    fn keyboard_search_filters_spell_rows() {
        let mut runtime = SpellbookUiRuntime::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        runtime.sync(&mut registry);
        runtime.has_keyboard_focus = true;
        let _ = runtime.handle_key_input(&mut registry, SpellbookKeyInput::Character('e'));
        let _ = runtime.handle_key_input(&mut registry, SpellbookKeyInput::Character('y'));
        let _ = runtime.handle_key_input(&mut registry, SpellbookKeyInput::Character('e'));
        let sample_spell_id = registry
            .get_by_name("SpellBookSpellName1")
            .expect("spell row label exists");
        let sample_spell =
            get_fontstring_text(&registry, sample_spell_id).expect("spell row is a text label");
        assert_eq!(sample_spell, "Eye of Tyr");
    }

    #[test]
    fn casting_spell_adds_cooldown_overlay() {
        let (mut runtime, mut registry) = setup_with_cast();
        let cooldown_id = registry
            .get_by_name("SpellBookSpellCooldown1")
            .expect("cooldown text should be present");
        let initial_text =
            get_fontstring_text(&registry, cooldown_id).expect("cooldown should be fontstring");
        assert_eq!(initial_text, "15.0");
        runtime.advance_cooldowns(&mut registry, 1.0);
        let cooldown_id = registry
            .get_by_name("SpellBookSpellCooldown1")
            .expect("cooldown text should still be present");
        let next_text =
            get_fontstring_text(&registry, cooldown_id).expect("cooldown should be fontstring");
        assert_eq!(next_text, "14.0");
    }

    fn setup_with_cast() -> (SpellbookUiRuntime, FrameRegistry) {
        let mut runtime = SpellbookUiRuntime::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        runtime.sync(&mut registry);
        let spell_name = registry
            .get_by_name("SpellBookSpellName1")
            .expect("first spell name exists");
        let rect = registry
            .get(spell_name)
            .and_then(|f| f.layout_rect.clone())
            .expect("first spell rect exists");
        let _ = runtime.handle_click(
            &mut registry,
            rect.x + rect.width * 0.5,
            rect.y + rect.height * 0.5,
        );
        (runtime, registry)
    }

    fn get_fontstring_text(registry: &FrameRegistry, id: u64) -> Option<&str> {
        registry
            .get(id)
            .and_then(|f| f.widget_data.as_ref())
            .and_then(|data| match data {
                WidgetData::FontString(fs) => Some(fs.text.as_str()),
                _ => None,
            })
    }
}
