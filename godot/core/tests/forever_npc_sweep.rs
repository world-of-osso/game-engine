//! Offline local-data sweep. Native pure policies are imported, not approximated.
#[path = "../../rust/src/assets/appearance_pixels.rs"]
mod appearance_pixels;
#[path = "../../rust/src/game/equipment/equipment_appearance_data.rs"]
pub mod equipment_appearance_data;
#[path = "../../rust/src/game/creatures/npc_gear_data.rs"]
pub mod npc_gear_data;
pub use game_engine_core::{asset, outfit_data};

use game_engine_core::{
    blp,
    creature_display_data::query_display,
    geoset_visibility_data::apply_exact_geoset_overrides,
    m2, m2_material,
    npc_appearance_assets::{load_compositor, load_customization_db},
    npc_appearance_data::query_authored_npc_appearance,
    npc_appearance_selection_data::{NpcTexturePixels, npc_geoset_visible, select_npc_choices},
};
use rusqlite::{Connection, OpenFlags};
use std::{
    collections::{BTreeSet, HashMap, HashSet},
    path::{Path, PathBuf},
};

const WORLD: &str = "skyborn-handoff/2026-10-07-c1cdf6e/server/world.db";
const SPAWNS: &str = "SELECT DISTINCT m.CreatureDisplayID FROM content_creature c
 JOIN content_creature_template_model m ON m.CreatureID IN (c.id1,c.id2,c.id3)
 WHERE c.map=2991 UNION SELECT DISTINCT modelid FROM content_creature
 WHERE map=2991 AND modelid>0 ORDER BY 1";

fn data_root() -> PathBuf {
    std::env::var_os("NPCSWEEP_DATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data"))
}
fn open(path: &Path) -> Connection {
    Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap()
}
fn pixels(data: &Path, fdid: u32) -> Result<NpcTexturePixels, String> {
    let path = data.join(format!("textures/{fdid}.blp"));
    let bytes = std::fs::read(&path).map_err(|e| format!("FDID {fdid} {}: {e}", path.display()))?;
    let image = blp::decode_rgba(&bytes).map_err(|e| format!("FDID {fdid}: {e}"))?;
    Ok((image.pixels, image.width, image.height))
}
fn model(data: &Path, fdid: u32) -> Result<m2::Model, String> {
    let read = |name: String| {
        std::fs::read(data.join("models").join(&name)).map_err(|e| format!("{name}: {e}"))
    };
    let bytes = read(format!("{fdid}.m2"))?;
    let refs = m2::parse_asset_references(&bytes)?;
    let skin_fdid = *refs.skin_fdids.first().ok_or("no primary SFID")?;
    // The native cache aliases the primary SFID to <model>00.skin.
    let skin = read(format!("{fdid}00.skin"))?;
    let skeleton = refs
        .skeleton_fdid
        .map(|_| read(format!("{fdid}.skel")))
        .transpose()?;
    m2::parse_model_with_skeleton(&bytes, &skin, skeleton.as_deref(), |anim| {
        read(format!("{anim}.anim")).ok()
    })
    .map_err(|e| format!("model FDID {fdid}, skin FDID {skin_fdid}: {e}"))
}

