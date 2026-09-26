#[path = "../../../src/asset/adt_format/mod.rs"]
pub mod adt_format;
#[path = "../../../src/asset/blp_format.rs"]
pub mod blp_format;
#[path = "../../../src/asset/m2_format/pure.rs"]
pub mod m2_format;
#[path = "../../../src/asset/read_bytes.rs"]
pub mod read_bytes;
#[path = "../../../src/asset/wdt.rs"]
pub mod wdt;
#[path = "../../../src/asset/wmo_format/mod.rs"]
pub mod wmo_format;

pub use adt_format::{adt, adt_obj};

// Original WMO format tests call these functions through asset::wmo.
#[cfg(test)]
pub(crate) mod wmo {
    pub use super::wmo_format::parser::load_wmo_root;
}
