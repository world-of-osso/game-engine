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

// Re-exported from ui-toolkit
pub use ui_toolkit::anchor;
pub use ui_toolkit::animation;
pub use ui_toolkit::atlas;
pub use ui_toolkit::event;
pub use ui_toolkit::font_registry;
pub use ui_toolkit::frame;
pub use ui_toolkit::input;
pub use ui_toolkit::layout;
pub use ui_toolkit::plugin;
pub use ui_toolkit::registry;
pub use ui_toolkit::render;
pub use ui_toolkit::render_border;
pub use ui_toolkit::render_button;
pub use ui_toolkit::render_nine_slice;
pub use ui_toolkit::render_text;
pub use ui_toolkit::render_text_fx;
pub use ui_toolkit::render_texture;
pub use ui_toolkit::render_tiled;
pub use ui_toolkit::screen;
pub use ui_toolkit::strata;
pub use ui_toolkit::text_measure;
pub use ui_toolkit::widgets;

// Game-specific modules (stay in game-engine)
pub mod addon_runtime;
pub mod addon_watcher;
pub mod automation;
pub mod automation_script;
pub mod game_plugin;
pub mod js_automation;
pub mod panel_styles;
pub mod screens;
pub mod spellbook_data;
pub mod spellbook_frames;
pub mod spellbook_runtime;
pub mod wasm_host;
