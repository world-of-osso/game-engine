//! PerksProgramProducts.TranslateDisplayInfo and Footer.UpdateTransmogControls.
use super::*;
use std::{collections::BTreeMap, sync::OnceLock};

const SCENE_CSV: &str = "db2/12.1.5.70391/UiModelScene.csv";
static FLAGS: OnceLock<Result<BTreeMap<u32, u32>, String>> = OnceLock::new();

fn load_scene_flags() -> Result<BTreeMap<u32, u32>, String> {
    let path = crate::paths::resolve_data_path(SCENE_CSV);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("Trading Post scene flags {}: {error}", path.display()))?;
    let records = game_engine_core::csv_util::parse_csv_records(&text);
    let header = records
        .first()
        .ok_or("Trading Post scene flags CSV empty")?;
    let id = header
        .iter()
        .position(|value| value == "ID")
        .ok_or("UiModelScene missing ID")?;
    let flags = header
        .iter()
        .position(|value| value == "Flags")
        .ok_or("UiModelScene missing Flags")?;
    records
        .iter()
        .skip(1)
        .map(|row| {
            let parse = |column: usize| -> Result<u32, String> {
                row.get(column)
                    .ok_or("UiModelScene truncated row")?
                    .parse()
                    .map_err(|error| format!("UiModelScene invalid number: {error}"))
            };
            Ok((parse(id)?, parse(flags)?))
        })
        .collect()
}

/// Installed 70391 UIModelSceneFlags.Autodress = 4. No category-derived defaults.
pub fn load_scene_autodress(scene: u32) -> Result<bool, String> {
    let flags = FLAGS
        .get_or_init(load_scene_flags)
        .as_ref()
        .map_err(Clone::clone)?;
    flags
        .get(&scene)
        .map(|flags| flags & 4 != 0)
        .ok_or_else(|| format!("Trading Post UiModelScene {scene} unavailable"))
}

impl View {
    pub fn hide_armor(&self) -> Result<bool, String> {
        if let Some(setting) = self.hide_armor_setting {
            return Ok(setting);
        }
        let row = self
            .preview_product()
            .ok_or("Trading Post preview selection missing")?;
        let scene = if self.cart_visible && !self.cart_selected.contains_key(&0) {
            self.snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.categories.iter().find(|category| category.id == 1))
                .map(|category| category.model_scene_id)
                .ok_or("Trading Post transmog scene missing")?
        } else if row.model_scene_id != 0 {
            row.model_scene_id
        } else {
            self.snapshot
                .as_ref()
                .and_then(|snapshot| {
                    snapshot
                        .categories
                        .iter()
                        .find(|category| category.id == row.category_id)
                })
                .map(|category| category.model_scene_id)
                .ok_or("Trading Post category scene missing")?
        };
        load_scene_autodress(scene).map(|autodress| !autodress)
    }
}

pub(super) fn rotation_controls(view: &View) -> Element {
    if !view
        .preview_product()
        .is_some_and(|row| row.category_id != 5 && row.grant_unavailable.is_none())
    {
        return vec![];
    }
    // PerksProgram.xml: BOTTOM(+140,+42), 100x48 in the 100px footer.
    let left = WIDTH / 2. + 140. - 50.;
    let mut children = rotation_button("Left", 0., "perks-icon-rotate-left", "perks:rotate-left");
    children.extend(rotation_button(
        "Right",
        52.,
        "perks-icon-rotate-right",
        "perks:rotate-right",
    ));
    rsx! {frame{name:"PerksProgramRotateButtonContainer",left,top:10.0,width:100.0,height:48.0,pos_type:"absolute",{children}}}
}

fn rotation_button(direction: &str, left: f32, atlas: &str, action: &str) -> Element {
    let name = DynName(format!("PerksProgramRotate{direction}Button"));
    let icon = named_atlas_texture(format!("{}Icon", name.0), atlas, (11., 11.5, 26., 25.));
    rsx! {button{name,left,top:0.0,width:48.0,height:48.0,pos_type:"absolute",button_default_skin:false,
    button_atlas_up:"perks-button-up",button_atlas_pressed:"perks-button-down",
    button_atlas_highlight:"perks-button-up",button_highlight_alpha:0.6,onclick:action,{icon}}}
}

/// Source mouse-down/up moves the icon; registry layout, not native Control state, owns its position.
pub fn apply_rotation_icon_positions(registry: &mut ui_toolkit::registry::FrameRegistry) {
    use ui_toolkit::{frame::WidgetData, layout_values::Val, widgets::button::ButtonState};
    for name in [
        "PerksProgramRotateLeftButton",
        "PerksProgramRotateRightButton",
    ] {
        let Some(button) = registry.get_by_name(name) else {
            continue;
        };
        let pushed = matches!(registry.get(button).and_then(|frame| frame.widget_data.as_ref()), Some(WidgetData::Button(button)) if button.state == ButtonState::Pushed);
        let icon = registry
            .get_by_name(&format!("{name}Icon"))
            .expect("Perks rotation icon");
        let offset = if pushed { 1. } else { 0. };
        let left = Val::Px(11. + offset);
        let top = Val::Px(11.5 + offset);
        let frame = registry.get(icon).expect("Perks rotation icon frame");
        if frame.position.left == left && frame.position.top == top {
            continue;
        }
        let frame = registry.get_mut(icon).expect("Perks rotation icon frame");
        frame.position.left = left;
        frame.position.top = top;
        registry.mark_rect_dirty(icon);
    }
}

pub(super) fn footer(view: &View) -> Element {
    let Some(row) = view.preview_product() else {
        return vec![];
    };
    if row.category_id == 2 {
        return preview_checkbox(
            "PerksProgramTogglePlayerPreview",
            "Hide Character",
            view.hide_player_preview,
            "perks:hide-character",
        );
    }
    if !matches!(row.category_id, 1 | 8) {
        return vec![];
    }
    match view.hide_armor() {
        Ok(checked) => preview_checkbox(
            "PerksProgramToggleHideArmor",
            "Hide Armor",
            checked,
            "perks:hide-armor",
        ),
        Err(error) => label(
            "PerksProgramOutfitError".into(),
            &error,
            (WIDTH / 2., 0., 400., 29.),
            (14., "1,0.1,0.1,1", "LEFT"),
        ),
    }
}

fn preview_checkbox(name: &str, text: &str, checked: bool, action: &str) -> Element {
    // Both source checkboxes: BOTTOM to RotateButtonContainer.TOP+10, LEFT to LEFT-18.
    let left = WIDTH / 2. + 140. - 50. - 18.;
    let top = 100. - 42. - 48. - 10. - 29.;
    let mut children = named_atlas_texture(
        format!("{name}Background"),
        "checkbox-minimal",
        (0., 0., 30., 29.),
    );
    if checked {
        children.extend(named_atlas_texture(
            format!("{name}Checked"),
            "checkmark-minimal",
            (0., 0., 30., 29.),
        ));
    }
    children.extend(preview_checkbox_label(name, text));
    let name = DynName(name.to_owned());
    rsx! {button{name,left,top,width:30.0,height:29.0,pos_type:"absolute",button_default_skin:false,onclick:action,{children}}}
}

fn preview_checkbox_label(name: &str, text: &str) -> Element {
    let (width, height) = ui_toolkit::text_measure::measure_text(
        text,
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        16.,
    )
    .expect("SystemFont_Large metrics");
    label(
        format!("{name}Text"),
        text,
        (35., (29. - height) / 2., width, height),
        (16., WHITE, "LEFT"),
    )
}
