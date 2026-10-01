//! Unit and world tooltips (docs/specs/unit-tooltip.md): the unit a unit frame shows or the
//! one the camera ray through the cursor picks, as Bevy `rendering/ui/unit_hover.rs` and
//! `scenes/tooltip_frame/unit_sources.rs` read it; game objects show their name. Each NPC
//! entry is asked from the server once (`CreatureTooltipQuery`) and its answer cached.

use game_engine_ui_model::char_create_data::{class_by_id, race_by_id};
use game_engine_ui_model::game_tooltip::GameTooltip;
use game_engine_ui_model::game_tooltip::unit::{
    NpcTooltipInput, PlayerTooltipInput, npc_tooltip, player_tooltip,
};
use game_engine_ui_model::tooltip_presentation::{TOOLTIP_WHITE, TooltipPresentation};
use godot::prelude::*;
use shared::components::{GuildMembership, Npc, Player, UnitLevel};
use shared::level_scaling::{LevelScaling, level_for_viewer};
use shared::protocol::GameObjectInfo;

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::replicated::UnitFields;
use crate::tooltips::{HoveredFrame, HoveredTooltip, named_ancestor};
use crate::unit_pick::pick_unit;

impl GameClient {
    /// The unit a hovered unit frame cluster shows (`PlayerFrame`, `TargetFrame`).
    pub(crate) fn unit_frame_unit(&self, hit: &HoveredFrame) -> Option<u64> {
        let ui = hit.ui.bind();
        let (_, root) = named_ancestor(ui.registry()?, hit.frame, |frame| {
            match frame.name.as_deref()? {
                "PlayerFrame" => Some(true),
                "TargetFrame" => Some(false),
                _ => None,
            }
        })?;
        if root {
            self.world.local_player_id()
        } else {
            self.targeting_target()
        }
    }

    /// With no UI under the cursor: the unit or game object the cursor ray picks.
    pub(crate) fn world_tooltip(&mut self) -> Result<Option<HoveredTooltip>, FrameError> {
        let Some(viewport) = self.base().get_viewport() else {
            return Ok(None);
        };
        if viewport.gui_get_hovered_control().is_some() {
            return Ok(None);
        }
        let Some(camera) = viewport.get_camera_3d() else {
            return Ok(None);
        };
        let Some(id) = pick_unit(&camera, Vector2::from_array(self.physical_input.pointer()))
        else {
            return Ok(None);
        };
        self.request_creature_tooltip(id)?;
        Ok(self.unit_tooltip(id).map(HoveredTooltip::text))
    }

    /// `CreatureTooltipQuery` the first time an NPC entry is hovered.
    fn request_creature_tooltip(&mut self, id: u64) -> Result<(), FrameError> {
        let Some(entry) = self
            .replica
            .unit(id)
            .and_then(|unit| unit.get::<Npc>())
            .map(|npc| npc.template_id)
        else {
            return Ok(());
        };
        if self.tooltips.take_unasked(entry) {
            self.account.send_creature_tooltip_query(entry)?;
        }
        Ok(())
    }

    /// The unit tooltip of `id`: player, NPC or game object lines.
    pub(crate) fn unit_tooltip(&mut self, id: u64) -> Option<GameTooltip> {
        let unit = self.replica.unit(id)?;
        if let Some(object) = unit.get::<GameObjectInfo>() {
            return Some(game_object_tooltip(&object.name));
        }
        let level = self.displayed_level(id);
        let reaction = self.reaction_to(id);
        let unit = self.replica.unit(id)?;
        if let Some(player) = unit.get::<Player>() {
            return Some(player_tooltip(&PlayerTooltipInput {
                name: &player.name,
                reaction,
                guild: unit
                    .get::<GuildMembership>()
                    .map(|guild| guild.guild_name.as_str()),
                level,
                race: race_by_id(player.race).map_or("Unknown", |race| race.name),
                class: class_by_id(player.class).map_or("Unknown", |class| class.name),
            }));
        }
        let npc = unit.get::<Npc>()?.clone();
        let faction = self.unit_faction_name(id);
        Some(npc_tooltip(
            &NpcTooltipInput {
                entry: npc.template_id,
                name: &npc.name,
                reaction,
                level,
                faction: faction.as_deref(),
                data: self.tooltips.creature(npc.template_id),
            },
            self.tooltips.appearances(),
        ))
    }

    /// The level the target frame shows: a tuned creature's level against the viewer
    /// (`UnitEffectiveLevel`), otherwise its replicated level.
    fn displayed_level(&self, id: u64) -> Option<u8> {
        let unit = self.replica.unit(id)?;
        let level = *unit.get::<UnitLevel>()?;
        let viewer = self.player_level().map_or(level.0, |level| level as u8);
        Some(level_for_viewer(level, unit.get::<LevelScaling>(), viewer))
    }

    /// The reputation faction of the unit's `FactionTemplate` ("Stormwind").
    fn unit_faction_name(&mut self, id: u64) -> Option<String> {
        let template = self.replica.unit(id)?.faction_template()?;
        let faction = self
            .nameplates
            .templates(&self.data_root)
            .ok()?
            .get(&template)?
            .faction;
        let data_root = self.data_root.clone();
        let names = self.tooltips.faction_names(&data_root)?;
        names.get(faction).map(str::to_owned)
    }
}

/// A game object's name (`C_TooltipInfo.GetWorldCursor` for objects).
fn game_object_tooltip(name: &str) -> GameTooltip {
    GameTooltip::new(
        TooltipPresentation {
            title: name.to_owned(),
            title_color: TOOLTIP_WHITE,
            ..TooltipPresentation::hidden()
        },
        None,
    )
}
