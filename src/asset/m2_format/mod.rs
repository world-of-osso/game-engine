#[path = "pure.rs"]
mod pure;
#[cfg(test)]
pub(crate) use pure::parser::parse_skin_full;
pub(crate) use pure::parser::{M2Chunks, M2Material, M2Submesh, M2Vertex, SkinData, TextureTables};
pub use pure::*;
mod file_loader;
pub use file_loader::ensure_primary_skin_path;
pub(crate) use file_loader::{load_anim_data, load_skin_data};
