use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static SHARED_REPO_ROOT: OnceLock<PathBuf> = OnceLock::new();

pub fn worktree_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn shared_repo_root() -> PathBuf {
    SHARED_REPO_ROOT
        .get_or_init(|| {
            std::env::var_os("GAME_ENGINE_SHARED_ROOT")
                .map(PathBuf::from)
                .unwrap_or_else(|| derive_shared_repo_root(worktree_root()))
        })
        .clone()
}

pub fn worktree_data_root() -> PathBuf {
    worktree_root().join("data")
}

pub fn shared_data_root() -> PathBuf {
    if let Some(path) = std::env::var_os("GAME_ENGINE_SHARED_DATA_DIR") {
        PathBuf::from(path)
    } else {
        shared_repo_root().join("data")
    }
}

pub fn resolve_data_path(relative: impl AsRef<Path>) -> PathBuf {
    let relative = relative.as_ref();
    let worktree_path = worktree_data_root().join(relative);
    if worktree_path.exists() {
        worktree_path
    } else {
        shared_data_root().join(relative)
    }
}

pub fn shared_data_path(relative: impl AsRef<Path>) -> PathBuf {
    shared_data_root().join(relative)
}

pub fn remap_to_shared_data_path(path: &Path) -> PathBuf {
    if let Ok(stripped) = path.strip_prefix(worktree_data_root()) {
        return shared_data_path(stripped);
    }
    if let Ok(stripped) = path.strip_prefix("data") {
        return shared_data_path(stripped);
    }
    path.to_path_buf()
}

fn derive_shared_repo_root(worktree_root: PathBuf) -> PathBuf {
    let git_path = worktree_root.join(".git");
    let Ok(metadata) = std::fs::metadata(&git_path) else {
        return worktree_root;
    };
    if metadata.is_dir() {
        return worktree_root;
    }
    let Ok(contents) = std::fs::read_to_string(&git_path) else {
        return worktree_root;
    };
    parse_gitdir_file(&contents)
        .and_then(|gitdir| canonical_root_from_gitdir(&gitdir))
        .unwrap_or(worktree_root)
}

fn parse_gitdir_file(contents: &str) -> Option<PathBuf> {
    let gitdir = contents.strip_prefix("gitdir:")?.trim();
    Some(PathBuf::from(gitdir))
}

fn canonical_root_from_gitdir(gitdir: &Path) -> Option<PathBuf> {
    let worktrees_dir = gitdir.parent()?;
    if worktrees_dir.file_name()? != "worktrees" {
        return None;
    }
    let git_common_dir = worktrees_dir.parent()?;
    if git_common_dir.file_name()? != ".git" {
        return None;
    }
    git_common_dir.parent().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_root_comes_from_linked_worktree_gitdir() {
        let gitdir = Path::new(
            "/syncthing/Sync/Projects/world-of-osso/game-engine/.git/worktrees/game-engine-fix",
        );
        assert_eq!(
            canonical_root_from_gitdir(gitdir),
            Some(PathBuf::from(
                "/syncthing/Sync/Projects/world-of-osso/game-engine"
            ))
        );
    }

    #[test]
    fn remap_relative_data_path_to_shared_root() {
        unsafe {
            std::env::set_var(
                "GAME_ENGINE_SHARED_DATA_DIR",
                "/tmp/game-engine-shared-data-test",
            );
        }
        assert_eq!(
            remap_to_shared_data_path(Path::new("data/textures/123.blp")),
            PathBuf::from("/tmp/game-engine-shared-data-test/textures/123.blp")
        );
        unsafe {
            std::env::remove_var("GAME_ENGINE_SHARED_DATA_DIR");
        }
    }
}
