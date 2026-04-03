use crate::ui::frame::{Dimension, WidgetData, WidgetType};
use crate::ui::layout::LayoutRect;
use crate::ui::registry::FrameRegistry;
use crate::ui::spellbook_data::{SpellbookSpell, SpellbookTab};
use crate::ui::strata::FrameStrata;
use crate::ui::widgets::font_string::{FontStringData, JustifyH, JustifyV};
use crate::ui::widgets::texture::{TextureData, TextureSource};

pub struct FrameBuilder<'a> {
    pub registry: &'a mut FrameRegistry,
    pub generated_frame_ids: &'a mut Vec<u64>,
    pub next_raise_order: &'a mut i32,
}

pub struct LabelSpec<'a> {
    pub text: &'a str,
    pub color: [f32; 4],
    pub rect: [f32; 4],
    pub font_size: f32,
    pub justify_h: JustifyH,
}

impl<'a> FrameBuilder<'a> {
    pub fn create_panel(
        &mut self,
        name: &str,
        parent_id: u64,
        color: [f32; 4],
        rect: [f32; 4],
    ) -> u64 {
        let id = self.registry.create_frame(name, Some(parent_id));
        let (abs_x, abs_y) = parent_space_to_screen(self.registry, parent_id, rect[0], rect[1]);
        let strata = parent_strata(self.registry, parent_id);
        if let Some(frame) = self.registry.get_mut(id) {
            frame.widget_type = WidgetType::Frame;
            frame.width = Dimension::Fixed(rect[2]);
            frame.height = Dimension::Fixed(rect[3]);
            frame.background_color = Some(color);
            frame.strata = strata;
            frame.raise_order = *self.next_raise_order;
            frame.mouse_enabled = true;
            frame.layout_rect = Some(LayoutRect {
                x: abs_x,
                y: abs_y,
                width: rect[2],
                height: rect[3],
            });
        }
        *self.next_raise_order += 1;
        self.generated_frame_ids.push(id);
        id
    }

    pub fn create_label(&mut self, name: &str, parent_id: u64, spec: LabelSpec<'_>) -> u64 {
        let id = self.registry.create_frame(name, Some(parent_id));
        let (abs_x, abs_y) =
            parent_space_to_screen(self.registry, parent_id, spec.rect[0], spec.rect[1]);
        let strata = parent_strata(self.registry, parent_id);
        if let Some(frame) = self.registry.get_mut(id) {
            frame.widget_type = WidgetType::FontString;
            frame.width = Dimension::Fixed(spec.rect[2]);
            frame.height = Dimension::Fixed(spec.rect[3]);
            frame.strata = strata;
            frame.layout_rect = Some(LayoutRect {
                x: abs_x,
                y: abs_y,
                width: spec.rect[2],
                height: spec.rect[3],
            });
            frame.raise_order = *self.next_raise_order;
            frame.widget_data = Some(WidgetData::FontString(FontStringData {
                text: spec.text.to_string(),
                font_size: spec.font_size,
                color: spec.color,
                justify_h: spec.justify_h,
                justify_v: JustifyV::Middle,
                ..Default::default()
            }));
        }
        *self.next_raise_order += 1;
        self.generated_frame_ids.push(id);
        id
    }

    pub fn create_spell_icon(
        &mut self,
        name: &str,
        parent_id: u64,
        spell: &SpellbookSpell,
        rect: [f32; 4],
    ) -> u64 {
        let id = self.registry.create_frame(name, Some(parent_id));
        let (abs_x, abs_y) = parent_space_to_screen(self.registry, parent_id, rect[0], rect[1]);
        let strata = parent_strata(self.registry, parent_id);
        if let Some(frame) = self.registry.get_mut(id) {
            frame.widget_type = WidgetType::Texture;
            frame.width = Dimension::Fixed(rect[2]);
            frame.height = Dimension::Fixed(rect[3]);
            frame.strata = strata;
            frame.layout_rect = Some(LayoutRect {
                x: abs_x,
                y: abs_y,
                width: rect[2],
                height: rect[3],
            });
            frame.raise_order = *self.next_raise_order;
            if spell.icon_file_data_id > 0 {
                frame.widget_data = Some(WidgetData::Texture(TextureData {
                    source: TextureSource::FileDataId(spell.icon_file_data_id),
                    ..Default::default()
                }));
            } else {
                frame.background_color = Some(icon_fallback_color(spell.passive));
            }
        }
        *self.next_raise_order += 1;
        self.generated_frame_ids.push(id);
        id
    }
}

fn icon_fallback_color(passive: bool) -> [f32; 4] {
    if passive {
        [0.45, 0.40, 0.29, 0.98]
    } else {
        [0.83, 0.67, 0.27, 0.98]
    }
}

