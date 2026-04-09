use super::*;

#[test]
fn dialog_all_requirements_met() {
    let dialog = NPCDialogState {
        required_items: vec![
            RequiredItem {
                name: "A".into(),
                icon_fdid: 1,
                current: 5,
                required: 5,
            },
            RequiredItem {
                name: "B".into(),
                icon_fdid: 2,
                current: 3,
                required: 3,
            },
        ],
        ..Default::default()
    };
    assert!(dialog.all_requirements_met());

    let partial = NPCDialogState {
        required_items: vec![RequiredItem {
            name: "C".into(),
            icon_fdid: 3,
            current: 1,
            required: 5,
        }],
        ..Default::default()
    };
    assert!(!partial.all_requirements_met());
}

#[test]
fn dialog_empty_requirements_met() {
    let dialog = NPCDialogState::default();
    assert!(dialog.all_requirements_met());
}
