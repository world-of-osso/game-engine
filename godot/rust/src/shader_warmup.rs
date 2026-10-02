//! Shaders compiled ahead of need. Godot compiles a shader when a material first takes
//! it, 10-50 ms of main-thread time each, which stalled character select and world
//! entry. Retail ships its shaders precompiled; this client records each shader the
//! scene uses and compiles the recorded ones during asset startup, login and loading
//! screen frames of later runs.

use std::{cell::RefCell, collections::HashSet, fs, io::Write, path::PathBuf};

use godot::{
    classes::{ProjectSettings, ResourceLoader, Shader},
    prelude::*,
};

use crate::{
    assets::material::{self, Pipeline},
    wmo::scene::{self as wmo, WmoShaderKey},
};

/// One used shader per line: `m2 <pipeline>`, `wmo <key>` or `resource <res:// path>`.
const USED_SHADERS_PATH: &str = "user://used_shaders.txt";

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum UsedShader {
    /// An M2 batch pipeline's shader variants.
    M2(Pipeline),
    /// A WMO material shader variant.
    Wmo(WmoShaderKey),
    /// A shader resource, used as authored.
    Resource(String),
}

impl UsedShader {
    fn line(&self) -> String {
        match self {
            Self::M2(pipeline) => format!("m2 {}", pipeline.line()),
            Self::Wmo(key) => format!("wmo {}", key.line()),
            Self::Resource(path) => format!("resource {path}"),
        }
    }

    fn parse(line: &str) -> Option<Self> {
        match line.split_once(' ')? {
            ("m2", pipeline) => Pipeline::parse(pipeline).map(Self::M2),
            ("wmo", key) => WmoShaderKey::parse(key).map(Self::Wmo),
            ("resource", path) if path.starts_with("res://") => {
                Some(Self::Resource(path.to_owned()))
            }
            _ => None,
        }
    }

    /// Compiles the shader; a resource shader is returned to be kept loaded.
    fn compile(&self) -> Result<Option<Gd<Shader>>, String> {
        match self {
            Self::M2(pipeline) => material::compile_pipeline(*pipeline).map(|()| None),
            Self::Wmo(key) => wmo::compile_shader(*key).map(|()| None),
            Self::Resource(path) => {
                let shader = load_resource(path)?;
                // The RID creates the rendering server's shader, which compiles it.
                shader.get_rid();
                Ok(Some(shader))
            }
        }
    }
}

#[derive(Default)]
struct UsedShaders {
    used: Vec<UsedShader>,
    compiled: HashSet<UsedShader>,
    /// Compiled shader resources stay loaded, so their users take these instances.
    resources: Vec<Gd<Shader>>,
}

thread_local! {
    /// `USED_SHADERS_PATH`, read on first use.
    static USED: RefCell<Option<UsedShaders>> = const { RefCell::new(None) };
}

pub(crate) fn clear() {
    USED.with_borrow_mut(|used| *used = None);
}

/// Loads the shader resource at `path` and records its use.
pub(crate) fn load_shader(path: &str) -> Result<Gd<Shader>, String> {
    let shader = load_resource(path)?;
    record(UsedShader::Resource(path.to_owned()));
    Ok(shader)
}

fn load_resource(path: &str) -> Result<Gd<Shader>, String> {
    ResourceLoader::singleton()
        .load(path)
        .ok_or_else(|| format!("Cannot load shader {path}"))?
        .try_cast::<Shader>()
        .map_err(|_| format!("{path} is not a shader"))
}

/// Compiles the next used shader this run has not compiled yet; false once none remain.
pub(crate) fn compile_next() -> Result<bool, String> {
    let next = with_used(|shaders| {
        let next = shaders
            .used
            .iter()
            .find(|shader| !shaders.compiled.contains(*shader))?
            .clone();
        shaders.compiled.insert(next.clone());
        Some(next)
    });
    let Some(shader) = next else {
        return Ok(false);
    };
    let _span = crate::profile::span(|| format!("shader.compile {shader:?}"));
    if let Some(resource) = shader.compile()? {
        with_used(|shaders| shaders.resources.push(resource));
    }
    Ok(true)
}

/// Appends `shader` to `USED_SHADERS_PATH` the first time any run uses it.
pub(crate) fn record(shader: UsedShader) {
    let line = shader.line();
    let new = with_used(|shaders| {
        let new = !shaders.used.contains(&shader);
        if new {
            shaders.used.push(shader);
        }
        new
    });
    if !new {
        return;
    }
    let path = used_shaders_path();
    let appended = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .and_then(|mut file| writeln!(file, "{line}"));
    if let Err(error) = appended {
        godot_error!("Cannot record used shader in {}: {error}", path.display());
    }
}

fn with_used<R>(f: impl FnOnce(&mut UsedShaders) -> R) -> R {
    USED.with_borrow_mut(|shaders| f(shaders.get_or_insert_with(read_used)))
}

fn used_shaders_path() -> PathBuf {
    PathBuf::from(
        ProjectSettings::singleton()
            .globalize_path(USED_SHADERS_PATH)
            .to_string(),
    )
}

fn read_used() -> UsedShaders {
    let path = used_shaders_path();
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(error) => {
            godot_error!("Cannot read {}: {error}", path.display());
            String::new()
        }
    };
    let mut used = Vec::new();
    for line in text.lines() {
        match UsedShader::parse(line) {
            Some(shader) if !used.contains(&shader) => used.push(shader),
            Some(_) => {}
            None => godot_error!("{}: malformed used shader {line:?}", path.display()),
        }
    }
    UsedShaders {
        used,
        ..UsedShaders::default()
    }
}
