use std::cell::RefCell;

use bevy::ui::PositionType;
use quick_js::{Arguments, Context, JsValue};
use ui_toolkit::anchor::AnchorTarget;

use super::AddonOperation;

const PRELUDE: &str = r#"
globalThis.addon = {
  createFrame: (name, parent = null) =>
    __addonCreateFrame(String(name), parent == null ? "" : String(parent)),
  createFontString: (name, parent = null, text = "") =>
    __addonCreateFontString(
      String(name),
      parent == null ? "" : String(parent),
      String(text),
    ),
  setSize: (name, width, height) =>
    __addonSetSize(String(name), Number(width), Number(height)),
  setPos: (...args) => __addonSetPos(...args),
  setPosType: (...args) => __addonSetPosType(...args),
  setAnchor: (...args) => __addonSetAnchor(...args),
  setText: (name, text) => __addonSetText(String(name), String(text)),
  show: (name) => __addonShow(String(name)),
  hide: (name) => __addonHide(String(name)),
  setAlpha: (name, alpha) => __addonSetAlpha(String(name), Number(alpha)),
  setBackgroundColor: (name, r, g, b, a = 1) =>
    __addonSetBackgroundColor(
      String(name),
      Number(r),
      Number(g),
      Number(b),
      Number(a),
    ),
  setFontColor: (name, r, g, b, a = 1) =>
    __addonSetFontColor(
      String(name),
      Number(r),
      Number(g),
      Number(b),
      Number(a),
    ),
};
"#;

