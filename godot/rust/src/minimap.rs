//! In-world MinimapCluster (docs/specs/minimap.md) over the shared cluster screen: the
//! round north-up composite of the local-CASC `world/minimaps/<map>/mapXX_YY.blp` tiles
//! under the player, the facing arrow, quest-giver blips from `QuestGiverStatusMultiple`,
//! the subzone text in its PvP colour, the local-time clock and calendar day. Hovering the
//! map shows the zoom buttons; the wheel over it zooms, as `MinimapMixin:OnMouseWheel`.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use game_engine_core::minimap_data::{
    AreaCatalog, MinimapView, TileImage, TileKey, compose, parse_race_faction_groups, sample,
    tile_path, zoom_in, zoom_out,
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::minimap::{
    ACTION_TOGGLE_WORLD_MAP, ACTION_ZOOM_IN, ACTION_ZOOM_OUT, BlipKind, MINIMAP_ARROW,
    MINIMAP_DISPLAY, MINIMAP_MAIL_FRAME, MINIMAP_ZONE_TEXT, MinimapBlip, MinimapClusterState,
    mail_tooltip_lines, minimap_texture_fdids,
};
use game_engine_ui_model::tooltip_presentation::{
    TOOLTIP_W, TooltipLineState, TooltipPresentation,
};
use game_engine_ui_model::world_map_view_data::arrow_rotation;
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion, Time};
use godot::global::MouseButton;
use godot::prelude::*;
use osso_asset_resolver::CascListfileResolver;
use shared::components::Position;
use shared::protocol::{NpcFlags, QuestGiverStatus};
use ui_toolkit::frame::WidgetData;

use crate::{GameClient, frame_error::FrameError, replicated::UnitFields, ui::RegistryUi};

/// Composite resolution: the 198-unit map at up to 1.3× UI scale without upsampling.
const COMPOSITE_PX: u32 = 256;
const DB2_DIR: &str = "db2/12.1.0.69933";

#[derive(Default)]
pub(crate) struct Minimap {
    pub(crate) ui: Option<Gd<RegistryUi>>,
    zoom: u8,
    hovered: bool,
    catalogs: Option<Result<Catalogs, String>>,
    resolver: Option<CascListfileResolver>,
    /// Decoded tiles by map directory and key; None where the install has no tile.
    tiles: HashMap<(String, TileKey), Option<Tile>>,
    /// Whether each chrome texture FDID is on disk.
    chrome: HashMap<u32, bool>,
    /// Map and view of the current composite, and its pixels.
    drawn: Option<(String, MinimapView, Vec<u8>)>,
    /// Quest givers already sent in a `QuestGiverStatusQuery`; the server re-sends
    /// their status after every quest change.
    queried: HashSet<u64>,
}

struct Catalogs {
    areas: AreaCatalog,
    races: HashMap<u8, u32>,
}

struct Tile {
    fdid: u32,
    image: TileImage,
}

impl Minimap {
    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.drawn = None;
        self.hovered = false;
    }

    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        match self.ui.as_mut() {
            Some(ui) => visit(ui),
            None => Ok(()),
        }
    }

    fn resolver(&mut self, data_root: &std::path::Path) -> &CascListfileResolver {
        self.resolver
            .get_or_insert_with(|| crate::assets::creature::local_resolver(data_root))
    }

    /// Decode one tile out of local CASC once; an ocean or unlisted tile stays None.
    fn load_tile(&mut self, data_root: &std::path::Path, map: &str, key: TileKey) {
        let slot = (map.to_owned(), key);
        if self.tiles.contains_key(&slot) {
            return;
        }
        let path = tile_path(map, key);
        let resolver = self.resolver(data_root);
        let tile = resolver.lookup_path(&path).and_then(|fdid| {
            let cache = data_root.join("textures").join(format!("{fdid}.blp"));
            let file = resolver.ensure_cached(fdid, &cache)?;
            let decoded = std::fs::read(&file)
                .map_err(|error| error.to_string())
                .and_then(|bytes| game_engine_core::blp::decode_rgba(&bytes));
            match decoded {
                Ok(rgba) => Some(Tile {
                    fdid,
                    image: TileImage {
                        pixels: rgba.pixels,
                        width: rgba.width,
                        height: rgba.height,
                    },
                }),
                Err(error) => {
                    godot_warn!("Minimap tile {path} (FDID {fdid}): {error}");
                    None
                }
            }
        });
        self.tiles.insert(slot, tile);
    }

    fn tile(&self, map: &str, key: TileKey) -> Option<&Tile> {
        self.tiles.get(&(map.to_owned(), key))?.as_ref()
    }

    /// Copy chrome textures out of local CASC; a missing one is reported, not fatal.
    fn cache_chrome(&mut self, data_root: &std::path::Path, fdids: &[u32]) {
        for &fdid in fdids {
            if self.chrome.contains_key(&fdid) {
                continue;
            }
            let path = data_root.join("textures").join(format!("{fdid}.blp"));
            let found = path.exists()
                || self
                    .resolver(data_root)
                    .ensure_cached(fdid, &path)
                    .is_some();
            if !found {
                godot_warn!("Minimap texture FDID {fdid} is not in local CASC");
            }
            self.chrome.insert(fdid, found);
        }
    }
}

