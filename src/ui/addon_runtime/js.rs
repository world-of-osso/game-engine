use std::cell::RefCell;

use quick_js::{Arguments, Context, JsValue};
use ui_toolkit::anchor::AnchorPoint;

use super::AddonOperation;

type SetPointArgs = (String, AnchorPoint, Option<String>, AnchorPoint, f32, f32);

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
  setPoint: (
    name,
    point,
    relativeTo = null,
    relativePoint = null,
    x = 0,
    y = 0,
  ) =>
    __addonSetPoint(
      String(name),
      String(point).toUpperCase(),
      relativeTo == null ? "" : String(relativeTo),
      relativePoint == null
        ? String(point).toUpperCase()
        : String(relativePoint).toUpperCase(),
      Number(x),
      Number(y),
    ),
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
    register_set_point_callback(ctx)?;
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

fn register_set_point_callback(ctx: &Context) -> Result<(), String> {
    ctx.add_callback(
        "__addonSetPoint",
        |args: Arguments| -> Result<bool, String> {
            let (name, point, relative_to, relative_point, x, y) = parse_set_point_args(args)?;
            push_operation(AddonOperation::SetPoint {
                name,
                point,
                relative_to,
                relative_point,
                x,
                y,
            });
            Ok(true)
        },
    )
    .map_err(|err| format!("failed to register setPoint callback: {err}"))
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

fn parse_anchor_point(value: &str) -> Result<AnchorPoint, String> {
    AnchorPoint::from_str(value).ok_or_else(|| format!("unknown anchor point '{value}'"))
}

fn parse_set_point_args(args: Arguments) -> Result<SetPointArgs, String> {
    let values = args.into_vec();
    if values.len() != 6 {
        return Err(format!(
            "setPoint expects 6 arguments, got {}",
            values.len()
        ));
    }
    let mut values = values.into_iter();
    let name = parse_js_string(values.next(), "setPoint name")?;
    let point = parse_anchor_point(&parse_js_string(values.next(), "setPoint point")?)?;
    let relative_to = optional_name(parse_js_string(values.next(), "setPoint relativeTo")?);
    let relative_point =
        parse_anchor_point(&parse_js_string(values.next(), "setPoint relativePoint")?)?;
    let x = parse_js_number(values.next(), "setPoint x")?;
    let y = parse_js_number(values.next(), "setPoint y")?;
    Ok((name, point, relative_to, relative_point, x, y))
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
