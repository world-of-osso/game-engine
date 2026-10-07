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
use std::collections::HashMap;

type Pixels = (Vec<u8>, u32, u32);
const DISPLAYS: [u32; 5] = [825, 1322, 1285, 90209, 110154];

pub(super) fn stage(project: &FixtureProject) -> Result<(), String> {
    let checkout = fixture_support::checkout_root_from_executable("native_npc_visual_fixture")?;
    let source = checkout.join("data");
    let data = project.root.join("data");
    for name in [
        "npc_appearance.sqlite",
        "customization-v4.sqlite",
        "char_texture-v2.sqlite",
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
    for name in [
        "ChrRaceXChrModel.csv",
        "ChrRaces.csv",
        "ChrCustomizationReq.csv",
        "ChrCustomizationReqChoice.csv",
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
    for display in DISPLAYS {
        let profile =
            query_authored_npc_appearance(&profiles, display)?.ok_or("missing real profile")?;
        let selected = select_npc_choices(&profile, &db)?;
        let layout = db
            .layout_id(profile.race, profile.sex)
            .ok_or("missing real layout")?;
        let textures =
            oracle_textures(&source, &compositor, &profile, &selected.materials, layout)?;
        write_display_oracle(&source, &output, display, &textures, &mut manifest)?;
    }
    fs::write(output.join("bindings.tsv"), manifest).map_err(|e| e.to_string())?;
    println!("AUTHORED_ORACLE {}", output.display());
    Ok(())
}

fn decode(source: &Path, fdid: u32) -> Result<Pixels, String> {
    let path = source.join(format!("textures/{fdid}.blp"));
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let image = blp::decode_rgba(&bytes).map_err(|e| format!("FDID {fdid}: {e}"))?;
    Ok((image.pixels, image.width, image.height))
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
    let model = m2::parse_model(
        &read(format!("{model_fdid}.m2"))?,
        &read(format!("{model_fdid}00.skin"))?,
    )
    .map_err(|e| e.to_string())?;
    let mut counts = HashMap::<u32, usize>::new();
    for (index, unit) in model.batches.iter().enumerate() {
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
                if !DISPLAYS.contains(&display) {
                    return Err("unexpected display request".into());
                }
                app.world_mut()
                    .entity_mut(npc.ok_or("NPC not spawned")?)
                    .insert(ModelDisplay {
                        display_id: display,
                    });
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