thread_local! {
    static ADDON_OPERATIONS: RefCell<Vec<AddonOperation>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn run_js_addon_to_operations(script: &str) -> Result<Vec<AddonOperation>, String> {
    let ctx = Context::new().map_err(|err| format!("failed to create JS context: {err}"))?;
    ADDON_OPERATIONS.with(|ops| ops.borrow_mut().clear());
    register_callbacks(&ctx)?;
    ctx.eval(PRELUDE)
        .map_err(|err| format!("failed to initialize addon JS helpers: {err}"))?;
    ctx.eval(script)
        .map_err(|err| format!("failed to execute addon script: {err}"))?;
    Ok(ADDON_OPERATIONS.with(|ops| ops.borrow().clone()))
}

fn register_callbacks(ctx: &Context) -> Result<(), String> {
    register_create_callbacks(ctx)?;
    register_layout_callbacks(ctx)?;
    register_visibility_callbacks(ctx)?;
    register_style_callbacks(ctx)?;
    Ok(())
}

fn register_create_callbacks(ctx: &Context) -> Result<(), String> {
    ctx.add_callback(
        "__addonCreateFrame",
        |name: String, parent: String| -> bool {
            push_operation(AddonOperation::CreateFrame {
                name,
                parent: optional_name(parent),
            });
            true
        },
    )
    .map_err(|err| format!("failed to register createFrame callback: {err}"))?;
    ctx.add_callback(
        "__addonCreateFontString",
        |name: String, parent: String, text: String| -> bool {
            push_operation(AddonOperation::CreateFontString {
                name,
                parent: optional_name(parent),
                text,
            });
            true
        },
    )
    .map_err(|err| format!("failed to register createFontString callback: {err}"))?;
    Ok(())
}

fn register_layout_callbacks(ctx: &Context) -> Result<(), String> {
    register_set_size_callback(ctx)?;
    register_position_callbacks(ctx)?;
    register_set_text_callback(ctx)?;
    Ok(())
}

fn register_set_size_callback(ctx: &Context) -> Result<(), String> {
    ctx.add_callback(
        "__addonSetSize",
        |args: Arguments| -> Result<bool, String> {
            let (name, width, height) = parse_set_size_args(args)?;
            push_operation(AddonOperation::SetSize {
                name,
                width,
                height,
            });
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register setSize callback: {err}"))
}

fn register_position_callbacks(ctx: &Context) -> Result<(), String> {
    ctx.add_callback("__addonSetPos", |args: Arguments| -> Result<bool, String> {
        let (name, x, y) = parse_set_pos_args(args)?;
        push_operation(AddonOperation::SetPos { name, x, y });
        Ok(true)
    })
    .map_err(|err| format!("failed to register setPos callback: {err}"))?;
    ctx.add_callback(
        "__addonSetPosType",
        |args: Arguments| -> Result<bool, String> {
            let (name, position_type) = parse_set_pos_type_args(args)?;
            push_operation(AddonOperation::SetPosType {
                name,
                position_type,
            });
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register setPosType callback: {err}"))?;
    ctx.add_callback(
        "__addonSetAnchor",
        |args: Arguments| -> Result<bool, String> {
            let (name, target) = parse_set_anchor_args(args)?;
            push_operation(AddonOperation::SetAnchor { name, target });
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register setAnchor callback: {err}"))
}

fn register_set_text_callback(ctx: &Context) -> Result<(), String> {
    ctx.add_callback("__addonSetText", |name: String, text: String| -> bool {
        push_operation(AddonOperation::SetText { name, text });
        true
    })
    .map_err(|err| format!("failed to register setText callback: {err}"))
}

fn register_visibility_callbacks(ctx: &Context) -> Result<(), String> {
    ctx.add_callback("__addonShow", |name: String| -> bool {
        push_operation(AddonOperation::Show { name });
        true
    })
    .map_err(|err| format!("failed to register show callback: {err}"))?;
    ctx.add_callback("__addonHide", |name: String| -> bool {
        push_operation(AddonOperation::Hide { name });
        true
    })
    .map_err(|err| format!("failed to register hide callback: {err}"))?;
    Ok(())
}

fn register_style_callbacks(ctx: &Context) -> Result<(), String> {
    ctx.add_callback(
        "__addonSetAlpha",
        |args: Arguments| -> Result<bool, String> {
            let (name, alpha) = parse_set_alpha_args(args)?;
            push_operation(AddonOperation::SetAlpha { name, alpha });
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register setAlpha callback: {err}"))?;
    register_background_color_callback(ctx)?;
    register_font_color_callback(ctx)?;
    Ok(())
}

fn register_background_color_callback(ctx: &Context) -> Result<(), String> {
    ctx.add_callback(
        "__addonSetBackgroundColor",
        |args: Arguments| -> Result<bool, String> {
            let (name, color) = parse_set_color_args(args, "setBackgroundColor")?;
            push_operation(AddonOperation::SetBackgroundColor { name, color });
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register setBackgroundColor callback: {err}"))
}

fn register_font_color_callback(ctx: &Context) -> Result<(), String> {
    ctx.add_callback(
        "__addonSetFontColor",
        |args: Arguments| -> Result<bool, String> {
            let (name, color) = parse_set_color_args(args, "setFontColor")?;
            push_operation(AddonOperation::SetFontColor { name, color });
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register setFontColor callback: {err}"))
}

fn push_operation(operation: AddonOperation) {
    ADDON_OPERATIONS.with(|ops| ops.borrow_mut().push(operation));
}

fn optional_name(value: String) -> Option<String> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

fn position_arguments(
    args: Arguments,
    label: &str,
    count: usize,
) -> Result<std::vec::IntoIter<JsValue>, String> {
    let values = args.into_vec();
    if values.len() != count {
        return Err(format!(
            "{label} expects {count} arguments, got {}",
            values.len()
        ));
    }
    Ok(values.into_iter())
}

fn parse_set_pos_args(args: Arguments) -> Result<(String, f32, f32), String> {
    let mut values = position_arguments(args, "setPos", 3)?;
    let name = parse_js_string(values.next(), "setPos name")?;
    let x = parse_finite_position(values.next(), "setPos x")?;
    let y = parse_finite_position(values.next(), "setPos y")?;
    Ok((name, x, y))
}

fn parse_finite_position(value: Option<JsValue>, label: &str) -> Result<f32, String> {
    let value = parse_js_number(value, label)?;
    if !value.is_finite() {
        return Err(format!("{label} must be finite and representable as f32"));
    }
    Ok(value)
}

fn parse_set_pos_type_args(args: Arguments) -> Result<(String, PositionType), String> {
    let mut values = position_arguments(args, "setPosType", 2)?;
    let name = parse_js_string(values.next(), "setPosType name")?;
    let value = parse_js_string(values.next(), "setPosType type")?;
    let position_type = match value.as_str() {
        "relative" => PositionType::Relative,
        "absolute" => PositionType::Absolute,
        _ => {
            return Err(format!(
                "setPosType type must be 'relative' or 'absolute', got '{value}'"
            ));
        }
    };
    Ok((name, position_type))
}

fn parse_set_anchor_args(args: Arguments) -> Result<(String, AnchorTarget), String> {
    let values = args.into_vec();
    if !(1..=2).contains(&values.len()) {
        return Err(format!(
            "setAnchor expects 1 or 2 arguments, got {}",
            values.len()
        ));
    }
    let mut values = values.into_iter();
    let name = parse_js_string(values.next(), "setAnchor name")?;
    let target = match values.next() {
        None | Some(JsValue::Undefined) => AnchorTarget::Parent,
        Some(JsValue::String(value)) => match value.as_str() {
            "parent" => AnchorTarget::Parent,
            "screen" => AnchorTarget::Screen,
            _ => {
                return Err(format!(
                    "setAnchor target must be 'parent' or 'screen', got '{value}'"
                ));
            }
        },
        _ => return Err("setAnchor target must be 'parent' or 'screen'".into()),
    };
    Ok((name, target))
}

fn parse_set_size_args(args: Arguments) -> Result<(String, f32, f32), String> {
    let values = args.into_vec();
    if values.len() != 3 {
        return Err(format!("setSize expects 3 arguments, got {}", values.len()));
    }
    let mut values = values.into_iter();
    let name = parse_js_string(values.next(), "setSize name")?;
    let width = parse_js_number(values.next(), "setSize width")?;
    let height = parse_js_number(values.next(), "setSize height")?;
    Ok((name, width, height))
}

fn parse_set_alpha_args(args: Arguments) -> Result<(String, f32), String> {
    let values = args.into_vec();
    if values.len() != 2 {
        return Err(format!(
            "setAlpha expects 2 arguments, got {}",
            values.len()
        ));
    }
    let mut values = values.into_iter();
    let name = parse_js_string(values.next(), "setAlpha name")?;
    let alpha = parse_js_number(values.next(), "setAlpha alpha")?;
    Ok((name, alpha))
}

fn parse_set_color_args(args: Arguments, label: &str) -> Result<(String, [f32; 4]), String> {
    let values = args.into_vec();
    if values.len() != 5 {
        return Err(format!("{label} expects 5 arguments, got {}", values.len()));
    }
    let mut values = values.into_iter();
    let name = parse_js_string(values.next(), &format!("{label} name"))?;
    let r = parse_js_number(values.next(), &format!("{label} r"))?;
    let g = parse_js_number(values.next(), &format!("{label} g"))?;
    let b = parse_js_number(values.next(), &format!("{label} b"))?;
    let a = parse_js_number(values.next(), &format!("{label} a"))?;
    Ok((name, [r, g, b, a]))
}

fn parse_js_string(value: Option<JsValue>, label: &str) -> Result<String, String> {
    match value {
        Some(JsValue::String(value)) => Ok(value),
        _ => Err(format!("{label} must be a string")),
    }
}

fn parse_js_number(value: Option<JsValue>, label: &str) -> Result<f32, String> {
    match value {
        Some(JsValue::Int(value)) => Ok(value as f32),
        Some(JsValue::Float(value)) => Ok(value as f32),
        _ => Err(format!("{label} must be numeric")),
    }
}
