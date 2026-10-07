//! In-world MinimapCluster (docs/specs/minimap.md) over the shared cluster screen: the
//! round north-up composite of the local-CASC `world/minimaps/<map>/mapXX_YY.blp` tiles
//! under the player, the facing arrow, quest-giver blips from `QuestGiverStatusMultiple`,
//! creature vignettes (`crate::vignettes`),
//! the subzone text in its PvP colour, the local-time clock and calendar day. Hovering the
//! map shows the zoom buttons; the wheel over it zooms, as `MinimapMixin:OnMouseWheel`.

use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;

use game_engine_core::asset_loader::{AssetLoader, Priority};
use game_engine_core::input_bindings_data::InputAction;
use game_engine_core::minimap_data::{
    AreaCatalog, FACTION_GROUP_ALLIANCE, FACTION_GROUP_HORDE, MinimapView, TileImage, TileKey,
    VignetteRow, ZonePvp, compose, parse_race_faction_groups, parse_vignettes, sample, tile_path,
    tint_quest_areas, zoom_in, zoom_out,
};
use game_engine_session::SessionScreen;
use game_engine_ui_model::game_tooltip::GameTooltip;
use game_engine_ui_model::game_tooltip::hud::zone_tooltip;
use game_engine_ui_model::minimap::{
    ACTION_TOGGLE_WORLD_MAP, ACTION_ZOOM_IN, ACTION_ZOOM_OUT, BlipKind, MINIMAP_ARROW,
    MINIMAP_BLIP_PREFIX, MINIMAP_DISPLAY, MINIMAP_VIGNETTE_PREFIX, MINIMAP_ZONE_TEXT, MinimapBlip,
    MinimapClusterState, TrackingState, cluster_style, group_minimap_blips, minimap_texture_fdids,
    target_minimap_blip, tracking_minimap_blips,
};
use game_engine_ui_model::world_map_view_data::arrow_rotation;
use godot::classes::{InputEvent, InputEventMouseButton, InputEventMouseMotion, Time};
use godot::global::MouseButton;
use godot::prelude::*;
use osso_asset_resolver::CascListfileResolver;
use shared::components::Position;
use shared::protocol::{NpcFlags, QuestGiverStatus};
use ui_toolkit::atlas::{ActiveSkin, thread_skin};
use ui_toolkit::frame::WidgetData;

use crate::background_load::BackgroundLoad;
use crate::{GameClient, frame_error::FrameError, ui::RegistryUi};

/// Composite resolution: the 198-unit Modern map at up to 1.3× UI scale and the 244-unit
/// Forever map at up to 1.05× without upsampling.
const COMPOSITE_PX: u32 = 256;
const DB2_DIR: &str = "db2/12.1.0.69933";

pub(crate) struct Minimap {
    pub(crate) ui: Option<Gd<RegistryUi>>,
    zoom: u8,
    hovered: bool,
    tracking: TrackingState,
    /// Skin the cluster is built under: it shapes the map mask.
    skin: ActiveSkin,
    /// `AreaTable` and `ChrRaces`, loaded from client start.
    catalogs: BackgroundLoad<Result<Catalogs, String>>,
    resolver: Option<CascListfileResolver>,
    /// Tiles extracted and decoded on a worker; the composite shows each once it arrives.
    tile_loader: AssetLoader<(String, TileKey), Option<Tile>>,
    /// Decoded tiles by map directory and key; None where the install has no tile.
    tiles: HashMap<(String, TileKey), Option<Tile>>,
    /// A tile arrived since the composite was drawn.
    tiles_arrived: bool,
    /// Whether each chrome texture FDID is on disk.
    chrome: HashMap<u32, bool>,
    /// Map and view of the current composite, and its pixels.
    drawn: Option<(String, MinimapView, Vec<u8>)>,
    /// Quest objective areas (engine `(x, z)` polygons) tinted into the composite, and
    /// the pixels they cover.
    quest_areas: (Vec<Vec<[f32; 2]>>, usize),
}

struct Catalogs {
    areas: AreaCatalog,
    races: HashMap<u8, u32>,
    vignettes: HashMap<u32, VignetteRow>,
}

struct Tile {
    fdid: u32,
    image: TileImage,
}

