//! World map canvas tiles, highlights, pins, and player marker.

use super::*;

pub(super) fn canvas(state: &WorldMapFrameState, layout: &WorldMapLayout) -> Element {
    let [left, top, w, h] = layout.local_canvas();
    let children = canvas_contents(state, [w, h]);
    rsx! {
        r#frame {
            name: WORLD_MAP_CANVAS,
            width: w,
            height: h,
            background_color: "0.0,0.0,0.0,1.0",
            pos_type: "absolute",
            left,
            top,
            {children}
        }
    }
}

fn scale_canvas_rect([x, y, rw, rh]: [f32; 4], [w, h]: [f32; 2]) -> [f32; 4] {
    [x * w, y * h, rw * w, rh * h]
}

/// Shared tiled map canvas for world and flight maps.
pub fn canvas_contents(state: &WorldMapFrameState, size: [f32; 2]) -> Element {
    let mut children: Element = state
        .tiles
        .iter()
        .enumerate()
        .flat_map(|(index, tile)| {
            let name = format!("WorldMapTile{index}");
            textured(
                name,
                tile.fdid,
                tile.tex_coords,
                scale_canvas_rect(tile.rect, size),
            )
        })
        .collect();
    if let Some(highlight) = &state.highlight {
        children.extend(canvas_highlight(highlight, size));
    }
    if !state.quest_areas.is_empty() {
        children.extend(quest_areas(size));
    }
    for (index, pin) in state.pins.iter().enumerate() {
        children.extend(map_pin(index, pin, size));
    }
    if let Some(player) = &state.player {
        children.extend(player_arrow(player, size));
    }
    children
}

fn canvas_highlight(highlight: &MapHighlight, [w, h]: [f32; 2]) -> Element {
    let mut children = Vec::new();
    if highlight.fdid != 0 {
        children.extend(highlight_texture(
            highlight,
            scale_canvas_rect(highlight.rect, [w, h]),
        ));
    }
    children.extend(label(
        WORLD_MAP_HIGHLIGHT_NAME.into(),
        &highlight.name,
        [0.0, 12.0, w, 32.0],
        24.0,
        GOLD,
    ));
    children
}

fn player_arrow(player: &MapPlayerMarker, [w, h]: [f32; 2]) -> Element {
    image(
        WORLD_MAP_PLAYER_ARROW.into(),
        art::PLAYER_ARROW,
        [
            player.x * w - ARROW_SIZE / 2.0,
            player.y * h - ARROW_SIZE / 2.0,
            ARROW_SIZE,
            ARROW_SIZE,
        ],
    )
}

/// The host's objective area overlay over the whole canvas; the host sets its texture.
fn quest_areas([width, height]: [f32; 2]) -> Element {
    rsx! {
        texture {
            name: WORLD_MAP_QUEST_AREAS,
            width,
            height,
            pos_type: "absolute",
            left: 0.0,
            top: 0.0,
        }
    }
}

fn highlight_texture(highlight: &MapHighlight, rect: [f32; 4]) -> Element {
    let [x, y, width, height] = rect;
    rsx! {
        texture {
            name: {DynName(WORLD_MAP_HIGHLIGHT.into())},
            width,
            height,
            texture_fdid: {highlight.fdid},
            pos_type: "absolute",
            left: x,
            top: y,
        }
    }
}

fn map_pin(index: usize, pin: &MapPin, [w, h]: [f32; 2]) -> Element {
    let size = pin.pin_type.size();
    let rect = [pin.x * w - size / 2.0, pin.y * h - size / 2.0, size, size];
    let mut pin_frames = image(format!("WorldMapPin{index}"), pin.pin_type.art(), rect);
    if !pin.badge.is_empty() {
        pin_frames.extend(label(
            format!("WorldMapPin{index}Badge"),
            &pin.badge,
            rect,
            11.0,
            WHITE,
        ));
    }
    pin_frames
}
