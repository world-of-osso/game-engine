use super::OptionType;

/// Demon Hunter-only option labels on the Night Elf and Blood Elf models.
pub(super) fn option_visible_for_class(race: u8, class: u8, opt_type: OptionType) -> bool {
    match opt_type {
        OptionType::Horns | OptionType::Blindfold | OptionType::EyeStyle | OptionType::Eyesight => {
            !matches!(race, 4 | 10) || class == 12
        }
        _ => true,
    }
}

/// TrinityCore `RaceMask::GetRaceBit`: playable race ID -> ChrCustomizationReq.RaceMask bit.
pub(super) fn race_mask_bit(race: u8) -> Option<u32> {
    match race {
        1..=11 | 22 | 24..=32 => Some(u32::from(race) - 1),
        34 => Some(11),
        35 => Some(12),
        36 => Some(13),
        37 => Some(14),
        52 => Some(16),
        70 => Some(15),
        84 => Some(17),
        85 => Some(18),
        86 => Some(20),
        91 => Some(19),
        _ => None,
    }
}
