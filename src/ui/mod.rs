/// Generates a `#[derive(Resource)]` struct whose fields are `u64` widget IDs
/// resolved by name from a `FrameRegistry`. Fields marked with `?` become `Option<u64>`.
///
/// Accepts either a `FrameName` constant (uses `.0` to get the `&str`) or a string literal.
///
/// ```ignore
/// ui_resource! {
///     pub(crate) LoginUi {
///         root: "LoginRoot",
///         username_input: USERNAME_INPUT,
///         reconnect_button?: "ReconnectButton",
///     }
/// }
/// ```
#[macro_export]
macro_rules! ui_resource {
    // Entry point: accumulate fields via TT muncher
    ( $vis:vis $name:ident { $($rest:tt)* } ) => {
        $crate::ui_resource!(@accum $vis $name [] [] $($rest)*);
    };

    // Optional field: `name?: IDENT,`
    (@accum $vis:vis $name:ident
        [ $( ($rf:ident, $rk:tt) )* ]
        [ $( ($of:ident, $ok:tt) )* ]
        $field:ident ?: $key:ident, $($rest:tt)*
    ) => {
        $crate::ui_resource!(@accum $vis $name
            [ $( ($rf, $rk) )* ]
            [ $( ($of, $ok) )* ($field, $key) ]
            $($rest)*
        );
    };

    // Optional field: `name?: "literal",`
    (@accum $vis:vis $name:ident
        [ $( ($rf:ident, $rk:tt) )* ]
        [ $( ($of:ident, $ok:tt) )* ]
        $field:ident ?: $key:literal, $($rest:tt)*
    ) => {
        $crate::ui_resource!(@accum $vis $name
            [ $( ($rf, $rk) )* ]
            [ $( ($of, $ok) )* ($field, $key) ]
            $($rest)*
        );
    };

    // Required field: `name: IDENT,`
    (@accum $vis:vis $name:ident
        [ $( ($rf:ident, $rk:tt) )* ]
        [ $( ($of:ident, $ok:tt) )* ]
        $field:ident : $key:ident, $($rest:tt)*
    ) => {
        $crate::ui_resource!(@accum $vis $name
            [ $( ($rf, $rk) )* ($field, $key) ]
            [ $( ($of, $ok) )* ]
            $($rest)*
        );
    };

    // Required field: `name: "literal",`
    (@accum $vis:vis $name:ident
        [ $( ($rf:ident, $rk:tt) )* ]
        [ $( ($of:ident, $ok:tt) )* ]
        $field:ident : $key:literal, $($rest:tt)*
    ) => {
        $crate::ui_resource!(@accum $vis $name
            [ $( ($rf, $rk) )* ($field, $key) ]
            [ $( ($of, $ok) )* ]
            $($rest)*
        );
    };

    // Terminal: emit struct + impl
    (@accum $vis:vis $name:ident
        [ $( ($rf:ident, $rk:tt) )* ]
        [ $( ($of:ident, $ok:tt) )* ]
    ) => {
        #[derive(Resource)]
        $vis struct $name {
            $( $vis $rf: u64, )*
            $( $vis $of: Option<u64>, )*
        }

        impl $name {
            $vis fn resolve(reg: &$crate::ui::registry::FrameRegistry) -> Self {
                Self {
                    $( $rf: reg.get_by_name($crate::ui_resource!(@key_str $rk))
                        .expect($crate::ui_resource!(@key_str $rk)), )*
                    $( $of: reg.get_by_name($crate::ui_resource!(@key_str $ok)), )*
                }
            }
        }
    };

    // Key string: identifier (FrameName constant) → access .0
    (@key_str $key:ident) => { $key.0 };
    // Key string: string literal → pass through
    (@key_str $key:literal) => { $key };
}

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
