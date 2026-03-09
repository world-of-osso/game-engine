use dioxus_core::{Element, ScopeId, VirtualDom};

use crate::ui::dioxus_renderer::{GameUiRenderer, MutationApplier};
use crate::ui::frame::WidgetData;
use crate::ui::registry::FrameRegistry;
use crate::ui::text_measure::measure_text;

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
        auto_size_fontstrings(&self.renderer, registry);
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

/// Auto-size fontstring frames that have width == 0 by measuring their text.
fn auto_size_fontstrings(renderer: &GameUiRenderer, registry: &mut FrameRegistry) {
    for fid in renderer.all_frame_ids() {
        let Some(frame) = registry.get(fid) else {
            continue;
        };
        let Some(WidgetData::FontString(fs)) = &frame.widget_data else {
            continue;
        };
        if frame.width > 0.0 || fs.text.is_empty() {
            continue;
        }
        let text = fs.text.clone();
        let font = fs.font.clone();
        let font_size = fs.font_size;
        if let Some((w, h)) = measure_text(&text, &font, font_size) {
            let frame = registry.get_mut(fid).unwrap();
            frame.width = w;
            frame.height = h;
        }
    }
}
