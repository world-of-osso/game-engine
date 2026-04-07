use super::*;
use ui_toolkit::layout::{LayoutRect, recompute_layouts};
use ui_toolkit::registry::FrameRegistry;
use ui_toolkit::screen::{Screen, SharedContext};

fn make_test_state() -> GuildControlState {
    GuildControlState {
        visible: true,
        ranks: vec![
            GuildRank {
                name: "Guild Master".into(),
                selected: true,
            },
            GuildRank {
                name: "Officer".into(),
                selected: false,
            },
            GuildRank {
                name: "Member".into(),
                selected: false,
            },
        ],
        rank_name: "Guild Master".into(),
        permissions: vec![
            PermissionRow {
                label: "Invite Members".into(),
                checked: true,
            },
            PermissionRow {
                label: "Remove Members".into(),
                checked: true,
            },
            PermissionRow {
                label: "Promote Members".into(),
                checked: true,
            },
            PermissionRow {
                label: "Edit Public Note".into(),
                checked: false,
            },
        ],
        bank_tab_permissions: vec![],
    }
}

fn build_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_test_state());
    Screen::new(guild_control_screen).sync(&shared, &mut reg);
    reg
}

fn layout_registry() -> FrameRegistry {
    let mut reg = build_registry();
    recompute_layouts(&mut reg);
    reg
}

fn rect(reg: &FrameRegistry, name: &str) -> LayoutRect {
    reg.get(reg.get_by_name(name).expect(name))
        .and_then(|f| f.layout_rect.clone())
        .unwrap_or_else(|| panic!("{name} has no layout_rect"))
}

#[test]
fn builds_frame_and_title() {
    let reg = build_registry();
    assert!(reg.get_by_name("GuildControlFrame").is_some());
    assert!(reg.get_by_name("GuildControlTitle").is_some());
}

#[test]
fn builds_rank_sidebar() {
    let reg = build_registry();
    assert!(reg.get_by_name("GuildControlRankSidebar").is_some());
    for i in 0..3 {
        assert!(
            reg.get_by_name(&format!("GuildControlRank{i}")).is_some(),
            "GuildControlRank{i} missing"
        );
    }
}

#[test]
fn builds_rank_name_editor() {
    let reg = build_registry();
    assert!(reg.get_by_name("GuildControlRankNameEditor").is_some());
    assert!(reg.get_by_name("GuildControlRankNameText").is_some());
}

#[test]
fn builds_permission_checkboxes() {
    let reg = build_registry();
    for i in 0..4 {
        assert!(
            reg.get_by_name(&format!("GuildControlPerm{i}Check"))
                .is_some(),
            "GuildControlPerm{i}Check missing"
        );
        assert!(
            reg.get_by_name(&format!("GuildControlPerm{i}Label"))
                .is_some(),
            "GuildControlPerm{i}Label missing"
        );
    }
}

#[test]
fn hidden_when_not_visible() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(GuildControlState::default());
    Screen::new(guild_control_screen).sync(&shared, &mut reg);
    let id = reg.get_by_name("GuildControlFrame").expect("frame");
    assert!(reg.get(id).expect("data").hidden);
}

// --- Coord validation ---

#[test]
fn coord_main_frame_centered() {
    let reg = layout_registry();
    let r = rect(&reg, "GuildControlFrame");
    let expected_x = (1920.0 - FRAME_W) / 2.0;
    let expected_y = (1080.0 - FRAME_H) / 2.0;
    assert!((r.x - expected_x).abs() < 1.0);
    assert!((r.y - expected_y).abs() < 1.0);
    assert!((r.width - FRAME_W).abs() < 1.0);
}

#[test]
fn coord_sidebar() {
    let reg = layout_registry();
    let frame_x = (1920.0 - FRAME_W) / 2.0;
    let frame_y = (1080.0 - FRAME_H) / 2.0;
    let r = rect(&reg, "GuildControlRankSidebar");
    assert!((r.x - (frame_x + SIDEBAR_INSET)).abs() < 1.0);
    assert!((r.y - (frame_y + CONTENT_TOP)).abs() < 1.0);
    assert!((r.width - SIDEBAR_W).abs() < 1.0);
}

#[test]
fn coord_first_checkbox() {
    let reg = layout_registry();
    let r = rect(&reg, "GuildControlPerm0Check");
    assert!((r.width - CHECKBOX_SIZE).abs() < 1.0);
    assert!((r.height - CHECKBOX_SIZE).abs() < 1.0);
}

