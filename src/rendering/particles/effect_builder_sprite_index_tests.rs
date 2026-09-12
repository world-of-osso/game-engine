use super::*;
use bevy_hanabi::{Modifier, ShaderWriter};

#[test]
fn random_flipbook_generates_integer_sprite_index_wgsl() {
    let emitter = M2ParticleEmitter {
        flags: PARTICLE_FLAG_RANDOM_TEXTURE,
        tile_rows: 2,
        tile_cols: 2,
        ..Default::default()
    };
    let writer = ExprWriter::new();
    let (init, _) = build_flipbook_sprite_index_modifiers(&emitter, &writer);
    let mut module = writer.finish();
    let properties = PropertyLayout::empty();
    let particles = ParticleLayout::new()
        .append(Attribute::SPRITE_INDEX)
        .build();
    let mut shader = ShaderWriter::new(ModifierContext::Init, &properties, &particles);
    init.expect("random flipbook initializes its sprite index")
        .apply(&mut module, &mut shader)
        .expect("sprite index initialization emits WGSL");

    assert_eq!(
        shader.main_code,
        "let var0 = frand();\nparticle.sprite_index = i32(floor((var0) * (4.)));\n",
        "Hanabi sprite_index is i32; random atlas selection must emit an integer assignment"
    );
}
