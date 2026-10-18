//! Synthetic assets only; exercises the ordinary native placement and upload path.
use super::*;
use crate::particle_models::GeometryModel;
use game_engine_core::m2::AnimTime;

#[derive(GodotClass)]
#[class(base = Node3D)]
struct WowParticleModelFixture {
    placed: Option<PlacedParticles>,
    pools: Option<ParticlePools>,
    base: Base<Node3D>,
}

#[godot_api]
impl INode3D for WowParticleModelFixture {
    fn init(base: Base<Node3D>) -> Self {
        Self {
            placed: None,
            pools: None,
            base,
        }
    }
}

fn emitter() -> ParticleEmitter {
    ParticleEmitter {
        flags: 0x20000,
        emitter_type: 1,
        texture_fdid: Some(910001),
        tile_rows: 1,
        tile_cols: 1,
        emission_rate: 100.0,
        lifespan: 1.0,
        emission_speed: 2.0,
        spin: 2.0,
        color_keys: vec![(0.0, [255.0, 0.0, 0.0]), (1.0, [0.0, 0.0, 255.0])],
        opacity_keys: vec![(0.0, 1.0), (1.0, 0.0)],
        scale_keys: vec![(0.0, [0.7, 0.7]), (1.0, [1.4, 1.4])],
        twinkle_percent: 1.0,
        twinkle_scale_min: 1.0,
        twinkle_scale_max: 1.0,
        ..Default::default()
    }
}

fn arrow_model(path: &GString) -> Result<(m2::Model, Rc<GeometryModel>), String> {
    let mut model = crate::assets::read_model(path)?;
    let positions = [
        [0.6, 0.0, 0.0],
        [-0.6, 0.0, 0.0],
        [-0.1, 0.25, 0.0],
        [-0.1, 0.0, 0.25],
        [-0.1, -0.25, 0.0],
        [-0.1, 0.0, -0.25],
    ];
    model.vertices = positions
        .into_iter()
        .map(|position| m2::Vertex {
            position,
            normal: [0.0, 0.0, 1.0],
            tex_coords: [0.5, 0.5],
            tex_coords_2: [0.5, 0.5],
            bone_weights: [0; 4],
            bone_indices: [0; 4],
        })
        .collect();
    model.indices = vec![
        0, 2, 3, 0, 3, 4, 0, 4, 5, 0, 5, 2, 1, 3, 2, 1, 4, 3, 1, 5, 4, 1, 2, 5,
    ];
    model.submeshes[0].vertex_count = 6;
    model.submeshes[0].triangle_count = 24;
    model.materials[0].flags = 7; // Explicit synthetic unlit, unfogged, two-sided geometry.
    let (node, missing) = crate::assets::build_model(&model, path, &[0; 3], None)?;
    if !missing.is_empty() {
        node.free();
        return Err(format!("synthetic geometry textures missing {missing:?}"));
    }
    let geometry = GeometryModel::from_node(&node);
    node.free();
    Ok((model, geometry?))
}

fn resolved_models(path: &GString) -> Result<(Rc<ModelParticles>, ParticleEmitter), String> {
    let (mut model, geometry) = arrow_model(path)?;
    let child = ParticleEmitter {
        lifespan: 2.0,
        emission_speed: 0.0,
        spin: 0.0,
        scale_keys: vec![(0.0, [0.08; 2])],
        color_keys: vec![(0.0, [0.0, 255.0, 0.0])],
        ..emitter()
    };
    model.particle_emitters = vec![child.clone()];
    let recursive =
        ModelParticles::from_model(910004, &model).ok_or("synthetic child not admitted")?;
    model.particle_emitters = vec![ParticleEmitter {
        particle_model_filename: Some("synthetic/arrow.m2".into()),
        child_emitters_model_filename: Some("synthetic/child.m2".into()),
        ..emitter()
    }];
    let mut root =
        ModelParticles::from_model(910003, &model).ok_or("synthetic parent not admitted")?;
    Rc::get_mut(&mut root)
        .expect("unique synthetic model")
        .resolved_models
        .insert(
            0,
            ResolvedParticleModels {
                geometry: Some(geometry),
                recursive: vec![(recursive, false)],
            },
        );
    Ok((root, child))
}

fn disable_births(sim: &mut EmitterSim, emitter: &ParticleEmitter) {
    let mut disabled = emitter.clone();
    disabled.emission_rate = 0.0;
    disabled.tracks = Default::default();
    sim.set_animation(
        &disabled,
        &AnimTime {
            sequence: 0,
            time_ms: 0,
            global_ms: 0.0,
            global_sequences: &[],
        },
    );
}

fn tick(sim: &mut EmitterSim, emitter: &ParticleEmitter, dt: f32, position: Vec3) {
    sim.update(
        emitter,
        dt,
        Mat4::from_translation(position),
        Mat3::IDENTITY,
        1.0,
        &mut |_, _| None,
    );
}

