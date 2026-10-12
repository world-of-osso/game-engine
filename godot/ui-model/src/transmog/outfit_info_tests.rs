use super::*;

#[test]
fn transmog_icon_edit_preserves_pending_apply_and_saved_configuration() {
    let mut book = super::wardrobe_tests::book();
    assert!(book.stage(TransmogChoice::Source(1181)));
    let pending = book.pending.clone();
    let money = book.money;
    assert!(book.edit_outfit_info(2));
    assert_eq!(book.outfit_info.as_ref().unwrap().icon_fdid, 0);
    assert!(!book.select_outfit_icon(999999));
    assert!(book.select_outfit_icon(135343));
    book.outfit_info.as_mut().unwrap().name = "Chosen blade".into();
    let Some(TransmogSend::SaveOutfit(save)) = book.commit_outfit_info() else {
        panic!("metadata save");
    };
    assert_eq!(save.outfit.icon_fdid, 135343);
    assert_eq!(save.outfit.name, "Chosen blade");
    assert_eq!(book.pending, pending);
    assert_eq!(book.money, money);
    assert!(!book.applying);
    let mut state = book.state.clone().unwrap();
    state.outfits = vec![save.outfit];
    book.receive(state);
    assert_eq!(book.outfit_rows()[0].icon_fdid, 135343);
    assert_eq!(book.pending, pending);
    assert_eq!(book.money, money);
    assert!(book.edit_outfit_info(2));
    assert_eq!(book.outfit_info.as_ref().unwrap().icon_fdid, 135343);
}
