//! Clockwise radial CooldownFrame swipe shared by journals, bars and viewers.
use super::RegistryUi;
use godot::classes::{ColorRect, Shader, ShaderMaterial};
use godot::prelude::*;

const SWIPE_SHADER: &str = r#"shader_type canvas_item;
uniform float remaining = 0.0;
uniform float opacity;
void fragment() {
    vec2 p = UV - vec2(0.5);
    float angle = mod(atan(p.x, -p.y) + 6.28318530718, 6.28318530718) / 6.28318530718;
    COLOR = vec4(0.0, 0.0, 0.0, angle < remaining ? opacity : 0.0);
}
"#;
impl RegistryUi {
    pub(crate) fn update_cooldown_swipe(
        &mut self,
        name: &str,
        fraction: f32,
        opacity: f32,
    ) -> Result<(), String> {
        let Some(mut parent) = self.frame_control(name) else {
            return Ok(());
        };
        let mut rect = if let Some(node) = parent.get_node_or_null("ToyRadialSwipe") {
            node.try_cast::<ColorRect>()
                .map_err(|_| format!("Cooldown {name} has non-ColorRect swipe"))?
        } else {
            if fraction <= 0.0 {
                return Ok(());
            }
            let mut shader = Shader::new_gd();
            shader.set_code(SWIPE_SHADER);
            let mut material = ShaderMaterial::new_gd();
            material.set_shader(&shader);
            let mut rect = ColorRect::new_alloc();
            rect.set_name("ToyRadialSwipe");
            rect.set_mouse_filter(godot::classes::control::MouseFilter::IGNORE);
            rect.set_material(&material);
            parent.add_child(&rect);
            rect
        };
        rect.set_size(parent.get_size());
        rect.set_visible(fraction > 0.0);
        let mut material = rect
            .get_material()
            .ok_or("Cooldown swipe material missing")?
            .try_cast::<ShaderMaterial>()
            .map_err(|_| "Cooldown swipe material is not ShaderMaterial")?;
        material.set_shader_parameter("remaining", &fraction.to_variant());
        material.set_shader_parameter("opacity", &opacity.to_variant());
        Ok(())
    }
}
