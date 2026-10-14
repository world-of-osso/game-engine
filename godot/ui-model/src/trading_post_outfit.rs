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
            .selected_item()
            .ok_or("Trading Post preview selection missing")?;
        let scene = if row.model_scene_id != 0 {
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

pub(super) fn footer(view: &View) -> Element {
    if !view
        .selected_item()
        .is_some_and(|row| matches!(row.category_id, 1 | 8))
    {
        return vec![];
    }
    let checked = match view.hide_armor() {
        Ok(checked) => checked,
        Err(error) => {
            return label(
                "PerksProgramOutfitError".into(),
                &error,
                (WIDTH / 2., 0., 400., 29.),
                (14., "1,0.1,0.1,1", "LEFT"),
            );
        }
    };
    // RotateButtonContainer is BOTTOM(+140,+42), width100 height48.
    // HideArmor BOTTOM to container.TOP+10; LEFT to container.LEFT-18.
    let left = WIDTH / 2. + 140. - 50. - 18.;
    let top = 100. - 42. - 48. - 10. - 29.;
    let mut children = named_atlas_texture(
        "PerksProgramToggleHideArmorBackground".into(),
        "checkbox-minimal",
        (0., 0., 30., 29.),
    );
    if checked {
        children.extend(named_atlas_texture(
            "PerksProgramToggleHideArmorChecked".into(),
            "checkmark-minimal",
            (0., 0., 30., 29.),
        ));
    }
    let (width, height) = ui_toolkit::text_measure::measure_text(
        "Hide Armor",
        ui_toolkit::widgets::font_string::GameFont::FrizQuadrata,
        16.,
    )
    .expect("SystemFont_Large metrics");
    children.extend(label(
        "PerksProgramToggleHideArmorText".into(),
        "Hide Armor",
        (35., (29. - height) / 2., width, height),
        (16., WHITE, "LEFT"),
    ));
    rsx! {button{name:"PerksProgramToggleHideArmor",left,top,width:30.0,height:29.0,pos_type:"absolute",button_default_skin:false,onclick:"perks:hide-armor",{children}}}
}