pub struct TabRowParams<'a> {
    pub index: usize,
    pub tab: &'a SpellbookTab,
    pub target: crate::ui::spellbook_runtime::HitTarget,
    pub tab_y: f32,
    pub is_active: bool,
    pub is_hover: bool,
    pub is_pressed: bool,
}

pub struct SpellRowExtrasParams<'a> {
    pub index: usize,
    pub spell: &'a SpellbookSpell,
    pub row_y: f32,
    pub target: crate::ui::spellbook_runtime::HitTarget,
    pub cooldown: f32,
}

pub fn create_header_title(builder: &mut FrameBuilder<'_>, root_id: u64) {
    builder.create_label(
        "SpellBookTitle",
        root_id,
        LabelSpec {
            text: "Spellbook",
            color: [0.98, 0.92, 0.74, 1.0],
            rect: [34.0, 16.0, 588.0, 40.0],
            font_size: 28.0,
            justify_h: JustifyH::Left,
        },
    );
    builder.create_label(
        "SpellBookSubtitle",
        root_id,
        LabelSpec {
            text: "Paladin data mirrored from wow-ui-sim",
            color: [0.80, 0.74, 0.58, 0.96],
            rect: [18.0, 60.0, 588.0, 24.0],
            font_size: 16.0,
            justify_h: JustifyH::Left,
        },
    );
}

pub fn create_header_search(builder: &mut FrameBuilder<'_>, root_id: u64, search_text: &str) {
    builder.create_label(
        "SpellBookSearch",
        root_id,
        LabelSpec {
            text: search_text,
            color: [0.78, 0.72, 0.58, 1.0],
            rect: [214.0, 684.0, 368.0, 18.0],
            font_size: 12.0,
            justify_h: JustifyH::Left,
        },
    );
}

pub fn create_header_panels(builder: &mut FrameBuilder<'_>, root_id: u64) {
    builder.create_panel(
        "SpellBookTabs",
        root_id,
        [0.20, 0.16, 0.11, 0.94],
        [20.0, 96.0, 160.0, 600.0],
    );
    builder.create_panel(
        "SpellBookSpells",
        root_id,
        [0.10, 0.08, 0.05, 0.95],
        [196.0, 96.0, 404.0, 600.0],
    );
}

fn create_tab_panel(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    tab_y: f32,
    is_active: bool,
    is_hover: bool,
    is_pressed: bool,
) -> u64 {
    builder.create_panel(
        &format!("SpellBookTabPanel{}", index + 1),
        root_id,
        tab_color(is_active, is_hover, is_pressed),
        [28.0, tab_y, 144.0, 42.0],
    )
}

fn create_tab_name_label(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    tab_name: &str,
    tab_y: f32,
) -> u64 {
    builder.create_label(
        &format!("SpellBookTabLabel{}", index + 1),
        root_id,
        LabelSpec {
            text: tab_name,
            color: [0.95, 0.88, 0.68, 1.0],
            rect: [36.0, tab_y + 9.0, 98.0, 22.0],
            font_size: 14.0,
            justify_h: JustifyH::Left,
        },
    )
}

fn create_tab_count_label(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    count: usize,
    tab_y: f32,
) -> u64 {
    builder.create_label(
        &format!("SpellBookTabCount{}", index + 1),
        root_id,
        LabelSpec {
            text: &count.to_string(),
            color: [0.76, 0.70, 0.56, 1.0],
            rect: [138.0, tab_y + 9.0, 24.0, 22.0],
            font_size: 12.0,
            justify_h: JustifyH::Right,
        },
    )
}

pub fn create_tab_row(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    params: TabRowParams<'_>,
) -> (u64, u64, u64) {
    let panel_id = create_tab_panel(
        builder,
        root_id,
        params.index,
        params.tab_y,
        params.is_active,
        params.is_hover,
        params.is_pressed,
    );
    let name_id = create_tab_name_label(
        builder,
        root_id,
        params.index,
        params.tab.name,
        params.tab_y,
    );
    let count_id = create_tab_count_label(
        builder,
        root_id,
        params.index,
        params.tab.spells.len(),
        params.tab_y,
    );
    (panel_id, name_id, count_id)
}

pub fn create_spell_list_header(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    tab_name: &str,
    page_index: usize,
    total_pages: usize,
) {
    builder.create_label(
        "SpellBookActiveHeader",
        root_id,
        LabelSpec {
            text: &format!("{} Spells", tab_name),
            color: [0.98, 0.90, 0.70, 1.0],
            rect: [216.0, 112.0, 240.0, 26.0],
            font_size: 18.0,
            justify_h: JustifyH::Left,
        },
    );
    builder.create_label(
        "SpellBookPageInfo",
        root_id,
        LabelSpec {
            text: &format!("Page {}/{}", page_index + 1, total_pages),
            color: [0.78, 0.72, 0.58, 1.0],
            rect: [470.0, 114.0, 112.0, 22.0],
            font_size: 12.0,
            justify_h: JustifyH::Right,
        },
    );
}

