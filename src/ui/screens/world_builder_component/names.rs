pub fn world_builder_row_name(entity_bits: u64) -> String {
    format!("WorldBuilderRow_{entity_bits}")
}

pub fn world_builder_row_expand_name(entity_bits: u64) -> String {
    format!("WorldBuilderRowExpand_{entity_bits}")
}

pub fn world_builder_row_render_name(entity_bits: u64) -> String {
    format!("WorldBuilderRowRender_{entity_bits}")
}

pub fn world_builder_row_processing_name(entity_bits: u64) -> String {
    format!("WorldBuilderRowProcessing_{entity_bits}")
}

pub fn world_builder_transform_edit_name(entity_bits: u64, axis: usize) -> String {
    format!("WorldBuilderTransform_{entity_bits}_{axis}")
}

pub fn world_builder_rotation_edit_name(entity_bits: u64, axis: usize) -> String {
    format!("WorldBuilderRotation_{entity_bits}_{axis}")
}

pub fn world_builder_scale_edit_name(entity_bits: u64, axis: usize) -> String {
    format!("WorldBuilderScale_{entity_bits}_{axis}")
}

pub fn world_builder_point_light_edit_name(entity_bits: u64, field: usize) -> String {
    format!("WorldBuilderPointLight_{entity_bits}_{field}")
}

pub fn world_builder_directional_light_edit_name(entity_bits: u64) -> String {
    format!("WorldBuilderDirectionalLight_{entity_bits}")
}
