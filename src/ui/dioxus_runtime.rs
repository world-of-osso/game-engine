use dioxus_core::{Element, Template, TemplateAttribute, TemplateNode, VNode, VirtualDom};

use crate::ui::dioxus_renderer::{GameUiRenderer, MutationApplier};
use crate::ui::registry::FrameRegistry;

/// Drives a Dioxus VirtualDom and applies its mutations into the frame registry.
pub struct DioxusUiRuntime {
    dom: VirtualDom,
    renderer: GameUiRenderer,
    initialized: bool,
}

impl DioxusUiRuntime {
    pub fn new() -> Self {
        Self {
            dom: VirtualDom::new(game_ui_root),
            renderer: GameUiRenderer::new(),
            initialized: false,
        }
    }

    pub fn sync(&mut self, registry: &mut FrameRegistry) {
        let mut applier = MutationApplier::new(&mut self.renderer, registry);
        if self.initialized {
            self.dom.render_immediate(&mut applier);
        } else {
            self.dom.rebuild(&mut applier);
            self.initialized = true;
        }
    }
}

fn game_ui_root() -> Element {
    static TEMPLATE: Template = Template {
        roots: &[TemplateNode::Element {
            tag: "Frame",
            namespace: None,
            attrs: &[
                TemplateAttribute::Static {
                    name: "width",
                    value: "320",
                    namespace: None,
                },
                TemplateAttribute::Static {
                    name: "height",
                    value: "64",
                    namespace: None,
                },
                TemplateAttribute::Static {
                    name: "background_color",
                    value: "0.08,0.08,0.12,0.9",
                    namespace: None,
                },
                TemplateAttribute::Static {
                    name: "strata",
                    value: "DIALOG",
                    namespace: None,
                },
            ],
            children: &[],
        }],
        node_paths: &[],
        attr_paths: &[],
    };

    Ok(VNode::new(None, TEMPLATE, Box::new([]), Box::new([])))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sync_builds_frames_from_virtual_dom() {
        let mut runtime = DioxusUiRuntime::new();
        let mut registry = FrameRegistry::new(1920.0, 1080.0);

        runtime.sync(&mut registry);

        let frames: Vec<_> = registry.frames_iter().collect();
        assert_eq!(frames.len(), 1);

        let frame = frames[0];
        assert_eq!(frame.width, 320.0);
        assert_eq!(frame.height, 64.0);
        assert_eq!(frame.strata, crate::ui::strata::FrameStrata::Dialog);
        assert_eq!(frame.background_color, Some([0.08, 0.08, 0.12, 0.9]));
    }
}