fn load_catalogs(data_root: &std::path::Path) -> Result<Catalogs, String> {
    let open = |path: PathBuf| {
        File::open(&path)
            .map(BufReader::new)
            .map_err(|error| format!("Cannot open {}: {error}", path.display()))
            .map(|reader| (reader, path))
    };
    let (areas, areas_path) = open(data_root.join("AreaTable.csv"))?;
    let (races, races_path) = open(data_root.join(DB2_DIR).join("ChrRaces.csv"))?;
    Ok(Catalogs {
        areas: AreaCatalog::parse(areas, &areas_path)?,
        races: parse_race_faction_groups(races, &races_path)?,
    })
}

/// Local wall-clock hour, minute and day of month (`timeMgrUseLocalTime` shows local time;
/// the protocol carries no realm time).
fn local_time() -> (u32, u32, u32) {
    let time = Time::singleton().get_datetime_dict_from_system();
    let field = |key: &str| time.get(key).map_or(0, |value| value.to::<i64>() as u32);
    (field("hour"), field("minute"), field("day"))
}

impl GameClient {
    /// Per frame in world: quest-giver queries, frame actions, and the cluster view.
    pub(super) fn update_minimap(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld
            || !self.client_options.hud.show_minimap
        {
            self.minimap.free_ui();
            self.minimap.queried.clear();
            self.account.quest_giver_status.clear();
            return Ok(());
        }
        self.query_quest_givers()?;
        self.poll_minimap_actions()?;
        Ok(self.sync_minimap()?)
    }

    /// Bevy `quests.rs`: every mirrored NPC with `NPCFlags::QUESTGIVER` is queried once.
    fn query_quest_givers(&mut self) -> Result<(), FrameError> {
        let units = &self.replica;
        let removed: Vec<u64> = self
            .minimap
            .queried
            .iter()
            .copied()
            .filter(|id| units.unit(*id).is_none())
            .collect();
        for id in removed {
            self.minimap.queried.remove(&id);
            self.account.quest_giver_status.remove(&id);
        }
        let new: Vec<u64> = self
            .replica
            .units()
            .filter(|unit| {
                unit.npc_flags()
                    .is_some_and(|flags| flags & NpcFlags::QUESTGIVER != 0)
            })
            .map(|unit| unit.server_id)
            .filter(|id| !self.minimap.queried.contains(id))
            .collect();
        if new.is_empty() {
            return Ok(());
        }
        self.minimap.queried.extend(new.iter().copied());
        Ok(self.account.send_quest_giver_status_query(new)?)
    }

    fn poll_minimap_actions(&mut self) -> Result<(), String> {
        let Some(ui) = self.minimap.ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        match action.as_str() {
            "" => {}
            ACTION_ZOOM_IN => self.minimap.zoom = zoom_in(self.minimap.zoom),
            ACTION_ZOOM_OUT => self.minimap.zoom = zoom_out(self.minimap.zoom),
            ACTION_TOGGLE_WORLD_MAP => {
                // As the M binding: a map failure is reported and closes the map.
                if let Err(error) = self.toggle_world_map() {
                    godot_error!("World map: {error}");
                    self.close_world_map();
                }
            }
            other => return Err(format!("Unknown minimap action: {other}")),
        }
        Ok(())
    }

