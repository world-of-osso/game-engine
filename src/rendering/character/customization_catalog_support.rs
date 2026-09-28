use super::{CustomizationChoice, OptionType};

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
            !matches!(race, 4 | 10) || class == 12
        }
        _ => true,
    }
}
