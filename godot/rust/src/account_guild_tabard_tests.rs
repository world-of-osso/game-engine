use super::*;
#[test]
fn guild_tabard_native_account_delivers_vendor_and_saved_authority() {
    use shared::{
        guild_creation::GuildTabardDesign,
        protocol::{GuildTabardReply, GuildTabardVendorSnapshot},
    };
    let mut account = Account::new(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data"));
    let design = GuildTabardDesign {
        icon: 1,
        icon_color: 2,
        border: 3,
        border_color: 4,
        background_color: 5,
    };
    for reply in [
        GuildTabardReply::Vendor(GuildTabardVendorSnapshot {
            npc: 701,
            npc_name: "Jondor Steelbrow".into(),
            own_rank: Some(0),
            design: None,
            cost: 100000,
        }),
        GuildTabardReply::Saved { design },
        GuildTabardReply::Failed {
            message: "You can't afford to do that.".into(),
        },
    ] {
        let mut events = Vec::new();
        account
            .dispatch_message(ProtocolMessage::for_tests(reply.clone()), &mut events)
            .unwrap();
        assert_eq!(events.len(), 1, "native account discarded tabard authority");
        assert!(matches!(&events[0], AccountEvent::GuildTabard(actual) if *actual == reply));
    }
}
