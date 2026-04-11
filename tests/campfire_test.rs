use game_engine::asset::adt_format::adt_obj;
use game_engine::asset::m2;
use game_engine::asset::m2_format::m2_light::evaluate_light;
use std::path::Path;

#[test]
fn find_warband_campfire_model() {
    let adt_path = Path::new("data/terrain/2703_31_37_obj0.adt");
    let data = std::fs::read(adt_path).expect("failed to read obj0");
    let obj = adt_obj::load_adt_obj0(&data).expect("failed to parse obj0");

    eprintln!("=== Warband Campfire Doodad Search ===");
    eprintln!(
        "Found {} doodads in warband tile 2703_31_37",
        obj.doodads.len()
    );

    // Look for campfire-like models
    let mut campfires = Vec::new();
    for (idx, doodad) in obj.doodads.iter().enumerate() {
        let model_name = doodad
            .path
            .as_deref()
            .or_else(|| {
                doodad
                    .fdid
                    .and_then(|fdid| game_engine::listfile::lookup_fdid(fdid))
            })
            .unwrap_or("<unknown>");

        if model_name.contains("campfire")
            || model_name.contains("10ct_centaur")
            || model_name.contains("centaur")
        {
            campfires.push((
                idx,
                model_name.to_string(),
                doodad.fdid,
                doodad.position.clone(),
                doodad.rotation.clone(),
                doodad.scale,
            ));
        }
    }

    eprintln!("\nFound {} campfire doodads:", campfires.len());
    for (idx, name, fdid, pos, rot, scale) in &campfires {
        eprintln!("\n  Index {}: {}", idx, name);
        eprintln!("    FDID: {:?}", fdid);
        eprintln!(
            "    Position: [{:.2}, {:.2}, {:.2}]",
            pos[0], pos[1], pos[2]
        );
        eprintln!(
            "    Rotation: [{:.2}, {:.2}, {:.2}]",
            rot[0], rot[1], rot[2]
        );
        eprintln!("    Scale: {:.4}", scale);
    }

    assert!(!campfires.is_empty(), "Should find at least one campfire");
}

#[test]
fn parse_campfire_m2_lights() {
    eprintln!("\n=== Campfire M2 Light Analysis ===");

    // The campfire model from the warband char select tile
    let campfire_fdid = 4182539u32;
    let campfire_path = "world/expansion09/doodads/centaur/10ct_centaur_campfire01.m2";

    eprintln!("Model: {}", campfire_path);
    eprintln!("FDID: {}", campfire_fdid);

    // Try to load from data directory first (should be cached)
    let model_data_path = format!("data/models/{}.m2", campfire_fdid);
    let _m2_data = match std::fs::read(&model_data_path) {
        Ok(data) => {
            eprintln!("Found cached M2 at: {}", model_data_path);
            eprintln!("File size: {} bytes", data.len());
            data
        }
        Err(_) => {
            eprintln!("M2 not cached - skipping light parsing");
            eprintln!("(M2 file would need to be extracted from game CASC data first)");
            return;
        }
    };

    // Parse using the game_engine M2 loader
    match m2::load_m2(Path::new(&model_data_path), &[0; 3]) {
        Ok(m2_model) => {
            eprintln!("\nM2 Parse Results:");
            eprintln!("  ✓ MD21 magic verified");
            eprintln!("  Bones: {}", m2_model.bones.len());
            eprintln!("  Batches: {}", m2_model.batches.len());
            eprintln!("  Lights: {}", m2_model.lights.len());

            if !m2_model.lights.is_empty() {
                eprintln!("\nLight Details:");
                for (i, light) in m2_model.lights.iter().enumerate() {
                    eprintln!("\n  Light {}:", i);
                    eprintln!("    Type: {} (1=point, 0=directional)", light.light_type);
                    eprintln!("    Bone Index: {}", light.bone_index);
                    eprintln!(
                        "    Position (local): [{:.2}, {:.2}, {:.2}]",
                        light.position[0], light.position[1], light.position[2]
                    );

                    // Evaluate at default time
                    let evaluated = evaluate_light(light, 0, 0);
                    eprintln!("    Evaluated Light (seq=0, time=0ms):");
                    eprintln!("      Visible: {}", evaluated.visible);
                    eprintln!(
                        "      Color: RGB([{:.4}, {:.4}, {:.4}])",
                        evaluated.color[0], evaluated.color[1], evaluated.color[2]
                    );
                    eprintln!("      Intensity: {:.1} lumens", evaluated.intensity);
                    eprintln!(
                        "      Attenuation Start: {:.2}",
                        evaluated.attenuation_start
                    );
                    eprintln!("      Attenuation End: {:.2}", evaluated.attenuation_end);
                }
            } else {
                eprintln!("\n⚠ No lights found in the M2 model");
                eprintln!("The campfire visual lighting comes from the char-select scene's");
                eprintln!("hardcoded \"CampfireLight\" PointLight component defined in:");
                eprintln!("  src/scenes/char_select/scene/lighting.rs");
                eprintln!("\nHardcoded campfire light parameters:");
                eprintln!("  CAMPFIRE_LIGHT_COLOR: srgb(1.0, 0.58, 0.28) [orange/amber]");
                eprintln!("  CAMPFIRE_LIGHT_INTENSITY: 0.0 (uses hardcoded scale)");
                eprintln!("  CAMPFIRE_LIGHT_RANGE: 18.0");
                eprintln!("  CAMPFIRE_LIGHT_RADIUS: 0.55");
                eprintln!("  Offset from character: Vec3::new(-2.8, 0.9, -3.1)");
            }
        }
        Err(e) => {
            eprintln!("Failed to parse M2: {}", e);
        }
    }
}
