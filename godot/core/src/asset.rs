#[path = "asset/adt_format/mod.rs"]
pub mod adt_format;
#[path = "asset/blp_format.rs"]
pub mod blp_format;
#[path = "asset/m2_batch_data.rs"]
pub mod m2_batch_data;
#[path = "asset/m2_format/pure.rs"]
pub mod m2_format;
#[path = "asset/m2_texture.rs"]
pub mod m2_texture;
#[path = "asset/read_bytes.rs"]
pub mod read_bytes;
#[path = "asset/wdt.rs"]
pub mod wdt;
#[path = "asset/wmo_format/mod.rs"]
pub mod wmo_format;

pub use crate::char_texture_data as char_texture;
pub use adt_format::{adt, adt_obj};

// Original WMO format tests call these functions through asset::wmo.
#[cfg(test)]
pub(crate) mod wmo {
    pub use super::wmo_format::parser::load_wmo_root;
}
