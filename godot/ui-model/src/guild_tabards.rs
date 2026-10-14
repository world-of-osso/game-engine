//! Installed catalog for Retail guild tabard presentation and draft controls.
use shared::{guild_creation::GuildTabardDesign, guild_tabards::GuildTabardCatalog};
use std::sync::OnceLock;

static CATALOG: OnceLock<Result<GuildTabardCatalog, String>> = OnceLock::new();

/// Data root is fixed for the client process. Missing source data fails, never substitutes a design.
pub fn load_catalog() -> Result<&'static GuildTabardCatalog, String> {
    CATALOG
        .get_or_init(|| GuildTabardCatalog::load(&crate::paths::resolve_data_path("db2/active")))
        .as_ref()
        .map_err(Clone::clone)
}

pub(crate) struct PortraitArt {
    pub fdid: u32,
    pub width: f32,
    pub uv: String,
    pub background: String,
    pub border: String,
    pub emblem: String,
}

/// GuildUtil115–127,180–202: missing registered design is an explicit source branch.
pub(crate) fn load_portrait_art(design: Option<GuildTabardDesign>) -> Result<PortraitArt, String> {
    let Some(design) = design else {
        return Ok(PortraitArt {
            fdid: 460904,
            width: 64.0,
            uv: "0,1,0,1".into(),
            background: "0.2245,0.2088,0.1794,1".into(),
            border: "0.2,0.2,0.2,1".into(),
            emblem: "1,1,1,1".into(),
        });
    };
    let catalog = load_catalog()?;
    catalog.validate(design)?;
    let size = 64.0 / 1024.0;
    let left = f32::from(design.icon % 16) * size;
    let top = f32::from(design.icon / 16) * size;
    Ok(PortraitArt {
        fdid: 459023,
        width: 56.0,
        uv: format!("{left},{},{top},{}", left + size, top + size),
        background: rgb(catalog.background_colors[&design.background_color]),
        border: rgb(catalog.border_colors[&design.border_color]),
        emblem: rgb(catalog.emblem_colors[&design.icon_color]),
    })
}

fn rgb([red, green, blue]: [u8; 3]) -> String {
    format!(
        "{},{},{},1",
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0
    )
}
