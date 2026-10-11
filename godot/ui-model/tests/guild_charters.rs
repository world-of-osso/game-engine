use game_engine_ui_model::bank::InputTexts;
use game_engine_ui_model::guild_charters::*;
use shared::protocol::*;
fn registrar() -> GuildRegistrarSnapshot {
    GuildRegistrarSnapshot {
        npc: 810,
        npc_name: "Aldwin".into(),
        charter_cost: 1000,
        charter_guid: None,
    }
}
fn petition(owner: bool) -> GuildPetitionSnapshot {
    GuildPetitionSnapshot {
        charter_guid: 701,
        guild_name: "Raid Team".into(),
        owner_name: "Cara".into(),
        signatures: vec!["Elowen".into()],
        required_signatures: 4,
        is_owner: owner,
        can_sign: !owner,
    }
}
#[test]
fn guild_charter_purchase_waits_for_authority_and_closes_on_purchase() {
    let mut view = GuildCharters::default();
    view.receive(GuildCharterReply::Registrar(registrar()));
    assert_eq!(view.registrar, Some(registrar()));
    view.click("charter:purchase_open", &InputTexts::default(), None);
    assert!(view.purchasing);
    let texts = InputTexts::from([(PURCHASE_NAME.into(), "Raid Team".into())]);
    assert_eq!(
        view.click("charter:purchase", &texts, None),
        Some(GuildCharterRequest::Purchase {
            npc: 810,
            guild_name: "Raid Team".into()
        })
    );
    assert!(view.waiting);
    assert_eq!(view.click("charter:purchase", &texts, None), None);
    view.receive(GuildCharterReply::Failed {
        message: "You cannot afford that.".into(),
    });
    assert!(!view.waiting);
    assert_eq!(view.error.as_deref(), Some("You cannot afford that."));
    view.click("charter:purchase", &texts, None);
    view.receive(GuildCharterReply::Purchased { charter_guid: 701 });
    assert!(view.registrar.is_none());
}
#[test]
fn guild_charter_owner_requests_target_signature_and_renames_without_optimism() {
    let mut view = GuildCharters::default();
    view.receive(GuildCharterReply::Petition(petition(true)));
    assert_eq!(
        view.click(
            "charter:offer",
            &InputTexts::default(),
            Some("Elowen".into())
        ),
        Some(GuildCharterRequest::Offer {
            charter_guid: 701,
            character_name: "Elowen".into()
        })
    );
    view.receive(GuildCharterReply::Petition(petition(true)));
    view.click("charter:rename_open", &InputTexts::default(), None);
    assert!(view.renaming);
    let texts = InputTexts::from([(RENAME_NAME.into(), "Raid Knights".into())]);
    assert_eq!(
        view.click("charter:rename", &texts, None),
        Some(GuildCharterRequest::Rename {
            charter_guid: 701,
            guild_name: "Raid Knights".into()
        })
    );
    assert_eq!(view.petition.as_ref().unwrap().guild_name, "Raid Team");
    let mut updated = petition(true);
    updated.guild_name = "Raid Knights".into();
    view.receive(GuildCharterReply::Petition(updated));
    assert!(!view.renaming);
    assert_eq!(view.petition.as_ref().unwrap().guild_name, "Raid Knights");
}
#[test]
fn guild_charter_signer_cannot_offer_or_rename_and_signs_once_per_reply() {
    let mut view = GuildCharters::default();
    view.receive(GuildCharterReply::Petition(petition(false)));
    assert_eq!(
        view.click("charter:offer", &InputTexts::default(), Some("Cara".into())),
        None
    );
    view.click("charter:rename_open", &InputTexts::default(), None);
    assert!(!view.renaming);
    assert_eq!(
        view.click("charter:sign", &InputTexts::default(), None),
        Some(GuildCharterRequest::Sign { charter_guid: 701 })
    );
    assert_eq!(
        view.click("charter:sign", &InputTexts::default(), None),
        None
    );
    let mut signed = petition(false);
    signed.can_sign = false;
    view.receive(GuildCharterReply::Petition(signed));
    assert_eq!(
        view.click("charter:sign", &InputTexts::default(), None),
        None
    );
}
#[test]
fn guild_charter_turn_in_uses_registrar_owned_guid_and_creation_closes_views() {
    let mut view = GuildCharters::default();
    let mut r = registrar();
    r.charter_guid = Some(701);
    view.receive(GuildCharterReply::Registrar(r));
    assert_eq!(
        view.click("charter:turn_in", &InputTexts::default(), None),
        Some(GuildCharterRequest::TurnIn {
            npc: 810,
            charter_guid: 701
        })
    );
    view.receive(GuildCharterReply::Created {
        guild_id: 9,
        guild_name: "Raid Team".into(),
    });
    assert!(view.registrar.is_none() && view.petition.is_none() && !view.waiting);
}

#[test]
fn guild_charter_retail_controls_use_authoritative_quota_in_both_skins() {
    use ui_toolkit::{
        atlas::{ActiveSkin, set_thread_skin},
        frame::WidgetData,
        registry::FrameRegistry,
        screen::{Screen, SharedContext},
    };
    game_engine_ui_model::paths::set_data_root(
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"),
    )
    .unwrap();
    for skin in [ActiveSkin::Modern, ActiveSkin::Forever] {
        set_thread_skin(skin);
        let mut view = GuildCharters::default();
        view.receive(GuildCharterReply::Registrar(registrar()));
        let mut shared = SharedContext::new();
        shared.insert(view.clone());
        let mut registry = FrameRegistry::new(1280.0, 720.0);
        let mut screen = Screen::new(game_engine_ui_model::guild_charter_frame::charter_screen);
        screen.sync(&shared, &mut registry);
        assert!(
            registry.get_by_name("GuildRegistrarButton1").is_some(),
            "{skin:?} registrar purchase missing"
        );
        assert!(
            registry
                .get_by_name("GuildRegistrarFramePortrait")
                .is_some(),
            "Retail NPC portrait missing"
        );
        view.click("charter:purchase_open", &InputTexts::default(), None);
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name(PURCHASE_NAME).is_some());
        view.receive(GuildCharterReply::Petition(petition(true)));
        shared.insert(view.clone());
        screen.sync(&shared, &mut registry);
        assert!(registry.get_by_name("PetitionFrameRequestButton").is_some());
        assert!(registry.get_by_name("PetitionFrameSignButton").is_none());
        let row = registry
            .get(
                registry
                    .get_by_name("PetitionFrameMemberName4")
                    .expect("authoritative fourth signature"),
            )
            .unwrap();
        assert!(
            matches!(&row.widget_data, Some(WidgetData::FontString(t)) if t.text == "Not yet signed")
        );
        assert!(registry.get_by_name("PetitionFrameMemberName5").is_none());
        view.receive(GuildCharterReply::Petition(petition(false)));
        shared.insert(view);
        screen.sync(&shared, &mut registry);
        let sign = registry
            .get(registry.get_by_name("PetitionFrameSignButton").unwrap())
            .unwrap();
        assert_eq!(sign.onclick.as_deref(), Some("charter:sign"));
        assert!(registry.get_by_name("PetitionFrameRenameButton").is_none());
    }
    set_thread_skin(ActiveSkin::Modern);
}