impl Minimap {
    pub(crate) fn new(data_root: &std::path::Path) -> Self {
        let catalog_root = data_root.to_owned();
        let tile_root = data_root.to_owned();
        let resolver = crate::assets::creature::local_resolver(data_root);
        Self {
            ui: None,
            zoom: 0,
            hovered: false,
            tracking: TrackingState::default(),
            skin: thread_skin(),
            catalogs: BackgroundLoad::start("minimap-catalogs", move || {
                load_catalogs(&catalog_root)
            }),
            resolver: None,
            tile_loader: AssetLoader::new(
                "minimap-tiles",
                1,
                move |(map, key): &(String, TileKey)| load_tile(&resolver, &tile_root, map, *key),
            ),
            tiles: HashMap::new(),
            tiles_arrived: false,
            chrome: HashMap::new(),
            drawn: None,
            quest_areas: (Vec::new(), 0),
        }
    }

    /// `Vignette.csv` rows, once the catalogs loaded.
    pub(crate) fn vignette_rows(&self) -> Option<&HashMap<u32, VignetteRow>> {
        Some(&self.catalogs.loaded()?.as_ref().ok()?.vignettes)
    }

    /// `AreaTable` name of `area_id` (the quest log's zone headers share the catalog).
    /// `None` while the catalog loads; then the name, `None` for an area it lacks.
    pub(crate) fn area_name(&mut self, area_id: u32) -> Option<Option<String>> {
        let catalogs = self.catalogs.poll()?.as_ref().ok();
        Some(catalogs.and_then(|catalogs| catalogs.areas.name(area_id).map(str::to_owned)))
    }

    /// The view around `position` at the current zoom, clipped to the skin's map mask.
    fn view(&self, position: [f32; 2]) -> MinimapView {
        MinimapView::new(position, self.zoom).masked(cluster_style(self.skin).mask)
    }