    /// Engine `(x, z)` of the local player and its facing yaw.
    fn minimap_player(&self) -> Option<([f32; 2], f32)> {
        let origin = self.world.local_player_transform()?.origin;
        Some(([origin.x, origin.z], self.world.local_player_facing()?))
    }

    fn player_faction_group(&self, catalogs: &Catalogs) -> u32 {
        let session = &self.account.session;
        session
            .characters
            .iter()
            .find(|entry| Some(entry.character_id) == session.selected_character_id)
            .and_then(|entry| catalogs.races.get(&entry.race).copied())
            .unwrap_or(0)
    }

    fn minimap_cluster_state(
        &mut self,
        position: [f32; 2],
        yaw: f32,
    ) -> Result<MinimapClusterState, String> {
        if self.minimap.catalogs.is_none() {
            self.minimap.catalogs = Some(load_catalogs(&self.data_root));
        }
        let catalogs = match self.minimap.catalogs.as_ref() {
            Some(Ok(catalogs)) => catalogs,
            Some(Err(error)) => return Err(format!("Minimap catalogs: {error}")),
            None => unreachable!("catalogs loaded above"),
        };
        let area = self.terrain.area_id_at(position[0], position[1]);
        let zone_text = area
            .and_then(|area| catalogs.areas.name(area))
            .unwrap_or_default()
            .to_owned();
        let pvp = area.map(|area| {
            catalogs
                .areas
                .pvp(area, self.player_faction_group(catalogs))
        });
        let (hour, minute, day) = local_time();
        let view = MinimapView::new(position, self.minimap.zoom);
        Ok(MinimapClusterState {
            zone_text,
            zone_color: pvp.map_or([1.0, 0.82, 0.0, 1.0], |pvp| pvp.text_color()),
            clock_text: game_engine_core::minimap_data::clock_text(hour, minute),
            calendar_day: Some(day),
            arrow_rotation: arrow_rotation(yaw),
            zoom_buttons: self.minimap.hovered,
            zoom: self.minimap.zoom,
            blips: self.quest_blips(&view),
            has_mail: !self.mailbox.session.pending_senders.is_empty(),
            map_texture: None,
        })
    }

    /// `QuestNormal` for givers with an available quest, `QuestTurnin` for a reward;
    /// the other statuses draw no minimap blip.
    fn quest_blips(&self, view: &MinimapView) -> Vec<MinimapBlip> {
        let mut blips: Vec<MinimapBlip> = self
            .account
            .quest_giver_status
            .iter()
            .filter_map(|(&unit, status)| {
                let kind = match status {
                    QuestGiverStatus::Available => BlipKind::QuestAvailable,
                    QuestGiverStatus::Reward => BlipKind::QuestTurnIn,
                    _ => return None,
                };
                let position = self.replica.unit(unit)?.get::<Position>()?;
                let offset = view.blip_offset([position.x, position.z])?;
                Some(MinimapBlip { unit, kind, offset })
            })
            .collect();
        blips.sort_by_key(|blip| blip.unit);
        blips
    }

    /// Recomposite when the player moved half a composite pixel, zoomed, or changed map.
    fn minimap_composite(&mut self, position: [f32; 2]) -> Option<(u32, Vec<u8>)> {
        let map = self.terrain.map_name()?.to_owned();
        let view = MinimapView::new(position, self.minimap.zoom);
        let pixel_yards = view.diameter / COMPOSITE_PX as f32;
        if let Some((drawn_map, drawn, _)) = &self.minimap.drawn
            && *drawn_map == map
            && drawn.diameter == view.diameter
            && (drawn.center[0] - view.center[0]).hypot(drawn.center[1] - view.center[1])
                < pixel_yards / 2.0
        {
            return None;
        }
        for key in view.tiles() {
            self.minimap.load_tile(&self.data_root, &map, key);
        }
        let minimap = &self.minimap;
        let pixels = compose(&view, COMPOSITE_PX, |key| {
            minimap.tile(&map, key).map(|tile| &tile.image)
        });
        self.minimap.drawn = Some((map, view, pixels.clone()));
        Some((COMPOSITE_PX, pixels))
    }