// --- Bank tab permissions tests ---

fn make_bank_perm_state() -> GuildControlState {
    let mut state = make_test_state();
    state.bank_tab_permissions = vec![
        BankTabPermission {
            tab_name: "Tab 1".into(),
            can_view: true,
            can_deposit: true,
            can_withdraw: true,
            withdraw_limit: "50".into(),
        },
        BankTabPermission {
            tab_name: "Tab 2".into(),
            can_view: true,
            can_deposit: false,
            can_withdraw: false,
            withdraw_limit: "0".into(),
        },
    ];
    state
}

fn bank_perm_registry() -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_bank_perm_state());
    Screen::new(guild_control_screen).sync(&shared, &mut reg);
    reg
}

#[test]
fn bank_perm_builds_header() {
    let reg = bank_perm_registry();
    assert!(reg.get_by_name("GuildControlBankPermHeaderTab").is_some());
    assert!(reg.get_by_name("GuildControlBankPermHeaderView").is_some());
    assert!(
        reg.get_by_name("GuildControlBankPermHeaderDeposit")
            .is_some()
    );
    assert!(
        reg.get_by_name("GuildControlBankPermHeaderWithdraw")
            .is_some()
    );
    assert!(reg.get_by_name("GuildControlBankPermHeaderLimit").is_some());
}

#[test]
fn bank_perm_builds_tab_rows() {
    let reg = bank_perm_registry();
    for i in 0..2 {
        assert!(
            reg.get_by_name(&format!("GuildControlBankTab{i}Name"))
                .is_some(),
            "GuildControlBankTab{i}Name missing"
        );
        assert!(
            reg.get_by_name(&format!("GuildControlBankTab{i}View"))
                .is_some(),
            "GuildControlBankTab{i}View missing"
        );
        assert!(
            reg.get_by_name(&format!("GuildControlBankTab{i}Deposit"))
                .is_some(),
            "GuildControlBankTab{i}Deposit missing"
        );
        assert!(
            reg.get_by_name(&format!("GuildControlBankTab{i}Withdraw"))
                .is_some(),
            "GuildControlBankTab{i}Withdraw missing"
        );
        assert!(
            reg.get_by_name(&format!("GuildControlBankTab{i}Limit"))
                .is_some(),
            "GuildControlBankTab{i}Limit missing"
        );
    }
}

// --- Additional coord validation ---

#[test]
fn coord_rank_name_editor() {
    let reg = layout_registry();
    let r = rect(&reg, "GuildControlRankNameEditor");
    let frame_x = (1920.0 - FRAME_W) / 2.0;
    let frame_y = (1080.0 - FRAME_H) / 2.0;
    let expected_x = frame_x + SIDEBAR_INSET + SIDEBAR_W + CONTENT_GAP + 84.0;
    assert!(
        (r.x - expected_x).abs() < 1.0,
        "editor x: expected {expected_x}, got {}",
        r.x
    );
    assert!((r.y - (frame_y + CONTENT_TOP)).abs() < 1.0);
    assert!((r.height - EDITOR_H).abs() < 1.0);
}

#[test]
fn coord_bank_perm_header() {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(make_bank_perm_state());
    Screen::new(guild_control_screen).sync(&shared, &mut reg);
    recompute_layouts(&mut reg);

    let header = rect(&reg, "GuildControlBankPermHeaderTab");
    assert!((header.height - BANK_TAB_ROW_H).abs() < 1.0);
}

// --- Text content tests ---

fn fontstring_text(reg: &FrameRegistry, name: &str) -> String {
    use ui_toolkit::frame::WidgetData;
    let id = reg.get_by_name(name).expect(name);
    let frame = reg.get(id).expect("frame data");
    match frame.widget_data.as_ref() {
        Some(WidgetData::FontString(fs)) => fs.text.clone(),
        _ => panic!("{name} is not a FontString"),
    }
}

fn build_with_state(state: GuildControlState) -> FrameRegistry {
    let mut reg = FrameRegistry::new(1920.0, 1080.0);
    let mut shared = SharedContext::new();
    shared.insert(state);
    Screen::new(guild_control_screen).sync(&shared, &mut reg);
    reg
}

#[test]
fn title_text() {
    let reg = build_registry();
    assert_eq!(fontstring_text(&reg, "GuildControlTitle"), "Guild Control");
}

