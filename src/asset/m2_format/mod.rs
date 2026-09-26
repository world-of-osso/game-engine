#[path = "pure.rs"]
mod pure;
pub(crate) use pure::parser::{
    M2Chunks, M2Material, M2Submesh, M2TextureUnit, M2Vertex, SkinData, TextureTables,
};
pub use pure::*;
mod file_loader;
pub use file_loader::ensure_primary_skin_path;
pub(crate) use file_loader::{load_anim_data, load_skin_data};
