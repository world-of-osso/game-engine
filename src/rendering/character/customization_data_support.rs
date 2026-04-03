use crate::asset::{asset_cache::texture, blp::load_blp_rgba};

use super::{CustomizationChoice, OptionType};

pub(super) fn sample_swatch_color(materials: &[(u16, u32)]) -> Option<[u8; 3]> {
    let &(_, fdid) = materials.first()?;
    let path = texture(fdid)?;
    let (rgba, w, h) = load_blp_rgba(&path).ok()?;
    let cx = w / 2;
    let cy = h / 2;
    let idx = ((cy * w + cx) * 4) as usize;
    (idx + 2 < rgba.len()).then_some([rgba[idx], rgba[idx + 1], rgba[idx + 2]])
}

pub(super) fn choice_visible_for_class(
    race: u8,
    class: u8,
    opt_type: OptionType,
    choice: &CustomizationChoice,
) -> bool {
    option_visible_for_class(race, class, opt_type)
        && match (opt_type, race, class, choice.requirement_id) {
            (OptionType::Face, 4 | 10, 12, 146) => true,
            (OptionType::Face, 4 | 10, 12, 142 | 144) => false,
            (OptionType::Face, 4 | 10, _, 142) => true,
            (OptionType::Face, 4 | 10, _, 144 | 146) => false,
            _ => true,
        }
}

fn option_visible_for_class(race: u8, class: u8, opt_type: OptionType) -> bool {
    match opt_type {
        OptionType::Horns | OptionType::Blindfold | OptionType::EyeStyle | OptionType::Eyesight => {
            matches!(race, 4 | 10) && class == 12
        }
        _ => true,
    }
}