#[test]
fn rank_sidebar_labels() {
    let reg = build_registry();
    assert_eq!(
        fontstring_text(&reg, "GuildControlRank0Label"),
        "Guild Master"
    );
    assert_eq!(fontstring_text(&reg, "GuildControlRank1Label"), "Officer");
    assert_eq!(fontstring_text(&reg, "GuildControlRank2Label"), "Member");
}

#[test]
fn rank_name_editor_text() {
    let reg = build_registry();
    assert_eq!(
        fontstring_text(&reg, "GuildControlRankNameLabel"),
        "Rank Name:"
    );
    assert_eq!(
        fontstring_text(&reg, "GuildControlRankNameText"),
        "Guild Master"
    );
}

#[test]
fn permission_labels_and_checks() {
    let reg = build_registry();
    assert_eq!(
        fontstring_text(&reg, "GuildControlPerm0Label"),
        "Invite Members"
    );
    assert_eq!(fontstring_text(&reg, "GuildControlPerm0CheckText"), "✓");
    assert_eq!(
        fontstring_text(&reg, "GuildControlPerm3Label"),
        "Edit Public Note"
    );
    assert_eq!(fontstring_text(&reg, "GuildControlPerm3CheckText"), "");
}

#[test]
fn bank_perm_header_text() {
    let reg = bank_perm_registry();
    assert_eq!(
        fontstring_text(&reg, "GuildControlBankPermHeaderTab"),
        "Bank Tab"
    );
    assert_eq!(
        fontstring_text(&reg, "GuildControlBankPermHeaderView"),
        "View"
    );
    assert_eq!(
        fontstring_text(&reg, "GuildControlBankPermHeaderDeposit"),
        "Deposit"
    );
    assert_eq!(
        fontstring_text(&reg, "GuildControlBankPermHeaderWithdraw"),
        "Withdraw"
    );
    assert_eq!(
        fontstring_text(&reg, "GuildControlBankPermHeaderLimit"),
        "Limit"
    );
}

#[test]
fn bank_tab_perm_row_text() {
    let reg = bank_perm_registry();
    assert_eq!(fontstring_text(&reg, "GuildControlBankTab0Name"), "Tab 1");
    assert_eq!(fontstring_text(&reg, "GuildControlBankTab0ViewText"), "✓");
    assert_eq!(
        fontstring_text(&reg, "GuildControlBankTab0DepositText"),
        "✓"
    );
    assert_eq!(
        fontstring_text(&reg, "GuildControlBankTab0WithdrawText"),
        "✓"
    );
    assert_eq!(fontstring_text(&reg, "GuildControlBankTab0Limit"), "50");
}

#[test]
fn bank_tab_perm_unchecked_row() {
    let reg = bank_perm_registry();
    assert_eq!(fontstring_text(&reg, "GuildControlBankTab1Name"), "Tab 2");
    assert_eq!(fontstring_text(&reg, "GuildControlBankTab1ViewText"), "✓");
    assert_eq!(fontstring_text(&reg, "GuildControlBankTab1DepositText"), "");
    assert_eq!(
        fontstring_text(&reg, "GuildControlBankTab1WithdrawText"),
        ""
    );
    assert_eq!(fontstring_text(&reg, "GuildControlBankTab1Limit"), "0");
}

#[test]
fn max_ranks_capped() {
    let mut state = make_test_state();
    state.ranks = (0..15)
        .map(|i| GuildRank {
            name: format!("Rank {i}"),
            selected: i == 0,
        })
        .collect();
    let reg = build_with_state(state);
    for i in 0..MAX_RANKS {
        assert!(
            reg.get_by_name(&format!("GuildControlRank{i}")).is_some(),
            "GuildControlRank{i} missing"
        );
    }
    assert!(
        reg.get_by_name(&format!("GuildControlRank{MAX_RANKS}"))
            .is_none()
    );
}

#[test]
fn max_permissions_capped() {
    let mut state = make_test_state();
    state.permissions = (0..16)
        .map(|i| PermissionRow {
            label: format!("Perm {i}"),
            checked: false,
        })
        .collect();
    let reg = build_with_state(state);
    for i in 0..MAX_PERMISSIONS {
        assert!(
            reg.get_by_name(&format!("GuildControlPerm{i}Check"))
                .is_some(),
            "GuildControlPerm{i}Check missing"
        );
    }
    assert!(
        reg.get_by_name(&format!("GuildControlPerm{MAX_PERMISSIONS}Check"))
            .is_none()
    );
}