fn create_spell_row_panel(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    row_y: f32,
    color: [f32; 4],
) -> u64 {
    builder.create_panel(
        &format!("SpellBookSpellRow{}", index + 1),
        root_id,
        color,
        [208.0, row_y, 380.0, 28.0],
    )
}

fn create_spell_name_label(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    spell: &SpellbookSpell,
    row_y: f32,
) -> u64 {
    builder.create_label(
        &format!("SpellBookSpellName{}", index + 1),
        root_id,
        LabelSpec {
            text: spell.name,
            color: [0.96, 0.90, 0.78, 1.0],
            rect: [242.0, row_y + 6.0, 262.0, 20.0],
            font_size: 13.0,
            justify_h: JustifyH::Left,
        },
    )
}

fn create_spell_id_label(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    spell: &SpellbookSpell,
    row_y: f32,
) -> u64 {
    builder.create_label(
        &format!("SpellBookSpellId{}", index + 1),
        root_id,
        LabelSpec {
            text: &spell.id.to_string(),
            color: [0.74, 0.68, 0.54, 1.0],
            rect: [512.0, row_y + 6.0, 70.0, 20.0],
            font_size: 12.0,
            justify_h: JustifyH::Right,
        },
    )
}

pub fn create_spell_row_base(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    spell: &SpellbookSpell,
    row_y: f32,
    color: [f32; 4],
) -> (u64, u64, u64, u64) {
    let row_id = create_spell_row_panel(builder, root_id, index, row_y, color);
    let icon_id = builder.create_spell_icon(
        &format!("SpellBookSpellIcon{}", index + 1),
        root_id,
        spell,
        [214.0, row_y + 4.0, 20.0, 20.0],
    );
    let name_id = create_spell_name_label(builder, root_id, index, spell, row_y);
    let spell_id_id = create_spell_id_label(builder, root_id, index, spell, row_y);
    (row_id, icon_id, name_id, spell_id_id)
}

pub fn create_spell_passive_badge(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    row_y: f32,
) -> u64 {
    builder.create_label(
        &format!("SpellBookSpellPassive{}", index + 1),
        root_id,
        LabelSpec {
            text: "Passive",
            color: [0.68, 0.62, 0.48, 1.0],
            rect: [438.0, row_y + 6.0, 64.0, 20.0],
            font_size: 11.0,
            justify_h: JustifyH::Right,
        },
    )
}

pub fn create_spell_cooldown_frames(
    builder: &mut FrameBuilder<'_>,
    root_id: u64,
    index: usize,
    row_y: f32,
    cooldown: f32,
) -> (u64, u64) {
    let overlay_id = builder.create_panel(
        &format!("SpellBookSpellCooldownOverlay{}", index + 1),
        root_id,
        [0.02, 0.02, 0.02, 0.70],
        [214.0, row_y + 4.0, 20.0, 20.0],
    );
    let text_id = builder.create_label(
        &format!("SpellBookSpellCooldown{}", index + 1),
        root_id,
        LabelSpec {
            text: &format!("{cooldown:.1}"),
            color: [1.0, 0.94, 0.70, 1.0],
            rect: [206.0, row_y + 6.0, 36.0, 16.0],
            font_size: 10.0,
            justify_h: JustifyH::Center,
        },
    );
    (overlay_id, text_id)
}

pub fn tab_color(is_active: bool, is_hover: bool, is_pressed: bool) -> [f32; 4] {
    if is_pressed {
        return [0.43, 0.34, 0.21, 0.98];
    }
    if is_active && is_hover {
        return [0.38, 0.30, 0.19, 0.98];
    }
    if is_active {
        return [0.35, 0.28, 0.18, 0.96];
    }
    if is_hover {
        return [0.29, 0.23, 0.15, 0.92];
    }
    [0.24, 0.19, 0.13, 0.84]
}

pub fn spell_row_color(index: usize, is_hover: bool, is_pressed: bool) -> [f32; 4] {
    if is_pressed {
        return [0.31, 0.22, 0.12, 0.92];
    }
    if is_hover {
        return [0.25, 0.18, 0.11, 0.90];
    }
    if index.is_multiple_of(2) {
        [0.18, 0.14, 0.10, 0.88]
    } else {
        [0.13, 0.10, 0.07, 0.88]
    }
}

fn parent_space_to_screen(
    registry: &FrameRegistry,
    parent_id: u64,
    local_x: f32,
    local_y: f32,
) -> (f32, f32) {
    let Some(parent) = registry.get(parent_id) else {
        return (local_x, local_y);
    };
    let Some(rect) = &parent.layout_rect else {
        return (local_x, local_y);
    };
    (rect.x + local_x, rect.y + local_y)
}

fn parent_strata(registry: &FrameRegistry, parent_id: u64) -> FrameStrata {
    registry
        .get(parent_id)
        .map(|parent| parent.strata)
        .unwrap_or_default()
}
