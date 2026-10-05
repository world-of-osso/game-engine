use game_engine_ui_model::character_frame::{
    CharacterFrameView, CharacterTab, REPUTATION_DESCRIPTION_SCROLL, REPUTATION_SCROLL,
    character_frame_screen, reputation_pan_extent, reputation_rows,
};
use shared::protocol_snapshots::ReputationEntrySnapshot;
use ui_toolkit::atlas::{ActiveSkin, set_active_skin};
use ui_toolkit::screen::{Screen, SharedContext};

use super::tests::{rebuild, rect};
use super::*;
use crate::ui::ui_parent::UiParent;
use crate::ui::{RegistryModel, ScreenPostsetup};

fn show_reputation(view: CharacterFrameView) -> RegistryModel {
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    let mut shared = SharedContext::new();
    shared.insert(view);
    let mut model = RegistryModel {
        screen: Screen::new(character_frame_screen),
        shared,
        registry: UiParent::for_viewport(1920.0, 1080.0).registry(),
        icon_masks: Default::default(),
        postsetup: ScreenPostsetup::None,
    };
    rebuild(&mut model);
    model
}

#[test]
fn character_reputation_scroll_lists_wheel_arrows_and_thumb_reveal_the_last_faction() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let entries: Vec<_> = (0..40)
            .map(|index| ReputationEntrySnapshot {
                faction_id: 100 + index,
                faction_name: format!("Faction {index}"),
                standing: "Friendly".into(),
                value: 24_000,
            })
            .collect();
        let mut model = show_reputation(CharacterFrameView {
            visible: true,
            tab: CharacterTab::Reputation,
            reputation: reputation_rows(&entries),
            ..Default::default()
        });
        let initial = rect(&model, "ReputationEntry1").unwrap().y;
        assert!(!wheel(&mut model, REPUTATION_SCROLL, true));
        assert!(wheel(&mut model, REPUTATION_SCROLL, false));
        rebuild(&mut model);
        let first = rect(&model, "ReputationEntry3").unwrap().y;
        assert!((first - initial).abs() < 0.01);
        let forward = model
            .registry
            .get_by_name(&forward_stepper_name(REPUTATION_SCROLL))
            .unwrap();
        assert!(press_stepper(&mut model, forward));
        assert_eq!(
            model
                .registry
                .scroll_lists
                .get(REPUTATION_SCROLL)
                .unwrap()
                .first_row,
            3 * reputation_pan_extent()
        );
        rebuild(&mut model);
        let thumb = model
            .registry
            .get_by_name(&thumb_name(REPUTATION_SCROLL))
            .unwrap();
        let top = rect(&model, &thumb_name(REPUTATION_SCROLL)).unwrap().y;
        assert!(press_thumb(&mut model.registry, thumb, top));
        let track = rect(&model, &track_name(REPUTATION_SCROLL)).unwrap();
        assert!(drag_thumbs(&mut model.registry, track.y + track.height));
        assert!(release_thumbs(&mut model.registry));
        rebuild(&mut model);
        assert!(!wheel(&mut model, REPUTATION_SCROLL, false));
        let area = rect(&model, REPUTATION_SCROLL).unwrap();
        let last = rect(&model, "ReputationEntry40").unwrap();
        assert!(last.y >= area.y && last.y + last.height <= area.y + area.height);
        assert!(model.registry.get_by_name("ReputationEntry1").is_none());
        let back = model
            .registry
            .get_by_name(&back_stepper_name(REPUTATION_SCROLL))
            .unwrap();
        let before = last.y;
        assert!(press_stepper(&mut model, back));
        rebuild(&mut model);
        let after = rect(&model, "ReputationEntry40").unwrap().y;
        assert!((after - before - reputation_pan_extent() as f32).abs() < 0.01);
    }
    set_active_skin(ActiveSkin::Modern);
}

#[test]
fn character_reputation_scroll_lists_long_description_reaches_its_final_line() {
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_active_skin(skin);
        let mut rows = reputation_rows(&[ReputationEntrySnapshot {
            faction_id: 72,
            faction_name: "Stormwind".into(),
            standing: "Friendly".into(),
            value: 24_000,
        }]);
        rows[0].description = "One of the last bastions of human power, this Alliance capital is ruled by the young but wise king, Anduin Wrynn. ".repeat(8);
        let mut model = show_reputation(CharacterFrameView {
            visible: true,
            tab: CharacterTab::Reputation,
            reputation: rows,
            selected_reputation: Some(72),
            ..Default::default()
        });
        let area = rect(&model, REPUTATION_DESCRIPTION_SCROLL).unwrap();
        let text = rect(&model, "ReputationDetailFrameDescription").unwrap();
        assert!(
            text.y + text.height > area.y + area.height,
            "full text extends beyond the viewport, not a one-line ellipsis"
        );
        let mut notches = 0;
        while wheel(&mut model, REPUTATION_DESCRIPTION_SCROLL, false) {
            rebuild(&mut model);
            notches += 1;
            assert!(notches < 200);
        }
        assert!(notches > 0);
        let text = rect(&model, "ReputationDetailFrameDescription").unwrap();
        assert!(text.y < area.y);
        assert!(
            (text.y + text.height - area.y - area.height).abs() <= 1.0,
            "last line lies at the bottom of the viewport"
        );
    }
    set_active_skin(ActiveSkin::Modern);
}
