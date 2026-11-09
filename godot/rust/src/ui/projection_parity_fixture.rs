//! Incremental projection contract: after each change, every frame's node matches the
//! node a fresh projection of the same registry draws.
use super::super::RegistryUi;
use super::UiProjection;
use godot::{classes::Control, prelude::*};
use ui_toolkit::{
    frame::WidgetData, rsx, screen::SharedContext, widget_def::Element, widgets::button::ButtonData,
};

#[derive(Clone, PartialEq)]
struct ParityStep(u32);

fn parity_screen(shared: &SharedContext) -> Element {
    let step = shared.get::<ParityStep>().unwrap().0;
    let left = if step >= 1 { 140.0 } else { 100.0 };
    let swatch = if step >= 2 {
        "0.2,0.6,1,0.5"
    } else {
        "1,0.4,0.1,1"
    };
    let group_hidden = step == 3;
    let text = if step >= 4 {
        "Changed text"
    } else {
        "Initial text"
    };
    let extra = if step == 5 {
        rsx! { r#frame { name: "ParityExtra", left: 300.0, top: 10.0, width: 30.0, height: 30.0,
        pos_type: "absolute", background_color: "1,1,0,1" } }
    } else {
        Element::default()
    };
    rsx! {
        r#frame { name: "ParityPanel", left, top: 50.0, width: 600.0, height: 300.0,
            pos_type: "absolute", background_color: "0.1,0.1,0.1,1",
            r#frame { name: "ParitySwatch", left: 10.0, top: 10.0, width: 40.0, height: 40.0,
                pos_type: "absolute", background_color: {swatch} }
            r#frame { name: "ParityGroup", hidden: group_hidden, left: 60.0, top: 10.0,
                width: 200.0, height: 100.0, pos_type: "absolute", background_color: "0,1,0,1",
                fontstring { name: "ParityText", text, left: 5.0, top: 5.0, width: 150.0,
                    height: 20.0, pos_type: "absolute", font_size: 12.0 }
            }
            button { name: "ParityButton", left: 10.0, top: 150.0, width: 120.0, height: 30.0,
                pos_type: "absolute", button_default_skin: false,
                r#frame { name: "ParityButtonHighlight", width: 120.0, height: 30.0,
                    pos_type: "absolute", draw_layer: "HIGHLIGHT", background_color: "1,1,1,0.3" }
            }
            {extra}
        }
    }
}

/// Each projected frame node's drawn properties and its parts, keyed by frame name.
fn node_signature(node: &Gd<Control>) -> String {
    let mut out = control_properties(node);
    if let Some(parts) = node.try_get_node_as::<Control>("Parts") {
        for part in parts.get_children().iter_shared() {
            if let Ok(part) = part.try_cast::<Control>() {
                out.push_str(" | ");
                out.push_str(&control_properties(&part));
            }
        }
    }
    out
}

fn control_properties(node: &Gd<Control>) -> String {
    format!(
        "{} pos={:?} size={:?} visible={} modulate={:?} self_modulate={:?} z={} clip={} mouse={:?} text={}",
        node.get_class(),
        node.get_position(),
        node.get_size(),
        node.is_visible(),
        node.get_modulate(),
        node.get_self_modulate(),
        node.get_z_index(),
        node.is_clipping_contents(),
        node.get_mouse_filter(),
        node.get("text"),
    )
}

impl RegistryUi {
    /// The first frame whose node differs from a fresh projection's, or `None`.
    fn compare_with_fresh_projection(&mut self) -> Result<Option<String>, String> {
        let incremental_root = self
            .projection
            .as_ref()
            .ok_or("Parity fixture has no projection")?
            .root
            .clone();
        let mut fresh = UiProjection::new();
        fresh.root.set_scale(incremental_root.get_scale());
        fresh.root.set_size(incremental_root.get_size());
        fresh.root.set_visible(false);
        self.base_mut().add_child(&fresh.root);
        let (Some(model), Some(incremental)) = (self.model.as_mut(), self.projection.as_ref())
        else {
            return Err("Parity fixture has no model".into());
        };
        let result = fresh.sync(&mut model.registry);
        let mut mismatch = None;
        if result.is_ok() {
            for frame in model.registry.frames_iter() {
                let (Some(left), Some(right)) = (incremental.node(frame.id), fresh.node(frame.id))
                else {
                    mismatch = Some(format!("frame {} has no node", frame.id));
                    break;
                };
                let (left, right) = (node_signature(&left), node_signature(&right));
                if left != right {
                    let name = frame.name.as_deref().unwrap_or("unnamed");
                    mismatch = Some(format!("{name}: incremental {left} != fresh {right}"));
                    break;
                }
            }
        }
        let mut root = fresh.root.clone();
        self.base_mut().remove_child(&root);
        root.queue_free();
        result.map(|()| mismatch)
    }

    fn set_parity_button_hovered(&mut self, hovered: bool) -> Result<(), String> {
        let model = self.model.as_mut().ok_or("Parity fixture has no model")?;
        let id = model
            .registry
            .get_by_name("ParityButton")
            .ok_or("ParityButton missing")?;
        let frame = model.registry.get_mut(id).ok_or("ParityButton missing")?;
        let Some(WidgetData::Button(ButtonData { hovered: state, .. })) = &mut frame.widget_data
        else {
            return Err("ParityButton is not a button".into());
        };
        *state = hovered;
        let projection = self
            .projection
            .as_mut()
            .ok_or("Parity fixture has no projection")?;
        projection.sync(&mut model.registry)
    }

    /// Apply change `step` through the path production uses for it.
    fn apply_parity_step(&mut self, step: u32) -> Result<(), String> {
        match step {
            7 => self.set_parity_button_hovered(true),
            8 => self.set_parity_button_hovered(false),
            9 => self.patch_frames([(
                "ParitySwatch".to_owned(),
                vec![("pos_x", "20".to_owned()), ("alpha", "0.6".to_owned())],
            )]),
            _ => self.set_state(ParityStep(step)),
        }
    }
}

#[godot_api(secondary)]
impl RegistryUi {
    #[func]
    fn show_projection_parity_fixture(&mut self) -> GString {
        let shown = super::super::party_preview::load_data_root().and_then(|()| {
            ui_toolkit::atlas::set_thread_skin(ui_toolkit::atlas::ActiveSkin::Modern);
            self.show_standalone_screen(ParityStep(0), parity_screen)
        });
        shown.err().unwrap_or_default().as_str().into()
    }

    /// Apply change `step`, then compare every node with a fresh projection; an empty
    /// result means they all match.
    #[func]
    fn projection_parity_step(&mut self, step: i64) -> GString {
        let result = (|| {
            let step = u32::try_from(step).map_err(|_| format!("invalid step {step}"))?;
            self.apply_parity_step(step)?;
            match self.compare_with_fresh_projection()? {
                Some(mismatch) => Err(format!("step {step}: {mismatch}")),
                None => Ok(()),
            }
        })();
        result.err().unwrap_or_default().as_str().into()
    }
}