    fn sync_minimap(&mut self) -> Result<(), String> {
        let Some((position, yaw)) = self.minimap_player() else {
            return Ok(());
        };
        let state = self.minimap_cluster_state(position, yaw)?;
        let data_root = self.data_root.clone();
        self.minimap
            .cache_chrome(&data_root, &minimap_texture_fdids(&state));
        let composite = self.minimap_composite(position);
        if let Some(ui) = self.minimap.ui.as_mut() {
            return ui.bind_mut().set_minimap(state, composite);
        }
        let mut ui = RegistryUi::new_alloc();
        ui.set_name("MinimapUI");
        self.base_mut().add_child(&ui);
        ui.bind_mut().set_ui_scale(self.effective_ui_scale())?;
        let shown = ui.bind_mut().show_minimap(state.clone());
        let shown = shown.and_then(|()| ui.bind_mut().set_minimap(state, composite));
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.minimap.ui = Some(ui);
        Ok(())
    }

    /// The map circle in physical pixels: centre and radius.
    fn minimap_circle(&self) -> Option<(Vector2, f32)> {
        let ui = self.minimap.ui.as_ref()?;
        let ui = ui.bind();
        let registry = ui.registry()?;
        let rect = registry
            .get(registry.get_by_name(MINIMAP_DISPLAY)?)?
            .layout_rect
            .clone()?;
        let scale = self.effective_ui_scale();
        let centre = Vector2::new(rect.x + rect.width / 2.0, rect.y + rect.height / 2.0) * scale;
        Some((centre, rect.width / 2.0 * scale))
    }

    /// `MiniMapMailFrameMixin:OnEnter`: the unread senders, `ANCHOR_BOTTOMLEFT` of the icon.
    pub(super) fn minimap_mail_tooltip(&self) -> Option<TooltipPresentation> {
        let senders = &self.mailbox.session.pending_senders;
        if senders.is_empty() {
            return None;
        }
        let ui = self.minimap.ui.as_ref()?.bind();
        let rect = ui.frame_control(MINIMAP_MAIL_FRAME)?.get_global_rect();
        if !rect.contains_point(Vector2::from_array(self.physical_input.pointer())) {
            return None;
        }
        let scale = self.effective_ui_scale();
        let (title, lines) = mail_tooltip_lines(senders);
        Some(TooltipPresentation {
            visible: true,
            x: (rect.position.x / scale - TOOLTIP_W).max(0.0),
            y: rect.end().y / scale,
            title: title.into(),
            lines: lines.into_iter().map(TooltipLineState::new).collect(),
            ..TooltipPresentation::hidden()
        })
    }

    /// Pointer over the map: hover shows the zoom buttons (`MinimapMixin:OnEnter`), and
    /// the wheel zooms instead of the camera. Returns whether `event` was consumed.
    pub(super) fn minimap_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        let Some((centre, radius)) = self.minimap_circle() else {
            return false;
        };
        let scale = radius / (game_engine_ui_model::minimap::MAP_SIZE / 2.0);
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            let point = motion.get_position();
            // ZoomHitArea 40×40 at CENTER (+77, −77) keeps the buttons while over them.
            let hit_area = centre + Vector2::new(77.0, 77.0) * scale;
            let over_buttons = (point.x - hit_area.x).abs() <= 20.0 * scale
                && (point.y - hit_area.y).abs() <= 20.0 * scale;
            self.minimap.hovered = point.distance_to(centre) <= radius || over_buttons;
            return false;
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if !button.is_pressed() || button.get_position().distance_to(centre) > radius {
            return false;
        }
        match button.get_button_index() {
            MouseButton::WHEEL_UP => self.minimap.zoom = zoom_in(self.minimap.zoom),
            MouseButton::WHEEL_DOWN => self.minimap.zoom = zoom_out(self.minimap.zoom),
            _ => return false,
        }
        true
    }
}

