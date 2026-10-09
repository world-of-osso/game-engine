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
    npc_appearance_assets::NpcAppearanceCatalogs,
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
    let read = |name: String, source_fdid: u32| {
        std::fs::read(data.join("models").join(&name))
            .map_err(|e| format!("model dependency FDID {source_fdid} at models/{name}: {e}"))
    };
    let bytes = read(format!("{fdid}.m2"), fdid)?;
    let refs = m2::parse_asset_references(&bytes)?;
    let skin_fdid = *refs.skin_fdids.first().ok_or("no primary SFID")?;
    // The native cache aliases the primary SFID to <model>00.skin.
    let skin = read(format!("{fdid}00.skin"), skin_fdid)?;
    let skeleton = refs
        .skeleton_fdid
        .map(|skeleton_fdid| read(format!("{fdid}.skel"), skeleton_fdid))
        .transpose()?;
    m2::parse_model_with_skeleton(&bytes, &skin, skeleton.as_deref(), |anim| {
        read(format!("{anim}.anim"), anim).ok()
    })
    .map_err(|e| format!("model FDID {fdid}, skin FDID {skin_fdid}: {e}"))
}

struct Sweep {
    data: PathBuf,
    profiles: Connection,
    displays: Connection,
    catalogs: NpcAppearanceCatalogs,
    gear: npc_gear_data::NpcGearData,
    outfit: outfit_data::OutfitData,
}
#[derive(Default)]
struct DisplayAudit {
    errors: Vec<(String, String)>,
    appearance_ready: bool,
    model_ready: bool,
    material_batches: usize,
    strict_npc_batches: usize,
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
            catalogs: NpcAppearanceCatalogs::load(&data).unwrap(),
            gear: npc_gear_data::NpcGearData::load(&data.join("db2/12.1.0.69933")).unwrap(),
            outfit: outfit_data::OutfitData::load(&data),
            data,
        }
    }
    fn prepare(&mut self, id: u32) -> Result<Option<Prepared>, String> {
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
        let (customization, compositor) =
            self.catalogs
                .load_for_display(id, appearance.race, appearance.sex)?;
        let mut selected = select_npc_choices(&appearance, customization)?;
        for &group in &armor.hidden_character_geoset_groups {
            selected.geosets.retain(|(active, _)| *active != group);
            let variant = if group == 0 {
                customization
                    .scalp_fallback_hair_geoset(appearance.race, appearance.sex)
                    .unwrap_or(1)
            } else {
                1
            };
            selected.geosets.push((group, variant));
        }
        let layout_id = customization
            .layout_id(appearance.race, appearance.sex)
            .ok_or("missing texture layout")?;
        let layout = compositor
            .layout(layout_id)
            .ok_or("missing compositor layout")?;
        let default = asset::m2_texture::default_fdid_for_type(
            1,
            layout.width == 2048 && layout.height == 1024,
            &[0, 0, 0],
        )
        .unwrap();
        let required = selected
            .materials
            .iter()
            .map(|(_, fdid)| *fdid)
            .chain(appearance.baked_texture_fdid)
            .chain(armor.merged_cape_texture_fdid)
            .chain(armor.texture_fdids.iter().copied())
            .chain(
                armor
                    .runtime_models
                    .iter()
                    .flat_map(|model| model.skin_fdids)
                    .filter(|fdid| *fdid != 0),
            )
            .chain(
                armor
                    .runtime_models
                    .iter()
                    .flat_map(|model| model.texture_replacements.iter().map(|(_, fdid)| *fdid)),
            )
            .collect::<BTreeSet<_>>();
        let mut decoded = HashMap::new();
        let mut dependency_errors = Vec::new();
        for fdid in required {
            match pixels(&self.data, fdid) {
                Ok(image) => {
                    decoded.insert(fdid, image);
                }
                Err(error) => dependency_errors.push(error),
            }
        }
        for item in &armor.runtime_models {
            if let Err(error) = model(&self.data, item.fdid) {
                dependency_errors.push(format!("Equipment {:?}: {error}", item.slot));
            }
        }
        if !dependency_errors.is_empty() {
            return Err(dependency_errors.join("\n"));
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
        let composed = compositor
            .composite_model_textures_with(&selected.materials, &[], layout_id, default, |fdid| {
                decoded.get(&fdid).cloned()
            })
            .ok_or("cannot composite model textures")?;
        let bake = appearance
            .baked_texture_fdid
            .map(|fdid| decoded[&fdid].clone());
        let textures = appearance_pixels::compose_replacement_pixels(
            compositor,
            &selected.materials,
            layout_id,
            composed,
            bake,
            &decoded,
            armor.merged_cape_texture_fdid,
        )?;
        Ok(Some(Prepared {
            textures,
            inactive: appearance_pixels::inactive_npc_texture_types(
                compositor,
                &selected.materials,
                layout_id,
            ),
            selected: selected.geosets,
            authored: appearance.geosets,
            armor,
        }))
    }
    fn audit_display(&mut self, id: u32) -> DisplayAudit {
        let mut audit = DisplayAudit::default();
        let prepared = match self.prepare(id) {
            Ok(p) => {
                audit.appearance_ready = true;
                p
            }
            Err(e) => {
                audit.errors.extend(
                    e.lines()
                        .map(|reason| ("appearance".into(), reason.to_owned())),
                );
                None
            }
        };
        let display = match query_display(&self.displays, id) {
            Ok(Some(d)) => d,
            other => {
                audit.errors.push(("display".into(), format!("{other:?}")));
                return audit;
            }
        };
        for fdid in display
            .skin_fdids
            .into_iter()
            .filter(|fdid| *fdid != 0)
            .collect::<BTreeSet<_>>()
        {
            if let Err(error) = pixels(&self.data, fdid) {
                audit.errors.push(("variation".into(), error));
            }
        }
        let model = match model(&self.data, display.model_fdid) {
            Ok(model) => {
                audit.model_ready = true;
                model
            }
            Err(error) => {
                audit.errors.push(("model".into(), error));
                return audit;
            }
        };
        let batches = match m2::resolve_render_batches(&model, &display.skin_fdids, true, |_| None)
        {
            Ok(b) => b,
            Err(e) => {
                audit.errors.push(("batches".into(), e));
                return audit;
            }
        };
        for batch in batches {
            audit.material_batches += 1;
            let result = (|| {
                let binding = m2_material::batch_binding(
                    &model,
                    &model.batches[batch.source_unit_index],
                    &display.skin_fdids,
                )?;
                let mut missing = Vec::new();
                for (&kind, fdid) in binding.texture_types.iter().zip(&binding.textures) {
                    if kind != 0
                        && prepared
                            .as_ref()
                            .is_some_and(|p| p.textures.contains_key(&kind))
                    {
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
                let Some(p) = &prepared else {
                    if !missing.is_empty() {
                        return Err(format!(
                            "missing file-backed batch texture FDIDs {missing:?}"
                        ));
                    }
                    return Ok(());
                };
                audit.strict_npc_batches += 1;
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
                audit
                    .errors
                    .push((format!("Batch{}", batch.source_unit_index), e));
            }
        }
        audit
    }
}

#[test]
fn forever_display137165_resolves_authored_handlebars_without_changing_retail() {
    let data = data_root();
    let profiles = open(&data.join("cache/npc_appearance.sqlite"));
    let appearance = query_authored_npc_appearance(&profiles, 137165)
        .unwrap()
        .unwrap();
    assert_eq!((appearance.race, appearance.sex), (7, 0));
    assert!(appearance.choice_ids.contains(&78872));
    let mut catalogs = NpcAppearanceCatalogs::load(&data).unwrap();
    let (customization, _) = catalogs
        .load_for_display(137165, appearance.race, appearance.sex)
        .unwrap();
    let selected = select_npc_choices(&appearance, customization).unwrap();
    assert!(
        selected.geosets.contains(&(1, 5)) && selected.geosets.contains(&(3, 4)),
        "{:#?}",
        selected.geosets
    );
    let retail = query_authored_npc_appearance(&profiles, 825)
        .unwrap()
        .unwrap();
    let (retail_db, _) = catalogs
        .load_for_display(825, retail.race, retail.sex)
        .unwrap();
    assert!(select_npc_choices(&retail, retail_db).is_ok());
}

#[test]
fn retail_display825_rejects_forever_only_choices_even_for_skyborne_race() {
    let data = data_root();
    let mut catalogs = NpcAppearanceCatalogs::load(&data).unwrap();
    let (retail, _) = catalogs.load_for_display(825, 95, 1).unwrap();
    let invalid = game_engine_core::npc_appearance_data::AuthoredNpcAppearance {
        race: 95,
        sex: 1,
        class: 0,
        baked_texture_fdid: None,
        choice_ids: vec![61137],
        geosets: vec![],
    };
    assert!(
        select_npc_choices(&invalid, retail).is_err(),
        "Retail display825 must reject a corrupted profile borrowing Forever61137"
    );
}

#[test]
fn forever_display136974_binds_its_authored_cape_to_type2() {
    let data = data_root();
    let gear = npc_gear_data::NpcGearData::load(&data.join("db2/12.1.0.69933")).unwrap();
    let armor = gear.display_armor(136974).unwrap();
    let outfit = outfit_data::OutfitData::load(&data);
    let resolved =
        equipment_appearance_data::load_baked_equipment_appearance(&armor, &outfit, 95, 0).unwrap();
    assert_eq!(resolved.merged_cape_texture_fdid, Some(7734308));
    assert!(resolved.outfit.geoset_overrides.contains(&(15, 2)));
    // Authentic CDI/Extra/armor/material joins; tiny pixel fixture because the local
    // bake/body/cape bytes are absent. This is not a native Batch52/rendering oracle.
    let cape = (vec![17, 61, 113, 255], 1, 1);
    let compositor = game_engine_core::char_texture_data::CharTextureData::from_parts(
        Vec::new(),
        HashMap::new(),
        HashMap::new(),
    );
    let composed = game_engine_core::char_texture_data::CompositedModelTextures {
        body: (vec![101, 102, 103, 255], 1, 1),
        head: None,
        hair: None,
    };
    let decoded = HashMap::from([(7734308, cape.clone())]);
    let textures = appearance_pixels::compose_replacement_pixels(
        &compositor,
        &[],
        201,
        composed.clone(),
        None,
        &decoded,
        resolved.merged_cape_texture_fdid,
    )
    .unwrap();
    assert_eq!(
        textures.get(&2),
        Some(&cape),
        "display136974 cape lost before strict type2 validation"
    );
    assert!(
        appearance_pixels::npc_pass_active(&[2], &[None], &HashSet::new(), &textures, true, &[])
            .unwrap()
    );
    assert_eq!(textures.get(&1), Some(&composed.body));
    let missing = appearance_pixels::compose_replacement_pixels(
        &compositor,
        &[],
        201,
        composed,
        None,
        &HashMap::new(),
        resolved.merged_cape_texture_fdid,
    )
    .unwrap_err();
    assert_eq!(missing, "missing authored NPC cape texture FDID 7734308");
    assert!(
        appearance_pixels::npc_pass_active(
            &[2],
            &[None],
            &HashSet::new(),
            &HashMap::<u32, ()>::new(),
            true,
            &[]
        )
        .is_err()
    );
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
    let mut sweep = Sweep::load(data);
    let mut report = String::from("display\tbatch\treason\n");
    let mut failures = 0;
    let mut coverage = String::from(
        "display\tappearance_ready\tmodel_ready\tmaterial_batches\tstrict_npc_batches\n",
    );
    for id in &ids {
        let audit = sweep.audit_display(*id);
        coverage.push_str(&format!(
            "{id}\t{}\t{}\t{}\t{}\n",
            audit.appearance_ready,
            audit.model_ready,
            audit.material_batches,
            audit.strict_npc_batches
        ));
        if !audit.errors.is_empty() {
            failures += 1;
        }
        for (batch, reason) in audit.errors {
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
    std::fs::write(out.join("coverage.tsv"), coverage).unwrap();
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