fn prime(
    sim: &mut EmitterSim,
    emitter: &ParticleEmitter,
    child: &ParticleEmitter,
    parents: i32,
) -> f32 {
    let mut children = std::mem::take(&mut sim.children);
    tick(sim, emitter, 0.015, Vec3::X * -2.0);
    if parents == 2 {
        tick(sim, emitter, 0.01, Vec3::X * 2.0);
    }
    disable_births(sim, emitter);
    let child_sim = &mut children[0].sim;
    tick(child_sim, child, 0.015, Vec3::ZERO);
    let tagged_age = child_sim.particles()[0].age;
    sim.children = children;
    tagged_age
}

pub(super) fn place(
    mut world: Gd<Node3D>,
    path: GString,
    parents: i32,
    elapsed: f32,
) -> Result<Gd<Node3D>, String> {
    if !(1..=2).contains(&parents) {
        return Err("synthetic parent count must be one or two".into());
    }
    let (model, child) = resolved_models(&path)?;
    let mut fixture = WowParticleModelFixture::new_alloc();
    let mut owner = fixture.clone().upcast::<Node3D>();
    world.add_child(&owner);
    let mut pools = ParticlePools::new(1.0);
    pools.attach(&mut owner);
    let texture_dir = godot::classes::ProjectSettings::singleton()
        .globalize_path(&path)
        .to_string();
    let texture_dir = Path::new(&texture_dir)
        .parent()
        .and_then(Path::parent)
        .ok_or("synthetic asset root")?
        .join("textures");
    let (mut placed, errors) = pools.place(&model, &owner, 17, &texture_dir);
    if !errors.is_empty() {
        owner.free();
        return Err(errors.join("; "));
    }
    let emitter = &model.emitters[0].1;
    let sim = &mut placed.emitters[0].sim;
    let tagged_age = prime(sim, emitter, &child, parents);
    let mut before = Vec::new();
    sim.model_particles(emitter, &mut before);
    owner.set_meta(
        "before_origin",
        &Vector3::from_array(before[0].transform.w_axis.truncate().to_array()).to_variant(),
    );
    owner.set_meta(
        "before_axis_x",
        &Vector3::from_array(before[0].transform.x_axis.truncate().to_array()).to_variant(),
    );
    owner.set_meta(
        "before_color",
        &Color::from_rgba(
            before[0].color.x,
            before[0].color.y,
            before[0].color.z,
            before[0].color.w,
        )
        .to_variant(),
    );
    let unchanged = sim.particles().to_vec();
    tick(sim, emitter, 0.0, Vec3::splat(99.0));
    owner.set_meta(
        "zero_duration_pass",
        &(sim.particles() == unchanged).to_variant(),
    );
    let view = ViewBasis {
        right: Vec3::X,
        up: Vec3::Y,
        back: Vec3::Z,
    };
    pools.begin_frame();
    placed.update_and_draw(&owner, elapsed, 1.0, &view, &mut pools);
    pools.end_frame();
    let sim = &placed.emitters[0].sim;
    owner.set_meta(
        "tagged_child_age_delta",
        &(sim.children[0].sim.particles()[0].age - tagged_age).to_variant(),
    );
    let child_pool = owner
        .find_child_ex("Particles910004_0")
        .recursive(true)
        .owned(false)
        .done()
        .ok_or("synthetic recursive pool missing")?;
    let mut child_pool = child_pool.cast::<MultiMeshInstance3D>();
    child_pool.set_name("RecursiveParticles");
    let mut drained = sim.clone();
    disable_births(&mut drained, emitter);
    disable_births(&mut drained.children[0].sim, &child);
    for _ in 0..30 {
        tick(&mut drained, emitter, 0.1, Vec3::ZERO);
    }
    owner.set_meta(
        "drained_parent_count",
        &(drained.particles().len() as i64).to_variant(),
    );
    owner.set_meta(
        "drained_child_count",
        &(drained.children[0].sim.particles().len() as i64).to_variant(),
    );
    let alive = std::mem::replace(&mut placed.emitters[0].sim, drained);
    pools.begin_frame();
    placed.update_and_draw(&owner, 0.0, 1.0, &view, &mut pools);
    pools.end_frame();
    let geometry = owner
        .find_child_ex("GeometryParticles")
        .recursive(true)
        .owned(false)
        .done()
        .ok_or("synthetic geometry pool missing")?
        .cast::<MultiMeshInstance3D>();
    owner.set_meta(
        "drained_geometry_visible",
        &geometry
            .get_multimesh()
            .expect("geometry multimesh")
            .get_visible_instance_count()
            .to_variant(),
    );
    owner.set_meta(
        "drained_children_visible",
        &child_pool
            .get_multimesh()
            .expect("child multimesh")
            .get_visible_instance_count()
            .to_variant(),
    );
    // Restore the same live snapshot for the raster and lifetime-transform assertions.
    placed.emitters[0].sim = alive;
    pools.begin_frame();
    placed.update_and_draw(&owner, 0.0, 1.0, &view, &mut pools);
    pools.end_frame();
    fixture.bind_mut().placed = Some(placed);
    fixture.bind_mut().pools = Some(pools);
    Ok(owner)
}