struct Sweep {
    data: PathBuf,
    profiles: Connection,
    displays: Connection,
    customization: game_engine_core::customization_data::CustomizationDb,
    compositor: game_engine_core::char_texture_data::CharTextureData,
    gear: npc_gear_data::NpcGearData,
    outfit: outfit_data::OutfitData,
}
struct Prepared {
    textures: HashMap<u32, NpcTexturePixels>,
    inactive: HashSet<u32>,
    selected: Vec<(u16, u16)>,
    authored: Vec<(u16, u16)>,
    armor: equipment_appearance_data::ResolvedEquipmentAppearance,
}
impl Sweep {
    fn load(data: PathBuf) -> Self {
        Self {
            profiles: open(&data.join("cache/npc_appearance.sqlite")),
            displays: open(&data.join("cache/creature_display.sqlite")),
            customization: load_customization_db(&data).unwrap(),
            compositor: load_compositor(&data).unwrap(),
            gear: npc_gear_data::NpcGearData::load(&data.join("db2/12.1.0.69933")).unwrap(),
            outfit: outfit_data::OutfitData::load(&data),
            data,
        }
    }
    fn prepare(&self, id: u32) -> Result<Option<Prepared>, String> {
        let Some(appearance) = query_authored_npc_appearance(&self.profiles, id)? else {
            return Ok(None);
        };
        let armor = self.gear.display_armor(id)?;
        let armor = if appearance.baked_texture_fdid.is_some() {
            equipment_appearance_data::load_baked_equipment_appearance(
                &armor,
                &self.outfit,
                appearance.race,
                appearance.sex,
            )?
        } else {
            equipment_appearance_data::resolve_equipment_appearance(
                &armor,
                &self.outfit,
                appearance.race,
                appearance.sex,
            )?
        };
        let mut selected = select_npc_choices(&appearance, &self.customization)?;
        for &group in &armor.hidden_character_geoset_groups {
            selected.geosets.retain(|(active, _)| *active != group);
            let variant = if group == 0 {
                self.customization
                    .scalp_fallback_hair_geoset(appearance.race, appearance.sex)
                    .unwrap_or(1)
            } else {
                1
            };
            selected.geosets.push((group, variant));
        }
        let layout_id = self
            .customization
            .layout_id(appearance.race, appearance.sex)
            .ok_or("missing texture layout")?;
        let layout = self
            .compositor
            .layout(layout_id)
            .ok_or("missing compositor layout")?;
        let default = asset::m2_texture::default_fdid_for_type(
            1,
            layout.width == 2048 && layout.height == 1024,
            &[0, 0, 0],
        )
        .unwrap();
        let mut decoded = HashMap::new();
        for fdid in selected
            .materials
            .iter()
            .map(|(_, fdid)| *fdid)
            .collect::<BTreeSet<_>>()
        {
            decoded.insert(fdid, pixels(&self.data, fdid)?);
        }
        // Same optional default-atlas behavior as the production reader, with a receipt.
        if let std::collections::hash_map::Entry::Vacant(entry) = decoded.entry(default) {
            match pixels(&self.data, default) {
                Ok(image) => {
                    entry.insert(image);
                }
                Err(error) => eprintln!("display {id}: {error}"),
            }
        }
        let composed = self
            .compositor
            .composite_model_textures_with(&selected.materials, &[], layout_id, default, |fdid| {
                decoded.get(&fdid).cloned()
            })
            .ok_or("cannot composite model textures")?;
        let bake = appearance
            .baked_texture_fdid
            .map(|fdid| pixels(&self.data, fdid))
            .transpose()?;
        let textures = appearance_pixels::compose_replacement_pixels(
            &self.compositor,
            &selected.materials,
            layout_id,
            composed,
            bake,
            &decoded,
        )?;
        Ok(Some(Prepared {
            textures,
            inactive: appearance_pixels::inactive_npc_texture_types(
                &self.compositor,
                &selected.materials,
                layout_id,
            ),
            selected: selected.geosets,
            authored: appearance.geosets,
            armor,
        }))
    }
    fn errors(&self, id: u32) -> Vec<(String, String)> {
        let mut errors = Vec::new();
        let prepared = match self.prepare(id) {
            Ok(p) => p,
            Err(e) => {
                errors.push(("appearance".into(), e));
                None
            }
        };
        let display = match query_display(&self.displays, id) {
            Ok(Some(d)) => d,
            other => {
                errors.push(("display".into(), format!("{other:?}")));
                return errors;
            }
        };
        let model = match model(&self.data, display.model_fdid) {
            Ok(m) => m,
            Err(e) => {
                errors.push(("model".into(), e));
                return errors;
            }
        };
        let batches = match m2::resolve_render_batches(&model, &display.skin_fdids, false, |_| None)
        {
            Ok(b) => b,
            Err(e) => {
                errors.push(("batches".into(), e));
                return errors;
            }
        };
        for batch in batches {
            let result = (|| {
                let binding = m2_material::batch_binding(
                    &model,
                    &model.batches[batch.source_unit_index],
                    &display.skin_fdids,
                )?;
                let Some(p) = &prepared else { return Ok(()) };
                let visible = !p
                    .armor
                    .hidden_character_geoset_ids
                    .contains(&batch.mesh_part_id)
                    && npc_geoset_visible(batch.mesh_part_id, &p.selected, &p.authored);
                let visible = apply_exact_geoset_overrides(
                    batch.mesh_part_id,
                    visible,
                    &p.armor.outfit.geoset_overrides,
                );
                let mut missing = Vec::new();
                for (&kind, fdid) in binding.texture_types.iter().zip(&binding.textures) {
                    if kind != 0 && p.textures.contains_key(&kind) {
                        continue;
                    }
                    if let Some(fdid) = fdid {
                        let path = self.data.join(format!("textures/{fdid}.blp"));
                        match std::fs::read(path) {
                            Ok(bytes) => {
                                blp::decode_rgba(&bytes)
                                    .map_err(|e| format!("FDID {fdid}: {e}"))?;
                            }
                            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                missing.push(*fdid)
                            }
                            Err(e) => return Err(format!("FDID {fdid}: {e}")),
                        }
                    }
                }
                appearance_pixels::npc_pass_active(
                    &binding.texture_types,
                    &binding.textures,
                    &p.inactive,
                    &p.textures,
                    visible,
                    &missing,
                )?;
                Ok(())
            })();
            if let Err(e) = result {
                errors.push((format!("Batch{}", batch.source_unit_index), e));
            }
        }
        errors
    }
}

