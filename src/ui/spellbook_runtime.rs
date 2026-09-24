use std::collections::HashMap;

use bevy::math::Vec2;
use ui_toolkit::rsx;
use ui_toolkit::screen::Screen;
use ui_toolkit::widget_def::WidgetChild;

use crate::ui::input::find_frame_at;
use crate::ui::registry::FrameRegistry;
use crate::ui::spellbook_data::{SpellbookSpell, SpellbookTab};
use crate::ui::spellbook_frames::{
    FrameBuilder, TabRowParams, create_header_panels, create_header_search, create_header_title,
    create_spell_list_header, create_spell_passive_badge, create_spell_row_base, create_tab_row,
    spell_row_color,
};

const SPELLBOOK_ROOT_NAME: &str = "SpellBookRoot";
const SPELLBOOK_ROOT_SIZE: (f32, f32) = (620.0, 720.0);
const SPELLS_PER_PAGE: usize = 14;
/// The class tab: most useful default once spells arrive.
const DEFAULT_TAB_INDEX: usize = 1;
/// Cursor travel (UI px) before a pressed spell becomes a drag.
const DRAG_THRESHOLD: f32 = 4.0;

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
    Character(char),
}

/// Drives a Screen and applies its mutations into the frame registry.
pub struct SpellbookUiRuntime {
    #[cfg(test)]
    pub(crate) execution_counts: (usize, usize),
    shared_ctx: ui_toolkit::screen::SharedContext,
    screen: Screen,
    spellbook_seeded: bool,
    open: bool,
    tabs: Vec<SpellbookTab>,
    active_tab_index: usize,
    page_index: usize,
    search_query: String,
    has_keyboard_focus: bool,
    hovered_target: Option<HitTarget>,
    pressed_target: Option<HitTarget>,
    press_position: Option<Vec2>,
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
            #[cfg(test)]
            execution_counts: (0, 0),
            shared_ctx: ui_toolkit::screen::SharedContext::new(),
            screen: Screen::new(game_ui_root),
            spellbook_seeded: false,
            open: false,
            tabs: Vec::new(),
            active_tab_index: DEFAULT_TAB_INDEX,
            page_index: 0,
            search_query: String::new(),
            has_keyboard_focus: false,
            hovered_target: None,
            pressed_target: None,
            press_position: None,
            generated_frame_ids: Vec::new(),
            click_targets: HashMap::new(),
            next_raise_order: 1,
        }
    }

    pub fn sync(&mut self, registry: &mut FrameRegistry) {
        #[cfg(test)]
        {
            self.execution_counts.0 += 1;
        }
        self.screen.sync(&self.shared_ctx, registry);

        if !self.spellbook_seeded {
            let _ = self.rebuild_spellbook(registry);
            self.spellbook_seeded = true;
        }
    }

    /// Removes every spellbook frame; the next `sync` builds them again.
    pub fn teardown(&mut self, registry: &mut FrameRegistry) {
        self.clear_generated_frames(registry);
        self.click_targets.clear();
        self.screen.teardown(registry);
        self.spellbook_seeded = false;
        self.clear_pointer_state();
    }

    pub fn is_open(&self) -> bool {
        self.open
    }

    /// Shows or hides the spellbook. Returns whether visibility changed.
    pub fn set_open(&mut self, registry: &mut FrameRegistry, open: bool) -> bool {
        if self.open == open {
            return false;
        }
        self.open = open;
        if !open {
            self.has_keyboard_focus = false;
            self.clear_pointer_state();
        }
        if let Some(root_id) = root_frame_id(registry) {
            registry.set_hidden(root_id, !open);
        }
        true
    }

    /// Replaces the listed spells. Returns whether the displayed tabs changed.
    pub fn set_tabs(&mut self, registry: &mut FrameRegistry, tabs: Vec<SpellbookTab>) -> bool {
        if self.tabs == tabs {
            return false;
        }
        self.tabs = tabs;
        if self.spellbook_seeded {
            let _ = self.rebuild_spellbook(registry);
        }
        true
    }

    pub fn has_focus(&self) -> bool {
        self.has_keyboard_focus
    }

    pub fn clear_focus(&mut self) {
        self.has_keyboard_focus = false;
    }

    /// Returns whether hover changed the displayed spellbook.
    pub fn handle_pointer_move(&mut self, registry: &mut FrameRegistry, x: f32, y: f32) -> bool {
        #[cfg(test)]
        {
            self.execution_counts.1 += 1;
        }
        let hovered = self.hit_target_at(registry, x, y);
        if hovered != self.hovered_target {
            self.hovered_target = hovered;
            return self.rebuild_spellbook(registry).is_some();
        }
        false
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
            self.press_position = hovered.map(|_| Vec2::new(x, y));
            if self.pressed_target != hovered {
                self.pressed_target = hovered;
                let _ = self.rebuild_spellbook(registry);
            }
            return None;
        }

        let clicked = self.pressed_target;
        self.pressed_target = None;
        self.press_position = None;
        let action = if clicked == hovered {
            self.activate_target(clicked)
        } else {
            None
        };
        let _ = self.rebuild_spellbook(registry);
        action
    }

    /// While the button is held: the pressed non-passive spell once the cursor
    /// has moved past the drag threshold. The press is consumed so its release
    /// does not cast.
    pub fn take_drag_start(&mut self, registry: &mut FrameRegistry, x: f32, y: f32) -> Option<u32> {
        let Some(HitTarget::Spell(spell_id)) = self.pressed_target else {
            return None;
        };
        let start = self.press_position?;
        if start.distance(Vec2::new(x, y)) < DRAG_THRESHOLD {
            return None;
        }
        self.pressed_target = None;
        self.press_position = None;
        let _ = self.rebuild_spellbook(registry);
        self.find_spell(spell_id)
            .filter(|spell| !spell.passive)
            .map(|spell| spell.id)
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

    /// Spell id shown by `frame_id` or one of its ancestors.
    pub fn spell_for_frame(&self, registry: &FrameRegistry, frame_id: u64) -> Option<u32> {
        if !self.open {
            return None;
        }
        match self.target_for_frame(registry, frame_id)? {
            HitTarget::Spell(spell_id) => Some(spell_id),
            HitTarget::Tab(_) => None,
        }
    }

    fn clear_pointer_state(&mut self) {
        self.hovered_target = None;
        self.pressed_target = None;
        self.press_position = None;
    }

    fn change_tab(&mut self, forward: bool) -> bool {
        let count = self.tabs.len();
        if count == 0 {
            return false;
        }
        let old = self.active_tab_index.min(count - 1);
        self.active_tab_index = if forward {
            (old + 1) % count
        } else {
            (old + count - 1) % count
        };
        if self.active_tab_index != old {
            self.page_index = 0;
            true
        } else {
            false
        }
    }

    fn change_page(&mut self, forward: bool) -> bool {
        let total_pages = total_pages(self.filtered_spells().len());
        if total_pages <= 1 {
            return false;
        }
        let old = self.page_index;
        self.page_index = if forward {
            (old + 1) % total_pages
        } else {
            (old + total_pages - 1) % total_pages
        };
        self.page_index != old
    }

    fn hit_target_at(&self, registry: &FrameRegistry, x: f32, y: f32) -> Option<HitTarget> {
        if !self.open {
            return None;
        }
        let frame_id = find_frame_at(registry, x, y)?;
        self.target_for_frame(registry, frame_id)
    }

    fn target_for_frame(&self, registry: &FrameRegistry, mut frame_id: u64) -> Option<HitTarget> {
        loop {
            if let Some(target) = self.click_targets.get(&frame_id).copied() {
                return Some(target);
            }
            let frame = registry.get(frame_id)?;
            frame_id = frame.parent_id?;
        }
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
                let spell = self.find_spell(spell_id).filter(|spell| !spell.passive)?;
                Some(SpellbookAction::CastSpell {
                    spell_id,
                    spell_name: spell.name.clone(),
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
        registry.set_hidden(root_id, !self.open);
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
        let known: usize = self.tabs.iter().map(|tab| tab.spells.len()).sum();
        let subtitle = format!("{known} known spells");
        let mut builder = self.make_builder(registry);
        create_header_title(&mut builder, root_id, &subtitle);
        create_header_panels(&mut builder, root_id);
        create_header_search(&mut builder, root_id, &search_text);
    }

    fn create_tab_frames(&mut self, registry: &mut FrameRegistry, root_id: u64) {
        let active = self.active_tab();
        let tabs = std::mem::take(&mut self.tabs);
        let mut tab_y = 116.0;
        for (index, tab) in tabs.iter().enumerate() {
            let target = HitTarget::Tab(index);
            let params = TabRowParams {
                index,
                tab,
                target,
                tab_y,
                is_active: index == active,
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
        self.tabs = tabs;
    }

    fn create_spell_list_frames(&mut self, registry: &mut FrameRegistry, root_id: u64) {
        let filtered: Vec<SpellbookSpell> = self.filtered_spells().into_iter().cloned().collect();
        let total_pages = total_pages(filtered.len());
        if self.page_index >= total_pages {
            self.page_index = total_pages.saturating_sub(1);
        }
        let page_start = self.page_index * SPELLS_PER_PAGE;
        let page_end = (page_start + SPELLS_PER_PAGE).min(filtered.len());
        {
            let page_index = self.page_index;
            let tab_name = self
                .tabs
                .get(self.active_tab())
                .map(|tab| tab.name.clone())
                .unwrap_or_default();
            let mut builder = self.make_builder(registry);
            create_spell_list_header(&mut builder, root_id, &tab_name, page_index, total_pages);
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
        let color = spell_row_color(index, is_hover, is_pressed);
        let mut builder = self.make_builder(registry);
        let (row_id, icon_id, name_id, subtext_id) =
            create_spell_row_base(&mut builder, root_id, index, spell, row_y, color);
        let badge_id = spell
            .passive
            .then(|| create_spell_passive_badge(&mut builder, root_id, index, row_y));
        for id in [row_id, icon_id, name_id, subtext_id]
            .into_iter()
            .chain(badge_id)
        {
            self.click_targets.insert(id, target);
        }
    }

    fn active_tab(&self) -> usize {
        self.active_tab_index.min(self.tabs.len().saturating_sub(1))
    }

    fn filtered_spells(&self) -> Vec<&SpellbookSpell> {
        let Some(tab) = self.tabs.get(self.active_tab()) else {
            return Vec::new();
        };
        tab.spells
            .iter()
            .filter(|spell| {
                self.search_query.is_empty()
                    || spell.name.to_ascii_lowercase().contains(&self.search_query)
            })
            .collect()
    }

    fn find_spell(&self, spell_id: u32) -> Option<&SpellbookSpell> {
        self.tabs
            .iter()
            .flat_map(|tab| &tab.spells)
            .find(|spell| spell.id == spell_id)
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
            hidden: true,
        }
    }
}

fn position_root_frame(registry: &mut FrameRegistry, root_id: u64) {
    if let Some(root) = registry.get_mut(root_id) {
        root.position_type = bevy::ui::PositionType::Absolute;
        root.position.left = bevy::ui::Val::Px(80.0);
        root.position.top = bevy::ui::Val::Px(120.0);
        root.width = crate::ui::frame::Dimension::Fixed(SPELLBOOK_ROOT_SIZE.0);
        root.height = crate::ui::frame::Dimension::Fixed(SPELLBOOK_ROOT_SIZE.1);
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

fn root_frame_id(registry: &FrameRegistry) -> Option<u64> {
    registry.get_by_name(SPELLBOOK_ROOT_NAME)
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;
    use crate::ui::layout::LayoutRect;

    pub(crate) fn spell(id: u32, name: &str, passive: bool) -> SpellbookSpell {
        SpellbookSpell {
            id,
            name: name.to_string(),
            subtext: String::new(),
            passive,
            icon_file_data_id: id + 1,
        }
    }

    pub(crate) fn paladin_tabs() -> Vec<SpellbookTab> {
        let tab = |name: &str, spells: Vec<SpellbookSpell>| SpellbookTab {
            name: name.to_string(),
            spells,
        };
        vec![
            tab("General", vec![spell(6603, "Auto Attack", false)]),
            tab(
                "Paladin",
                vec![
                    spell(35395, "Crusader Strike", false),
                    spell(20271, "Judgment", false),
                    spell(853, "Hammer of Justice", false),
                ],
            ),
            tab(
                "Protection",
                vec![
                    spell(31935, "Avenger's Shield", false),
                    spell(387174, "Eye of Tyr", false),
                    spell(76671, "Mastery: Divine Bulwark", true),
                ],
            ),
        ]
    }

    /// Layout readback for the spellbook's absolutely positioned frames.
    pub(crate) fn simulate_layout_readback(registry: &mut FrameRegistry) {
        simulate_layout_readback_at(registry, Vec2::new(80.0, 120.0));
    }

    /// Readback with the root laid out at `origin` instead of its authored position.
    pub(crate) fn simulate_layout_readback_at(registry: &mut FrameRegistry, origin: Vec2) {
        let Some(root_id) = root_frame_id(registry) else {
            return;
        };
        let (root_x, root_y) = (origin.x, origin.y);
        let root = LayoutRect {
            x: root_x,
            y: root_y,
            width: SPELLBOOK_ROOT_SIZE.0,
            height: SPELLBOOK_ROOT_SIZE.1,
        };
        registry.set_computed_layout(root_id, root).unwrap();
        let children: Vec<(u64, LayoutRect)> = registry
            .frames_iter()
            .filter(|frame| frame.parent_id == Some(root_id))
            .filter_map(|frame| {
                let (bevy::ui::Val::Px(left), bevy::ui::Val::Px(top)) =
                    (frame.position.left, frame.position.top)
                else {
                    return None;
                };
                Some((
                    frame.id,
                    LayoutRect {
                        x: root_x + left,
                        y: root_y + top,
                        width: frame.width.value(),
                        height: frame.height.value(),
                    },
                ))
            })
            .collect();
        for (id, rect) in children {
            registry.set_computed_layout(id, rect).unwrap();
        }
    }

    /// An open, laid-out spellbook listing [`paladin_tabs`].
    pub(crate) fn open_runtime(registry: &mut FrameRegistry) -> SpellbookUiRuntime {
        let mut runtime = SpellbookUiRuntime::new();
        runtime.set_tabs(registry, paladin_tabs());
        runtime.sync(registry);
        runtime.set_open(registry, true);
        simulate_layout_readback(registry);
        runtime
    }

    pub(crate) fn center_of(registry: &FrameRegistry, name: &str) -> (f32, f32) {
        let id = registry.get_by_name(name).expect(name);
        let rect = registry.get(id).unwrap().layout_rect.clone().expect(name);
        (rect.x + rect.width * 0.5, rect.y + rect.height * 0.5)
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;
    use crate::ui::frame::{Dimension, WidgetData};
    use crate::ui::strata::FrameStrata;

    /// Hover, press and release on separate frames, each followed by layout
    /// readback, as the layout pass does between real input frames.
    fn relayout_click(
        runtime: &mut SpellbookUiRuntime,
        registry: &mut FrameRegistry,
        name: &str,
    ) -> Option<SpellbookAction> {
        let (x, y) = center_of(registry, name);
        runtime.handle_pointer_move(registry, x, y);
        simulate_layout_readback(registry);
        let _ = runtime.handle_pointer_button(registry, true, x, y);
        simulate_layout_readback(registry);
        let action = runtime.handle_pointer_button(registry, false, x, y);
        simulate_layout_readback(registry);
        action
    }

    #[test]
    fn sync_builds_frames_from_virtual_dom() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        open_runtime(&mut registry);
        let root_id = root_frame_id(&registry).expect("spellbook root frame exists");
        let root = registry.get(root_id).expect("spellbook root is present");
        assert_eq!(root.width, Dimension::Fixed(620.0));
        assert_eq!(root.height, Dimension::Fixed(720.0));
        assert_eq!(root.strata, FrameStrata::Dialog);
        assert_eq!(root.background_color, Some([0.16, 0.12, 0.08, 0.96]));
        assert!(root.visible);
        assert_eq!(first_spell(&registry), "Crusader Strike");
        assert!(registry.get_by_name("SpellBookSpell3").is_some());
    }

    #[test]
    fn closed_spellbook_is_hidden_and_ignores_clicks() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut runtime = open_runtime(&mut registry);
        let (x, y) = center_of(&registry, "SpellBookSpellName1");
        runtime.set_open(&mut registry, false);
        let root = registry.get(root_frame_id(&registry).unwrap()).unwrap();
        assert!(!root.visible);
        assert_eq!(runtime.handle_click(&mut registry, x, y), None);
    }

    #[test]
    fn clicking_tab_rebuilds_active_spell_list() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut runtime = open_runtime(&mut registry);
        let _ = relayout_click(&mut runtime, &mut registry, "SpellBookTabPanel3");
        assert_eq!(first_spell(&registry), "Avenger's Shield");
    }

    #[test]
    fn clicking_spell_returns_cast_action() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut runtime = open_runtime(&mut registry);
        let action = relayout_click(&mut runtime, &mut registry, "SpellBookSpellName1");
        assert_eq!(
            action,
            Some(SpellbookAction::CastSpell {
                spell_id: 35395,
                spell_name: "Crusader Strike".to_string(),
            })
        );
    }

    #[test]
    fn clicking_passive_spell_does_not_cast() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut runtime = open_runtime(&mut registry);
        let _ = relayout_click(&mut runtime, &mut registry, "SpellBookTabPanel3");
        assert_eq!(
            relayout_click(&mut runtime, &mut registry, "SpellBookSpellName3"),
            None
        );
    }

    #[test]
    fn pressed_spell_becomes_drag_after_threshold_and_release_does_not_cast() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut runtime = open_runtime(&mut registry);
        let (x, y) = center_of(&registry, "SpellBookSpellName2");
        let _ = runtime.handle_pointer_button(&mut registry, true, x, y);
        simulate_layout_readback(&mut registry);
        assert_eq!(runtime.take_drag_start(&mut registry, x + 1.0, y), None);
        assert_eq!(
            runtime.take_drag_start(&mut registry, x + 30.0, y),
            Some(20271)
        );
        simulate_layout_readback(&mut registry);
        assert_eq!(
            runtime.handle_pointer_button(&mut registry, false, x, y),
            None
        );
    }

    #[test]
    fn keyboard_search_filters_spell_rows() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut runtime = open_runtime(&mut registry);
        runtime.has_keyboard_focus = true;
        runtime.active_tab_index = 2;
        for ch in ['e', 'y', 'e'] {
            let _ = runtime.handle_key_input(&mut registry, SpellbookKeyInput::Character(ch));
        }
        assert_eq!(first_spell(&registry), "Eye of Tyr");
    }

    #[test]
    fn spell_for_frame_returns_hovered_spell() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let runtime = open_runtime(&mut registry);
        let frame_id = registry
            .get_by_name("SpellBookSpellName1")
            .expect("spell row label exists");
        assert_eq!(runtime.spell_for_frame(&registry, frame_id), Some(35395));
    }

    #[test]
    fn new_tabs_rebuild_rows_and_tab_counts() {
        let mut registry = FrameRegistry::new(1920.0, 1080.0);
        let mut runtime = open_runtime(&mut registry);
        let mut tabs = paladin_tabs();
        tabs[1].spells.remove(0);
        assert!(runtime.set_tabs(&mut registry, tabs.clone()));
        assert!(!runtime.set_tabs(&mut registry, tabs));
        assert_eq!(first_spell(&registry), "Judgment");
        assert_eq!(text(&registry, "SpellBookTabCount2"), "2");
    }

    fn first_spell(registry: &FrameRegistry) -> String {
        text(registry, "SpellBookSpellName1")
    }

    fn text(registry: &FrameRegistry, name: &str) -> String {
        let id = registry.get_by_name(name).expect(name);
        match registry.get(id).and_then(|f| f.widget_data.as_ref()) {
            Some(WidgetData::FontString(fs)) => fs.text.clone(),
            _ => panic!("{name} is not a font string"),
        }
    }
}
