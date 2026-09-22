use bevy::prelude::*;
use game_engine::ui::character_creation_icons::CharacterCreationIconMasks;
use game_engine::ui::plugin::UiState;
use game_engine::ui::widgets::{WidgetData, texture::TextureSource};

pub(super) fn mask_character_create_icons(
    mut ui: ResMut<UiState>,
    mut masks: ResMut<CharacterCreationIconMasks>,
    images: Option<ResMut<Assets<Image>>>,
) {
    let Some(mut images) = images else { return };
    let pending: Vec<_> = ui
        .registry
        .frames_iter()
        .filter_map(|frame| {
            let name = frame.name.as_deref()?;
            if !name.ends_with("_Icon")
                || !(name.starts_with("Race_") || name.starts_with("Class_"))
            {
                return None;
            }
            let Some(WidgetData::Texture(texture)) = &frame.widget_data else {
                return None;
            };
            let TextureSource::FileDataId(fdid) = texture.source else {
                return None;
            };
            Some((frame.id, fdid))
        })
        .collect();
    for (id, fdid) in pending {
        let source = match masks.get_or_load(fdid, &mut images) {
            Ok(image) => TextureSource::Dynamic(image),
            Err(error) => {
                error!("Character-creation icon {fdid} cannot be masked: {error}");
                TextureSource::None
            }
        };
        if let Some(frame) = ui.registry.get_mut(id)
            && let Some(WidgetData::Texture(texture)) = &mut frame.widget_data
        {
            texture.source = source;
        }
    }
}
