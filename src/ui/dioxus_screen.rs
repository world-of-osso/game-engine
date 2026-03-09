use dioxus_core::{Element, ScopeId, VirtualDom};

use crate::ui::dioxus_renderer::{GameUiRenderer, MutationApplier};
use crate::ui::registry::FrameRegistry;

/// Generic Dioxus-to-FrameRegistry bridge. One per screen.
pub struct DioxusScreen {
    dom: VirtualDom,
    renderer: GameUiRenderer,
    initialized: bool,
}

impl DioxusScreen {
    pub fn new(component: fn() -> Element) -> Self {
        Self {
            dom: VirtualDom::new(component),
            renderer: GameUiRenderer::new(),
            initialized: false,
        }
    }

    pub fn sync(&mut self, registry: &mut FrameRegistry) {
        {
            let mut applier = MutationApplier::new(&mut self.renderer, registry);
            if self.initialized {
                self.dom.render_immediate(&mut applier);
            } else {
                self.dom.rebuild(&mut applier);
                self.initialized = true;
            }
        }
        self.renderer.resolve_pending_anchors(registry);
    }

    pub fn renderer(&self) -> &GameUiRenderer {
        &self.renderer
    }

    pub fn provide_root_context<T: Clone + 'static>(&self, context: T) {
        self.dom.provide_root_context(context);
    }

    pub fn mark_dirty_root(&mut self) {
        self.dom.mark_dirty(ScopeId::APP);
    }

    pub fn teardown(&mut self, registry: &mut FrameRegistry) {
        for fid in self.renderer.all_frame_ids() {
            registry.remove_frame(fid);
        }
        self.renderer = GameUiRenderer::new();
        self.initialized = false;
    }
}
