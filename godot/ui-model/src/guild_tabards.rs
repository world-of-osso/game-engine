//! Installed catalog for Retail guild tabard presentation and draft controls.
use shared::{
    guild_creation::GuildTabardDesign,
    guild_tabards::GuildTabardCatalog,
    protocol::{GuildTabardReply, GuildTabardRequest, GuildTabardVendorSnapshot},
};
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

#[derive(Debug, Clone, PartialEq, Default)]
pub struct GuildTabards {
    pub vendor: Option<GuildTabardVendorSnapshot>,
    pub opening: Option<u64>,
    pub draft: Option<GuildTabardDesign>,
    pub waiting: bool,
    pub gold: Option<u64>,
}

impl GuildTabards {
    pub fn begin_open(&mut self, npc: u64) -> GuildTabardRequest {
        *self = Self {
            opening: Some(npc),
            gold: self.gold,
            ..Self::default()
        };
        GuildTabardRequest::Open { npc }
    }

    pub fn close(&mut self) -> Option<u64> {
        let npc = self
            .vendor
            .as_ref()
            .map(|vendor| vendor.npc)
            .or(self.opening);
        *self = Self {
            gold: self.gold,
            ..Self::default()
        };
        npc
    }

    pub fn receive(&mut self, reply: GuildTabardReply) -> Result<(), String> {
        match reply {
            GuildTabardReply::Vendor(vendor) => {
                if self.opening != Some(vendor.npc) {
                    return Ok(());
                }
                let catalog = load_catalog()?;
                let design = match vendor.design {
                    Some(design) => design,
                    None => initial_design(catalog)?,
                };
                catalog.validate(design)?;
                self.draft = Some(design);
                self.vendor = Some(vendor);
                self.opening = None;
                self.waiting = false;
            }
            GuildTabardReply::Saved { design } => {
                if let Some(vendor) = &mut self.vendor {
                    vendor.design = Some(design);
                }
                self.waiting = false;
            }
            GuildTabardReply::Failed { .. } => {
                self.waiting = false;
                self.opening = None;
            }
        }
        Ok(())
    }

    pub fn can_save(&self) -> bool {
        self.vendor
            .as_ref()
            .is_some_and(|vendor| vendor.own_rank == Some(0))
            && self.draft.is_some()
            && !self.waiting
    }

    pub fn click(&mut self, action: &str) -> Result<Option<GuildTabardRequest>, String> {
        if action == "tabard:close" {
            self.close();
            return Ok(None);
        }
        if action == "tabard:accept" {
            if !self.can_save() {
                return Ok(None);
            }
            let vendor = self.vendor.as_ref().expect("save requires vendor");
            let design = self.draft.expect("save requires draft");
            self.waiting = true;
            return Ok(Some(GuildTabardRequest::Save {
                npc: vendor.npc,
                design,
            }));
        }
        let Some(cycle) = action.strip_prefix("tabard:cycle:") else {
            return Ok(None);
        };
        let (field, direction) = cycle
            .split_once(':')
            .ok_or("Malformed tabard variation action")?;
        let field = field
            .parse::<u8>()
            .map_err(|error| format!("Invalid tabard variation: {error}"))?;
        let direction = direction
            .parse::<i32>()
            .map_err(|error| format!("Invalid tabard direction: {error}"))?;
        let Some(mut design) = self.draft else {
            return Ok(None);
        };
        cycle_design(&mut design, field, direction, load_catalog()?)?;
        self.draft = Some(design);
        Ok(None)
    }
}

/// TabardModelFrameMixin uses held rotate buttons, not click increments (0.5turn/s).
/// Rotation is native model state, not a reason to rebuild the UI each frame.
pub fn calculate_rotation(rotation: f32, elapsed: f32, left: bool, right: bool) -> f32 {
    let direction = if right {
        1.0
    } else if left {
        -1.0
    } else {
        0.0
    };
    (rotation + direction * elapsed * std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
}

fn initial_design(catalog: &GuildTabardCatalog) -> Result<GuildTabardDesign, String> {
    let first = |ids: Vec<u8>| {
        ids.into_iter()
            .next()
            .ok_or_else(|| "Guild tabard variation has no source IDs".to_string())
    };
    Ok(GuildTabardDesign {
        icon: first(catalog.emblems.keys().copied().collect())?,
        icon_color: first(catalog.emblem_colors.keys().copied().collect())?,
        border: first(catalog.border_styles())?,
        border_color: first(catalog.border_colors.keys().copied().collect())?,
        background_color: first(catalog.background_colors.keys().copied().collect())?,
    })
}

fn cycle_design(
    design: &mut GuildTabardDesign,
    field: u8,
    direction: i32,
    catalog: &GuildTabardCatalog,
) -> Result<(), String> {
    if !matches!(direction, -1 | 1) {
        return Err("Tabard variation direction must be -1 or1".into());
    }
    let (value, ids): (&mut u8, Vec<_>) = match field {
        1 => (&mut design.icon, catalog.emblems.keys().copied().collect()),
        2 => (
            &mut design.icon_color,
            catalog.emblem_colors.keys().copied().collect(),
        ),
        3 => (&mut design.border, catalog.border_styles()),
        4 => (
            &mut design.border_color,
            catalog.border_colors.keys().copied().collect(),
        ),
        5 => (
            &mut design.background_color,
            catalog.background_colors.keys().copied().collect(),
        ),
        _ => return Err("Unknown tabard variation".into()),
    };
    let index = ids
        .iter()
        .position(|id| id == value)
        .ok_or("Current tabard variation is not in source catalog")?;
    let next = (index as i32 + direction).rem_euclid(ids.len() as i32) as usize;
    *value = ids[next];
    catalog.validate(*design)
}

fn rgb([red, green, blue]: [u8; 3]) -> String {
    format!(
        "{},{},{},1",
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0
    )
}