#[test]
#[ignore = "requires full local Skyborn data; see npc-appearance.md"]
fn sweep_skyborn_forever_npc_appearances() {
    let data = data_root();
    let world = open(&data.join(WORLD));
    let forever = data.join("db2/1.60.1.70205/CreatureDisplayInfo.csv");
    let mut source_ids = HashSet::new();
    game_engine_core::csv_util::read_numeric_rows(&forever, ["ID"], |[id]| {
        source_ids.insert(id as u32);
    })
    .unwrap();
    let mut stmt = world.prepare(SPAWNS).unwrap();
    let ids: Vec<u32> = stmt
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .filter(|id| source_ids.contains(id))
        .collect();
    assert!(
        ids.contains(&136974),
        "historical strict Batch52 display absent from source query"
    );
    let sweep = Sweep::load(data);
    let mut report = String::from("display\tbatch\treason\n");
    let mut failures = 0;
    for id in &ids {
        let errors = sweep.errors(*id);
        if !errors.is_empty() {
            failures += 1;
        }
        for (batch, reason) in errors {
            report.push_str(&format!(
                "{id}\t{batch}\t{}\n",
                reason.replace(['\n', '\t'], " ")
            ));
        }
    }
    let out = PathBuf::from(
        std::env::var_os("NPCSWEEP_OUT").expect("set NPCSWEEP_OUT to evidence directory"),
    );
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("errors.tsv"), report).unwrap();
    std::fs::write(
        out.join("display-ids.txt"),
        ids.iter().map(|id| format!("{id}\n")).collect::<String>(),
    )
    .unwrap();
    std::fs::write(out.join("spawn-query.sql"), SPAWNS).unwrap();
    println!(
        "SWEEP displays={} failed_displays={failures}; display136974 included; report={}",
        ids.len(),
        out.display()
    );
    // The diagnostic succeeds after recording all failures, not after hiding them.
    assert!(!ids.is_empty());
}
