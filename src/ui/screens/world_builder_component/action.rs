use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorldBuilderAction {
    Close,
    Refresh,
    PreviousPage,
    NextPage,
    RenderOffRoots,
    ProcessOffRoots,
    EnableAll,
    SelectEntity(u64),
    ToggleExpand(u64),
    ToggleRender(u64),
    ToggleProcessing(u64),
    ApplyTransform(u64),
    ApplyPointLight(u64),
    ApplyDirectionalLight(u64),
    TogglePointLightShadows(u64),
    ToggleDirectionalLightShadows(u64),
}

impl fmt::Display for WorldBuilderAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Close => f.write_str("world_builder_close"),
            Self::Refresh => f.write_str("world_builder_refresh"),
            Self::PreviousPage => f.write_str("world_builder_previous_page"),
            Self::NextPage => f.write_str("world_builder_next_page"),
            Self::RenderOffRoots => f.write_str("world_builder_render_off_roots"),
            Self::ProcessOffRoots => f.write_str("world_builder_process_off_roots"),
            Self::EnableAll => f.write_str("world_builder_enable_all"),
            Self::SelectEntity(bits) => write!(f, "world_builder_select:{bits}"),
            Self::ToggleExpand(bits) => write!(f, "world_builder_toggle_expand:{bits}"),
            Self::ToggleRender(bits) => write!(f, "world_builder_toggle_render:{bits}"),
            Self::ToggleProcessing(bits) => write!(f, "world_builder_toggle_processing:{bits}"),
            Self::ApplyTransform(bits) => write!(f, "world_builder_apply_transform:{bits}"),
            Self::ApplyPointLight(bits) => write!(f, "world_builder_apply_point_light:{bits}"),
            Self::ApplyDirectionalLight(bits) => {
                write!(f, "world_builder_apply_directional_light:{bits}")
            }
            Self::TogglePointLightShadows(bits) => {
                write!(f, "world_builder_toggle_point_light_shadows:{bits}")
            }
            Self::ToggleDirectionalLightShadows(bits) => {
                write!(f, "world_builder_toggle_directional_light_shadows:{bits}")
            }
        }
    }
}

impl WorldBuilderAction {
    pub fn parse(value: &str) -> Option<Self> {
        parse_static_action(value).or_else(|| parse_entity_action_variants(value))
    }
}

fn parse_static_action(value: &str) -> Option<WorldBuilderAction> {
    match value {
        "world_builder_close" => Some(WorldBuilderAction::Close),
        "world_builder_refresh" => Some(WorldBuilderAction::Refresh),
        "world_builder_previous_page" => Some(WorldBuilderAction::PreviousPage),
        "world_builder_next_page" => Some(WorldBuilderAction::NextPage),
        "world_builder_render_off_roots" => Some(WorldBuilderAction::RenderOffRoots),
        "world_builder_process_off_roots" => Some(WorldBuilderAction::ProcessOffRoots),
        "world_builder_enable_all" => Some(WorldBuilderAction::EnableAll),
        _ => None,
    }
}

type EntityAction = (&'static str, fn(u64) -> WorldBuilderAction);

const ENTITY_ACTIONS: [EntityAction; 9] = [
    ("world_builder_select:", WorldBuilderAction::SelectEntity),
    (
        "world_builder_toggle_expand:",
        WorldBuilderAction::ToggleExpand,
    ),
    (
        "world_builder_toggle_render:",
        WorldBuilderAction::ToggleRender,
    ),
    (
        "world_builder_toggle_processing:",
        WorldBuilderAction::ToggleProcessing,
    ),
    (
        "world_builder_apply_transform:",
        WorldBuilderAction::ApplyTransform,
    ),
    (
        "world_builder_apply_point_light:",
        WorldBuilderAction::ApplyPointLight,
    ),
    (
        "world_builder_apply_directional_light:",
        WorldBuilderAction::ApplyDirectionalLight,
    ),
    (
        "world_builder_toggle_point_light_shadows:",
        WorldBuilderAction::TogglePointLightShadows,
    ),
    (
        "world_builder_toggle_directional_light_shadows:",
        WorldBuilderAction::ToggleDirectionalLightShadows,
    ),
];

fn parse_entity_action_variants(value: &str) -> Option<WorldBuilderAction> {
    ENTITY_ACTIONS
        .iter()
        .find_map(|(prefix, build)| parse_entity_action(value, prefix, *build))
}

fn parse_entity_action(
    value: &str,
    prefix: &str,
    build: fn(u64) -> WorldBuilderAction,
) -> Option<WorldBuilderAction> {
    value.strip_prefix(prefix)?.parse().ok().map(build)
}
