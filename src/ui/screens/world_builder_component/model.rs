#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBuilderRow {
    pub entity_bits: u64,
    pub depth: usize,
    pub label: String,
    pub has_children: bool,
    pub expanded: bool,
    pub selected: bool,
    pub render_hidden: bool,
    pub processing_suspended: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBuilderPointLightState {
    pub intensity: String,
    pub range: String,
    pub shadows_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBuilderDirectionalLightState {
    pub illuminance: String,
    pub shadows_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBuilderPropertyState {
    pub selected_label: String,
    pub selected_id: u64,
    pub translation: [String; 3],
    pub rotation_degrees: [String; 3],
    pub scale: [String; 3],
    pub component_names: Vec<String>,
    pub point_light: Option<WorldBuilderPointLightState>,
    pub directional_light: Option<WorldBuilderDirectionalLightState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorldBuilderViewState {
    pub open: bool,
    pub total_entities: usize,
    pub matching_entities: usize,
    pub rows: Vec<WorldBuilderRow>,
    pub page_index: usize,
    pub page_count: usize,
    pub filter: String,
    pub selected: Option<WorldBuilderPropertyState>,
    pub status: String,
}
