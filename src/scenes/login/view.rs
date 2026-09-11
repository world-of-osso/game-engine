use super::*;

#[cfg(test)]
#[path = "view_tests.rs"]
mod tests;

pub(super) fn build_login_screen(
    status: &LoginStatus,
    realm_text: String,
    realm_selectable: bool,
) -> LoginScreenRes {
    let mut shared = ui_toolkit::screen::SharedContext::new();
    shared.insert::<SharedStatusText>(SharedStatusText(status.0.clone()));
    shared.insert::<SharedConnecting>(SharedConnecting(false));
    shared.insert::<SharedRealmText>(SharedRealmText(realm_text));
    shared.insert::<SharedRealmSelectable>(SharedRealmSelectable(realm_selectable));
    let screen = Screen::new(login_screen);

    LoginScreenRes { screen, shared }
}

pub(crate) fn apply_post_setup(reg: &mut FrameRegistry, login: &LoginUi) {
    set_editbox_backdrop(reg, login.username_input);
    set_editbox_backdrop(reg, login.password_input);
    set_login_primary_button_textures(reg, login.connect_button);
    if let Some(reconnect_button) = login.reconnect_button {
        set_login_primary_button_textures(reg, reconnect_button);
    }
}

fn set_editbox_backdrop(reg: &mut FrameRegistry, id: u64) {
    if let Some(frame) = reg.get_mut(id) {
        frame.nine_slice = Some(NineSlice {
            edge_size: 8.0,
            part_textures: Some(common_input_border_part_textures()),
            bg_color: EDITBOX_BG,
            border_color: EDITBOX_BORDER,
            ..Default::default()
        });
        if let Some(WidgetData::EditBox(eb)) = &mut frame.widget_data {
            eb.text_insets = [12.0, 5.0, 8.0, 8.0];
            eb.font = GameFont::ArialNarrow;
            eb.text_color = GLUE_EDITBOX_TEXT_COLOR;
        }
    }
}

fn common_input_border_part_textures() -> [TextureSource; 9] {
    let base = "data/ui/Common-Input-Border-";
    [
        TextureSource::File(format!("{base}TL.blp")),
        TextureSource::File(format!("{base}T.blp")),
        TextureSource::File(format!("{base}TR.blp")),
        TextureSource::File(format!("{base}L.blp")),
        TextureSource::File(format!("{base}M.blp")),
        TextureSource::File(format!("{base}R.blp")),
        TextureSource::File(format!("{base}BL.blp")),
        TextureSource::File(format!("{base}B.blp")),
        TextureSource::File(format!("{base}BR.blp")),
    ]
}

pub(super) fn sync_login_status(
    reg: &mut FrameRegistry,
    screen_res: Option<&mut ResMut<LoginScreenResWrap>>,
    status: &LoginStatus,
    realm_text: String,
    realm_selectable: bool,
) {
    let Some(res) = screen_res else { return };
    let inner = &mut res.0;
    let connecting = status.0 == STATUS_CONNECTING;
    if inner
        .shared
        .get::<SharedStatusText>()
        .is_none_or(|value| value.0 != status.0)
    {
        inner.shared.insert(SharedStatusText(status.0.clone()));
    }
    if inner
        .shared
        .get::<SharedConnecting>()
        .is_none_or(|value| value.0 != connecting)
    {
        inner.shared.insert(SharedConnecting(connecting));
    }
    if inner
        .shared
        .get::<SharedRealmText>()
        .is_none_or(|value| value.0 != realm_text)
    {
        inner.shared.insert(SharedRealmText(realm_text));
    }
    if inner
        .shared
        .get::<SharedRealmSelectable>()
        .is_none_or(|value| value.0 != realm_selectable)
    {
        inner.shared.insert(SharedRealmSelectable(realm_selectable));
    }
    inner.screen.sync(&inner.shared, reg);
}
