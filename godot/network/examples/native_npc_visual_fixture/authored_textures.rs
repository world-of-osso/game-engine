//! Independent DB2/BLP oracle for real authored displays replicated by the fixture.
use super::*;
use game_engine_core::{
    asset::m2_texture,
    blp,
    char_texture_data::CharTextureData,
    m2, m2_material,
    npc_appearance_assets::{load_compositor, load_customization_db},
    npc_appearance_data::query_authored_npc_appearance,
    npc_appearance_selection_data::{select_npc_choices, select_npc_type6_texture},
};
use std::{collections::HashMap, io::Write};

type Pixels = (Vec<u8>, u32, u32);
// Real content_creature/template_model rows; fixture relocates them to its owned map.
const CASES: [(u32, u32, &str); 8] = [
    (825, 3728, "Dark Strand Adept"),
    (1322, 3322, "Kaja"),
    (1285, 2079, "Conservator Ilthalaine"),
    (90209, 149131, "Apprentice Mage"),
    (110154, 198506, "Krenzen"),
    (150, 3833, "Cenarion Vindicator"),
    (35297, 46785, "Lord Cannon"),
    (110189, 198551, "Thaza"),
];

pub(super) fn stage(project: &FixtureProject) -> Result<(), String> {
    let checkout = fixture_support::checkout_root_from_executable("native_npc_visual_fixture")?;
    let source = checkout.join("data");
    let data = project.root.join("data");
    for name in [
        "npc_appearance.sqlite",
        "customization-v4.sqlite",
        "char_texture-v2.sqlite",
        "creature_display.sqlite",
    ] {
        fixture_data::backup_database(
            &source.join("cache").join(name),
            &data.join("cache").join(name),
        )?;
    }
    execute_fixture_sql(
        &data.join("cache/npc_appearance.sqlite"),
        "INSERT OR REPLACE INTO display_coverage VALUES (910010,0);",
    )?;
    execute_fixture_sql(
        &data.join("cache/creature_display.sqlite"),
        "INSERT OR REPLACE INTO creature_displays (display_id,model_fdid,skin_fdid_0,skin_fdid_1,skin_fdid_2,skin_fdid_3,scale_milli) VALUES (910010,910010,910001,0,0,0,1500);",
    )?;
    for name in [
        "ChrRaceXChrModel.csv",
        "ChrRaces.csv",
        "ChrCustomizationReq.csv",
        "ChrCustomizationReqChoice.csv",
        "equipment_transforms.ron",
    ] {
        fs::copy(source.join(name), data.join(name)).map_err(|e| e.to_string())?;
    }
    let output = project.root.join("oracle");
    fs::create_dir_all(&output).map_err(|e| e.to_string())?;
    let db = load_customization_db(&source)?;
    let compositor = load_compositor(&source)?;
    let profiles = rusqlite::Connection::open_with_flags(
        source.join("cache/npc_appearance.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| e.to_string())?;
    let mut manifest = String::new();
    fs::write(output.join("image-formats.tsv"), "").map_err(|error| error.to_string())?;
    for (display, _, _) in CASES {
        let textures = match query_authored_npc_appearance(&profiles, display)? {
            Some(profile) => {
                let selected = select_npc_choices(&profile, &db)?;
                let layout = db
                    .layout_id(profile.race, profile.sex)
                    .ok_or("missing real layout")?;
                oracle_textures(&source, &compositor, &profile, &selected.materials, layout)?
            }
            None => {
                write_creature_gpu_oracle(&source, &output, display)?;
                read_creature_skin_oracle(&source, display)?
            }
        };
        write_display_oracle(&source, &output, display, &textures, &mut manifest)?;
    }
    fs::write(output.join("bindings.tsv"), manifest).map_err(|e| e.to_string())?;
    println!("AUTHORED_ORACLE {}", output.display());
    Ok(())
}

/// Keep proof outside the disposable project; FixtureProject::drop removes its root.
pub(super) fn persist_artifacts(project: &FixtureProject) -> Result<(), String> {
    let data = project.root.parent().ok_or("fixture has no data parent")?;
    let destination = data
        .join("diagnostics/npc-authored-textures")
        .join(std::process::id().to_string());
    for folder in ["oracle", "captures"] {
        let target = destination.join(folder);
        fs::create_dir_all(&target).map_err(|error| format!("{}: {error}", target.display()))?;
        let source = project.root.join(folder);
        for entry in
            fs::read_dir(&source).map_err(|error| format!("{}: {error}", source.display()))?
        {
            let entry = entry.map_err(|error| error.to_string())?;
            fs::copy(entry.path(), target.join(entry.file_name()))
                .map_err(|error| format!("preserve {}: {error}", entry.path().display()))?;
        }
    }
    println!("AUTHORED_ARTIFACTS {}", destination.display());
    Ok(())
}

fn decode(source: &Path, fdid: u32) -> Result<Pixels, String> {
    let path = source.join(format!("textures/{fdid}.blp"));
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let image = blp::decode_rgba(&bytes).map_err(|e| format!("FDID {fdid}: {e}"))?;
    Ok((image.pixels, image.width, image.height))
}

/// Read all four DB2 variations directly, independently of the runtime SQLite importer.
fn read_creature_skin_oracle(source: &Path, display: u32) -> Result<HashMap<u32, Pixels>, String> {
    let variations = read_creature_variations(source, display)?;
    let mut textures = HashMap::new();
    for (fdid, kind) in variations.into_iter().zip([11, 12, 13, 5]) {
        if fdid != 0 {
            textures.insert(kind, decode(source, fdid)?);
        }
    }
    Ok(textures)
}

/// Ordinary skins stay block-compressed; CPU and Godot DXT expansion round differently.
/// Retain the complete authored upload and an explicit format for an independent engine image.
fn write_creature_gpu_oracle(source: &Path, output: &Path, display: u32) -> Result<(), String> {
    let variations = read_creature_variations(source, display)?;
    let mut formats = fs::OpenOptions::new()
        .append(true)
        .open(output.join("image-formats.tsv"))
        .map_err(|error| format!("Open oracle formats: {error}"))?;
    for (fdid, kind) in variations
        .into_iter()
        .zip([11, 12, 13, 5])
        .filter(|(fdid, _)| *fdid != 0)
    {
        let bytes = fs::read(source.join(format!("textures/{fdid}.blp")))
            .map_err(|error| format!("Read oracle BLP {fdid}: {error}"))?;
        let image =
            blp::decode_gpu(&bytes).map_err(|error| format!("Oracle BLP {fdid}: {error}"))?;
        fs::write(output.join(format!("{display}-{kind}.gpu")), &image.data)
            .map_err(|error| format!("Write oracle upload: {error}"))?;
        writeln!(
            formats,
            "{display}\t{kind}\t{:?}\t{}\t{}\t{}",
            image.format,
            image.width,
            image.height,
            u8::from(image.mipmaps)
        )
        .map_err(|error| format!("Write oracle format: {error}"))?;
    }
    Ok(())
}

fn read_creature_variations(source: &Path, display: u32) -> Result<[u32; 4], String> {
    let csv = fs::read_to_string(source.join("CreatureDisplayInfo.csv"))
        .map_err(|error| format!("Read CreatureDisplayInfo: {error}"))?;
    parse_creature_variations(&csv, display)
}

fn parse_creature_variations(csv: &str, display: u32) -> Result<[u32; 4], String> {
    let mut lines = csv.lines();
    let header: Vec<_> = lines
        .next()
        .ok_or("empty CreatureDisplayInfo")?
        .split(',')
        .collect();
    let column = |name: &str| {
        header
            .iter()
            .position(|value| *value == name)
            .ok_or_else(|| format!("missing {name}"))
    };
    let id_column = column("ID")?;
    let id = display.to_string();
    let row = lines
        .find(|line| line.split(',').nth(id_column) == Some(id.as_str()))
        .ok_or_else(|| format!("missing DB2 display {display}"))?;
    let values: Vec<_> = row.split(',').collect();
    let mut variations = [0; 4];
    for (variation, fdid) in variations.iter_mut().enumerate() {
        let index = column(&format!("TextureVariationFileDataID_{variation}"))?;
        *fdid = values
            .get(index)
            .ok_or("truncated DB2 display")?
            .parse()
            .map_err(|error| format!("display {display} variation {variation}: {error}"))?;
    }
    Ok(variations)
}

fn oracle_textures(
    source: &Path,
    compositor: &CharTextureData,
    profile: &game_engine_core::npc_appearance_data::AuthoredNpcAppearance,
    materials: &[(u16, u32)],
    layout: u32,
) -> Result<HashMap<u32, Pixels>, String> {
    let size = compositor.layout(layout).ok_or("missing layout")?;
    let default =
        m2_texture::default_fdid_for_type(1, size.width == 2048 && size.height == 1024, &[0; 3])
            .ok_or("missing default")?;
    let mut decoded = HashMap::new();
    for fdid in materials.iter().map(|(_, fdid)| *fdid).chain([default]) {
        decoded.insert(fdid, decode(source, fdid)?);
    }
    let mut composed = compositor
        .composite_model_textures_with(materials, &[], layout, default, |fdid| {
            decoded.get(&fdid).cloned()
        })
        .ok_or("missing composition")?;
    if let Some(bake) = profile.baked_texture_fdid {
        composed.body = decode(source, bake)?;
    }
    let mut result = HashMap::from([(1, composed.body)]);
    if let Some(hair) = select_npc_type6_texture(
        compositor.declares_hair(materials, layout),
        composed.hair,
        composed.head,
    )? {
        result.insert(6, hair);
    }
    for kind in compositor
        .separate_texture_types(layout)
        .into_iter()
        .filter(|kind| *kind != 6)
    {
        if let Some(pixels) = compositor
            .composite_texture_type(materials, layout, kind, |fdid| decoded.get(&fdid).cloned())
        {
            result.insert(kind, pixels);
        }
    }
    Ok(result)
}

fn write_display_oracle(
    source: &Path,
    output: &Path,
    display: u32,
    textures: &HashMap<u32, Pixels>,
    manifest: &mut String,
) -> Result<(), String> {
    let catalog = rusqlite::Connection::open_with_flags(
        source.join("cache/creature_display.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .map_err(|e| e.to_string())?;
    let model_fdid: u32 = catalog
        .query_row(
            "SELECT model_fdid FROM creature_displays WHERE display_id=?1",
            [display],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let read = |name: String| fs::read(source.join("models").join(name)).map_err(|e| e.to_string());
    let skeleton_path = source.join(format!("models/{model_fdid}.skel"));
    let skeleton = if skeleton_path.exists() {
        Some(fs::read(&skeleton_path).map_err(|e| format!("{}: {e}", skeleton_path.display()))?)
    } else {
        None
    };
    let model = m2::parse_model_with_skeleton(
        &read(format!("{model_fdid}.m2"))?,
        &read(format!("{model_fdid}00.skin"))?,
        skeleton.as_deref(),
        |_| None,
    )
    .map_err(|e| e.to_string())?;
    let mut counts = HashMap::<u32, usize>::new();
    // Native node names follow stable draw order, not source SKIN record order.
    let mut units: Vec<_> = model.batches.iter().collect();
    units.sort_by_key(|unit| (unit.priority_plane, unit.material_layer));
    for (index, unit) in units.into_iter().enumerate() {
        let binding = m2_material::batch_binding(&model, unit, &[0; 3])?;
        for (slot, kind) in binding.texture_types.iter().enumerate() {
            let Some((pixels, width, height)) = textures.get(kind) else {
                continue;
            };
            fs::write(output.join(format!("{display}-{kind}.rgba")), pixels)
                .map_err(|e| e.to_string())?;
            manifest.push_str(&format!(
                "{display}\t{kind}\tBatch{index}\t{slot}\t{width}\t{height}\n"
            ));
            *counts.entry(*kind).or_default() += 1;
        }
    }
    let mut kinds: Vec<_> = textures.keys().copied().collect();
    kinds.sort_unstable();
    for kind in kinds {
        println!(
            "AUTHORED_DECLARATION display={display} model={model_fdid} type={kind} slots={}",
            counts.get(&kind).copied().unwrap_or_default()
        );
    }
    Ok(())
}

pub(super) fn run(
    app: &mut App,
    child: &mut Child,
    lines: Receiver<String>,
    readers: Vec<thread::JoinHandle<()>>,
) -> Result<(), String> {
    let (mut player, mut npc) = (None, None);
    let deadline = Instant::now() + Duration::from_secs(300);
    let mut completed = false;
    let mut readers = Some(readers);
    while Instant::now() < deadline {
        app.update();
        respond_to_login(app)?;
        respond_to_selection(app, &mut player, &mut npc)?;
        let status = child.try_wait().map_err(|e| e.to_string())?;
        if status.is_some() {
            for reader in readers.take().ok_or("readers already drained")? {
                reader.join().map_err(|_| "reader panicked")?;
            }
        }
        for line in lines.try_iter() {
            reject_material_error(&line)?;
            if let Some(value) = line.strip_prefix("FIXTURE AUTHORED_REQUEST ") {
                let display: u32 = value.parse().map_err(|_| "invalid requested display")?;
                let (_, template, name) = CASES
                    .iter()
                    .find(|(id, _, _)| *id == display)
                    .ok_or("unexpected display request")?;
                app.world_mut()
                    .despawn(npc.take().ok_or("NPC not spawned")?);
                let entity = spawn_named_npc(app, display, NPC);
                // Stable fixture node alias; replicated template/display are real authored rows.
                app.world_mut().entity_mut(entity).insert(Npc {
                    template_id: *template,
                    name: NPC.into(),
                });
                npc = Some(entity);
                println!("AUTHORED_SPAWN display={display} template={template} name={name}");
            }
            completed |= line == "FIXTURE AUTHORED_COMPLETE";
        }
        if let Some(status) = status {
            return if status.success() && completed {
                Ok(())
            } else {
                Err(format!(
                    "Authored fixture exited {status}, completed={completed}"
                ))
            };
        }
        thread::sleep(TICK);
    }
    Err("Authored fixture timed out".into())
}