    fn free_ui(&mut self) {
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
        self.drawn = None;
        self.hovered = false;
        self.tracking.open = false;
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

    /// Request the tiles of `keys` on `map`, and keep those that arrived; a tile that
    /// failed to decode is reported and stays empty.
    fn receive_tiles(&mut self, map: &str, keys: impl IntoIterator<Item = TileKey>) {
        for key in keys {
            self.tile_loader
                .request((map.to_owned(), key), Priority::Now);
        }
        for (slot, tile) in self.tile_loader.poll() {
            let tile = tile.unwrap_or_else(|error| {
                godot_warn!("{error}");
                None
            });
            self.tiles.insert(slot, tile);
            self.tiles_arrived = true;
        }
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

/// Worker: tile `key` of `map` out of local CASC, decoded; `None` for an ocean or
/// unlisted tile.
fn load_tile(
    resolver: &CascListfileResolver,
    data_root: &std::path::Path,
    map: &str,
    key: TileKey,
) -> Result<Option<Tile>, String> {
    let path = tile_path(map, key);
    let Some(fdid) = resolver.lookup_path(&path) else {
        return Ok(None);
    };
    let cache = data_root.join("textures").join(format!("{fdid}.blp"));
    let Some(file) = resolver.ensure_cached(fdid, &cache) else {
        return Ok(None);
    };
    let rgba = std::fs::read(&file)
        .map_err(|error| error.to_string())
        .and_then(|bytes| game_engine_core::blp::decode_rgba(&bytes))
        .map_err(|error| format!("Minimap tile {path} (FDID {fdid}): {error}"))?;
    Ok(Some(Tile {
        fdid,
        image: TileImage {
            pixels: rgba.pixels,
            width: rgba.width,
            height: rgba.height,
        },
    }))
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
    let (vignettes, vignettes_path) = open(data_root.join(DB2_DIR).join("Vignette.csv"))?;
    Ok(Catalogs {
        areas: AreaCatalog::parse(areas, &areas_path)?,
        races: parse_race_faction_groups(races, &races_path)?,
        vignettes: parse_vignettes(vignettes, &vignettes_path)?,
    })
}

/// Local wall-clock hour, minute and day of month (`timeMgrUseLocalTime` shows local time;
/// the protocol carries no realm time).
pub(crate) fn local_time() -> (u32, u32, u32) {
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
            return Ok(());
        }
        self.apply_minimap_zoom_bindings();
        self.poll_minimap_actions()?;
        Ok(self.sync_minimap()?)
    }

    /// Retail `MINIMAPZOOMIN`/`MINIMAPZOOMOUT` (Bindings_Standard.xml:1378-1383).
    fn apply_minimap_zoom_bindings(&mut self) {
        let input = self.physical_input.gameplay_state(self.keyboard_free());
        let bindings = &self.client_options.bindings;
        if bindings.is_just_pressed(InputAction::MinimapZoomIn, &input) {
            self.minimap.zoom = zoom_in(self.minimap.zoom);
        }
        if bindings.is_just_pressed(InputAction::MinimapZoomOut, &input) {
            self.minimap.zoom = zoom_out(self.minimap.zoom);
        }
    }

    fn poll_minimap_actions(&mut self) -> Result<(), String> {
        let Some(ui) = self.minimap.ui.as_mut() else {
            return Ok(());
        };
        let action = ui.bind_mut().pop_action().to_string();
        match action.as_str() {
            "" => {}
            game_engine_ui_model::launcher::ACTION_OPEN => self.open_launcher()?,
            ACTION_ZOOM_IN => self.minimap.zoom = zoom_in(self.minimap.zoom),
            ACTION_ZOOM_OUT => self.minimap.zoom = zoom_out(self.minimap.zoom),
            ACTION_TOGGLE_WORLD_MAP => {
                // As the M binding: a map failure is reported and closes the map.
                if let Err(error) = self.toggle_world_map() {
                    godot_error!("World map: {error}");
                    self.close_world_map();
                }
            }
            other => {
                if !self.minimap.tracking.toggle_action(other) {
                    return Err(format!("Unknown minimap action: {other}"));
                }
            }
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

    /// `MinimapZoneTextButtonMixin:OnEnter`: the zone, the subzone in its PvP colour and
    /// the world map button text with its key.
    pub(crate) fn zone_text_tooltip(&mut self) -> Option<GameTooltip> {
        let (position, _) = self.minimap_player()?;
        let catalogs = self.minimap.catalogs.loaded()?.as_ref().ok()?;
        let area = self.terrain.area_id_at(position[0], position[1])?;
        let group = self.player_faction_group(catalogs);
        let pvp = catalogs.areas.pvp(area, group);
        // C_PvP.GetZonePVPInfo's factionName: the side controlling the zone.
        let own = match group {
            FACTION_GROUP_ALLIANCE => Some(("Alliance", "Horde")),
            FACTION_GROUP_HORDE => Some(("Horde", "Alliance")),
            _ => None,
        };
        let faction = match pvp {
            ZonePvp::Friendly => own.map(|(own, _)| own),
            ZonePvp::Hostile => own.map(|(_, other)| other),
            _ => None,
        };
        let key = self
            .client_options
            .bindings
            .binding(InputAction::ToggleWorldMap)
            .map(|binding| binding.display());
        Some(zone_tooltip(
            catalogs.areas.zone(area)?,
            catalogs.areas.name(area)?,
            pvp,
            faction,
            key.as_deref(),
        ))
    }

    fn minimap_cluster_state(
        &mut self,
        position: [f32; 2],
        yaw: f32,
    ) -> Result<MinimapClusterState, String> {
        self.minimap.catalogs.poll();
        let catalogs = match self.minimap.catalogs.loaded() {
            Some(Ok(catalogs)) => Some(catalogs),
            Some(Err(error)) => return Err(format!("Minimap catalogs: {error}")),
            // Loading: no zone text yet.
            None => None,
        };
        let area = self.terrain.area_id_at(position[0], position[1]);
        let zone = catalogs.zip(area);
        let zone_text = zone
            .and_then(|(catalogs, area)| catalogs.areas.name(area))
            .unwrap_or_default()
            .to_owned();
        let pvp = zone.map(|(catalogs, area)| {
            catalogs
                .areas
                .pvp(area, self.player_faction_group(catalogs))
        });
        let (hour, minute, day) = local_time();
        // Retail GameTime.lua:1-2: dawn 05:30, dusk 21:00, in minutes.
        const DAWN: u32 = 5 * 60 + 30;
        const DUSK: u32 = 21 * 60;
        let night = !(DAWN..DUSK).contains(&(hour * 60 + minute));
        let view = self.minimap.view(position);
        let mut blips = self.tracking_minimap_blips(&view);
        blips.extend(self.quest_blips(&view));
        blips.extend(group_minimap_blips(
            &self.account.group,
            self.account.session.selected_character_name.as_deref(),
            &view,
            cluster_style(self.minimap.skin).map_size,
        ));
        if let Some(map_id) = self.world_map_id {
            blips.extend(game_engine_ui_model::death_flow::corpse_minimap_blip(
                &self.death_flow,
                &view,
                map_id,
            ));
        }
        if let Some(Ok(catalogs)) = self.minimap.catalogs.loaded() {
            let sightings = crate::vignettes::sightings(&self.replica, &catalogs.vignettes)?;
            blips.extend(crate::vignettes::minimap_blips(&view, &sightings));
        }
        // Target overlay comes last so selecting a member does not hide its indicator.
        blips.extend(self.target_minimap_blip(&view));
        Ok(MinimapClusterState {
            zone_text,
            zone_color: pvp.map_or([1.0, 0.82, 0.0, 1.0], |pvp| pvp.text_color()),
            clock_text: game_engine_core::minimap_data::clock_text(hour, minute),
            calendar_day: Some(day),
            night,
            arrow_rotation: arrow_rotation(yaw),
            zoom_buttons: self.minimap.hovered,
            zoom: self.minimap.zoom,
            blips,
            has_mail: !self.mailbox.session.pending_senders.is_empty(),
            map_texture: None,
            tracking: self.minimap.tracking.clone(),
        })
    }

    fn tracking_minimap_blips(&self, view: &MinimapView) -> Vec<MinimapBlip> {
        let units = self.replica.units().filter_map(|unit| {
            let flags = unit.get::<NpcFlags>()?;
            let position = unit.get::<Position>()?;
            Some((unit.server_id, *flags, [position.x, position.z]))
        });
        tracking_minimap_blips(view, &self.minimap.tracking, units)
    }

    fn target_minimap_blip(&self, view: &MinimapView) -> Option<MinimapBlip> {
        let target = self.targeting_target()?;
        let position = self.replica.unit(target)?.get::<Position>()?;
        target_minimap_blip(target, [position.x, position.z], view)
    }

    /// `QuestNormal` for givers with an available quest, `QuestTurnin` for a reward;
    /// the other statuses draw no minimap blip.
    fn quest_blips(&self, view: &MinimapView) -> Vec<MinimapBlip> {
        let mut blips: Vec<MinimapBlip> = self
            .account
            .quests
            .giver_status
            .iter()
            .filter_map(|(&unit, status)| {
                let kind = match status {
                    QuestGiverStatus::Available(_) => BlipKind::QuestAvailable,
                    QuestGiverStatus::Reward(_) => BlipKind::QuestTurnIn,
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
        let view = self.minimap.view(position);
        self.minimap.receive_tiles(&map, view.tiles());
        let pixel_yards = view.diameter / COMPOSITE_PX as f32;
        let areas = self.minimap_quest_areas();
        if let Some((drawn_map, drawn, _)) = &self.minimap.drawn
            && !self.minimap.tiles_arrived
            && *drawn_map == map
            && self.minimap.quest_areas.0 == areas
            && drawn.diameter == view.diameter
            && (drawn.center[0] - view.center[0]).hypot(drawn.center[1] - view.center[1])
                < pixel_yards / 2.0
        {
            return None;
        }
        self.minimap.tiles_arrived = false;
        let _span = crate::profile::span(|| "minimap.compose".to_owned());
        let minimap = &self.minimap;
        let mut pixels = compose(&view, COMPOSITE_PX, |key| {
            minimap.tile(&map, key).map(|tile| &tile.image)
        });
        let tinted = tint_quest_areas(&view, COMPOSITE_PX, &mut pixels, &areas);
        self.minimap.quest_areas = (areas, tinted);
        self.minimap.drawn = Some((map, view, pixels.clone()));
        Some((COMPOSITE_PX, pixels))
    }

    /// Objective areas of the watched quests on the player's map, as engine `(x, z)`
    /// polygons (world `(x, y)` is engine `(x, -z)`).
    fn minimap_quest_areas(&self) -> Vec<Vec<[f32; 2]>> {
        let Some(map_id) = self.world_map_id else {
            return Vec::new();
        };
        self.account
            .quests
            .watched_objective_areas()
            .into_iter()
            .filter(|poi| poi.map_id == map_id)
            .map(|poi| {
                poi.points
                    .iter()
                    .map(|point| [point.x as f32, -(point.y as f32)])
                    .collect()
            })
            .collect()
    }

    fn sync_minimap(&mut self) -> Result<(), String> {
        let Some((position, yaw)) = self.minimap_player() else {
            return Ok(());
        };
        // Frames of one level draw in creation order, so another skin's cluster and its
        // composite are built afresh instead of patched.
        let skin = thread_skin();
        if self.minimap.skin != skin {
            self.minimap.free_ui();
            self.minimap.skin = skin;
        }
        let span = crate::profile::span(|| "minimap.cluster_state".to_owned());
        let state = self.minimap_cluster_state(position, yaw)?;
        let data_root = self.data_root.clone();
        drop(span);
        let span = crate::profile::span(|| "minimap.chrome".to_owned());
        self.minimap.cache_chrome(
            &data_root,
            &minimap_texture_fdids(&state, self.minimap.skin),
        );
        drop(span);
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

    /// The map in physical pixels: centre and half its side (the radius of a round map).
    fn minimap_extent(&self) -> Option<(Vector2, f32)> {
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

    fn minimap_hit(&self, name: &str, point: Vector2) -> bool {
        let Some(ui) = self.minimap.ui.as_ref() else {
            return false;
        };
        let ui = ui.bind();
        let Some(registry) = ui.registry() else {
            return false;
        };
        let rect = registry
            .get_by_name(name)
            .and_then(|id| registry.get(id))
            .and_then(|frame| frame.layout_rect.as_ref());
        let Some(rect) = rect else { return false };
        let point = point / self.effective_ui_scale();
        point.x >= rect.x
            && point.x <= rect.x + rect.width
            && point.y >= rect.y
            && point.y <= rect.y + rect.height
    }

    fn minimap_tracking_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if !button.is_pressed() || button.get_button_index() != MouseButton::LEFT {
            return false;
        }
        let point = button.get_position();
        if self.minimap_hit("MinimapClusterTrackingBackground", point) {
            self.minimap.tracking.open = !self.minimap.tracking.open;
            return true;
        }
        if self.minimap.tracking.open && !self.minimap_hit("MinimapTrackingMenu", point) {
            self.minimap.tracking.open = false;
            return true;
        }
        false
    }

    /// Pointer over the map: hover shows the zoom buttons (`MinimapMixin:OnEnter`), and
    /// the wheel zooms instead of the camera. Returns whether `event` was consumed.
    pub(super) fn minimap_pointer(&mut self, event: &Gd<InputEvent>) -> bool {
        if self.minimap_tracking_pointer(event) {
            return true;
        }
        let Some((centre, radius)) = self.minimap_extent() else {
            return false;
        };
        let scale = self.effective_ui_scale();
        let mask = cluster_style(self.minimap.skin).mask;
        let on_map = |point: Vector2| {
            let offset = (point - centre) / (2.0 * radius);
            mask.contains(offset.x, offset.y)
        };
        if let Ok(motion) = event.clone().try_cast::<InputEventMouseMotion>() {
            let point = motion.get_position();
            // ZoomHitArea 40×40 at CENTER (+77, −77) keeps the buttons while over them.
            let hit_area = centre + Vector2::new(77.0, 77.0) * scale;
            let over_buttons = (point.x - hit_area.x).abs() <= 20.0 * scale
                && (point.y - hit_area.y).abs() <= 20.0 * scale;
            self.minimap.hovered = on_map(point) || over_buttons;
            return false;
        }
        let Ok(button) = event.clone().try_cast::<InputEventMouseButton>() else {
            return false;
        };
        if !button.is_pressed() || !on_map(button.get_position()) {
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
        result.set("tracking_open", self.minimap.tracking.open);
        let tracking_enabled: Array<bool> = self.minimap.tracking.enabled.iter().copied().collect();
        result.set("tracking_enabled", &tracking_enabled);
        if let Some((_, yaw)) = self.minimap_player() {
            result.set("facing_yaw", yaw);
        }
        let mut areas = VarArray::new();
        for area in &self.minimap.quest_areas.0 {
            let points: PackedVector2Array =
                area.iter().map(|[x, z]| Vector2::new(*x, *z)).collect();
            areas.push(&points.to_variant());
        }
        result.set("quest_areas", &areas);
        result.set("quest_area_pixels", self.minimap.quest_areas.1 as i64);
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
        let count = |prefix: &str| {
            registry
                .frames_iter()
                .filter(|frame| {
                    frame
                        .name
                        .as_deref()
                        .is_some_and(|name| name.starts_with(prefix))
                })
                .count() as i64
        };
        result.set("blips", count(MINIMAP_BLIP_PREFIX));
        result.set("vignettes", count(MINIMAP_VIGNETTE_PREFIX));
        result.set(
            "members",
            count(game_engine_ui_model::minimap::MINIMAP_MEMBER_PREFIX),
        );
        result.set(
            "targets",
            count(game_engine_ui_model::minimap::MINIMAP_TARGET_PREFIX),
        );
        result.set("tracked", count("MinimapTrackedUnit"));
    }
}

fn rgba_array(pixel: &[u8]) -> PackedInt32Array {
    pixel.iter().map(|&value| i32::from(value)).collect()
}
