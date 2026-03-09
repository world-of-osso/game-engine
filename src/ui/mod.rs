pub mod addon_watcher;
pub mod anchor;
pub mod animation;
pub mod atlas;
pub mod automation;
pub mod automation_script;
mod dioxus_anchor;
pub mod dioxus_elements;
pub mod dioxus_renderer;
pub mod dioxus_runtime;
pub mod dioxus_screen;
pub mod event;
pub mod font_registry;
pub mod frame;
pub mod input;
pub mod js_automation;
pub mod layout;
pub mod plugin;
pub mod registry;
pub mod render;
pub mod render_border;
pub mod render_button;
pub mod render_nine_slice;
pub mod render_text;
pub mod render_text_fx;
pub mod render_texture;
pub mod render_tiled;
pub mod screens;
pub mod spellbook_data;
pub mod strata;
pub mod text_measure;
pub mod wasm_host;
pub mod widgets;

#[cfg(test)]
mod panel_tests;
#[cfg(test)]
mod render_tests;