#[godot_api(secondary)]
impl GameClient {
    /// Minimap view, drawn tile, arrow, zone text and blips, for fixtures.
    #[func]
    fn minimap_state(&self) -> VarDictionary {
        let mut result = VarDictionary::new();
        result.set("open", self.minimap.ui.is_some());
        result.set("zoom", i64::from(self.minimap.zoom));
        result.set("hovered", self.minimap.hovered);
        if let Some((_, yaw)) = self.minimap_player() {
            result.set("facing_yaw", yaw);
        }
        self.describe_minimap_composite(&mut result);
        self.describe_minimap_frames(&mut result);
        result
    }

    /// RGBA of the current composite at map UV `(u, v)` (0..1), for rendered-pixel checks.
    #[func]
    fn minimap_composite_pixel(&self, u: f32, v: f32) -> PackedInt32Array {
        let Some((_, _, pixels)) = &self.minimap.drawn else {
            return PackedInt32Array::new();
        };
        let last = COMPOSITE_PX - 1;
        let x = ((u * COMPOSITE_PX as f32) as u32).min(last);
        let y = ((v * COMPOSITE_PX as f32) as u32).min(last);
        let offset = ((y * COMPOSITE_PX + x) * 4) as usize;
        rgba_array(&pixels[offset..offset + 4])
    }
}

impl GameClient {
    /// The composite's map, view and tile under the player, and both their centre colours.
    fn describe_minimap_composite(&self, result: &mut VarDictionary) {
        let Some((map, view, pixels)) = &self.minimap.drawn else {
            return;
        };
        result.set("map", map.as_str());
        result.set("diameter", view.diameter);
        result.set("center", Vector2::new(view.center[0], view.center[1]));
        let size = COMPOSITE_PX as usize;
        let centre = (size / 2 * size + size / 2) * 4;
        result.set("center_pixel", &rgba_array(&pixels[centre..centre + 4]));
        let tiles = view.tiles();
        let loaded = tiles
            .iter()
            .filter(|key| self.minimap.tile(map, **key).is_some());
        result.set("tiles_loaded", loaded.count() as i64);
        result.set("tiles", tiles.len() as i64);
        let Some((key, uv)) = view.tile_uv(view.center) else {
            return;
        };
        result.set("tile", Vector2i::new(key.0 as i32, key.1 as i32));
        result.set("tile_path", tile_path(map, key).as_str());
        result.set("tile_uv", Vector2::new(uv[0], uv[1]));
        if let Some(tile) = self.minimap.tile(map, key) {
            result.set("tile_fdid", i64::from(tile.fdid));
        }
        // The tile texel under the centre composite pixel, sampled directly.
        let half = COMPOSITE_PX / 2;
        let texel = view
            .tile_uv(view.pixel_position(half, half, COMPOSITE_PX))
            .and_then(|(key, uv)| Some(sample(&self.minimap.tile(map, key)?.image, uv)));
        if let Some(texel) = texel {
            result.set("tile_texel", &rgba_array(&texel));
        }
    }

    /// What the registry holds: arrow rotation, zone text and blip count.
    fn describe_minimap_frames(&self, result: &mut VarDictionary) {
        let Some(ui) = &self.minimap.ui else {
            return;
        };
        let ui = ui.bind();
        let Some(registry) = ui.registry() else {
            return;
        };
        let frame = |name: &str| registry.get(registry.get_by_name(name)?);
        if let Some(WidgetData::Texture(texture)) =
            frame(MINIMAP_ARROW).and_then(|frame| frame.widget_data.as_ref())
        {
            result.set("arrow_rotation", texture.rotation);
        }
        if let Some(WidgetData::FontString(text)) =
            frame(MINIMAP_ZONE_TEXT).and_then(|frame| frame.widget_data.as_ref())
        {
            result.set("zone_text", text.text.as_str());
        }
        let blips = registry
            .frames_iter()
            .filter(|frame| {
                frame
                    .name
                    .as_deref()
                    .is_some_and(|name| name.starts_with("MinimapBlip"))
            })
            .count();
        result.set("blips", blips as i64);
    }
}

fn rgba_array(pixel: &[u8]) -> PackedInt32Array {
    pixel.iter().map(|&value| i32::from(value)).collect()
}
