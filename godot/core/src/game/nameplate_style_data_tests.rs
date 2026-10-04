use super::*;
use NameplateBarThickness::{Thick, Thin};

#[test]
fn presets_keep_independent_heights_and_choose_nearest_at_midpoint() {
    let mut style = NameplateStyle::default();
    assert_eq!((style.health_height, style.cast_height), (20.0, 15.0));
    assert_eq!((style.health_preset(), style.cast_preset()), (Thick, Thick));
    assert_eq!(
        NameplateStyle::from_presets(Thin, Thick).health_height,
        10.0
    );
    assert_eq!(NameplateStyle::from_presets(Thin, Thick).cast_height, 15.0);
    style.health_width = 150.0;
    style.apply_health_preset(Thin);
    assert_eq!((style.health_width, style.health_height), (150.0, 10.0));
    style.health_height = 14.0;
    assert_eq!(style.health_preset(), Thin);
    style.health_height = 15.0;
    assert_eq!(style.health_preset(), Thick);
    style.cast_height = 10.0;
    assert_eq!(style.cast_preset(), Thin);
    style.cast_height = 11.0;
    assert_eq!(style.cast_preset(), Thick);
    assert_eq!(Thin.toggled(), Thick);
    assert_eq!(Thick.toggled(), Thin);
}

#[test]
fn sliders_preserve_keys_ranges_rounding_and_color_channels() {
    let mut style = NameplateStyle::default();
    let green = StyleSlider::Channel(StyleColor::Neutral, 1);
    assert_eq!(green.key(), "nameplate_neutral_g");
    assert_eq!(StyleSlider::from_key("nameplate_neutral_g"), Some(green));
    assert_eq!(StyleSlider::from_key("nameplate_neutral_a"), None);
    assert_eq!(
        StyleSlider::from_key("nameplate_health_width"),
        Some(StyleSlider::HealthWidth)
    );
    green.set(&mut style, 0.35);
    assert_eq!(style.health_colors.neutral, [1.0, 0.35, 0.0]);
    green.set(&mut style, 2.0);
    assert_eq!(green.get(&style), 1.0);
    StyleSlider::HealthWidth.set(&mut style, 150.4);
    assert_eq!(style.health_width, 150.0);
    StyleSlider::CastHeight.set(&mut style, 99.0);
    assert_eq!(StyleSlider::CastHeight.get(&style), 20.0);
    assert_eq!(StyleColor::Channel.rgb(&style), [0.0, 1.0, 0.0]);
    assert_eq!(StyleSlider::NameFontSize.bounds(), (6.0, 24.0));
}

#[test]
fn all_editable_colors_and_sizes_have_stable_keys() {
    assert_eq!(StyleSlider::SIZES.len(), 6);
    assert_eq!(StyleColor::ALL.len(), 6);
    for slider in StyleSlider::SIZES {
        assert_eq!(StyleSlider::from_key(&slider.key()), Some(slider));
    }
    for color in StyleColor::ALL {
        for channel in 0..3 {
            let slider = StyleSlider::Channel(color, channel);
            assert_eq!(StyleSlider::from_key(&slider.key()), Some(slider));
        }
    }
}
