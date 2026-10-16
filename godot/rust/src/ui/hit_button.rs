//! Native button picking uses the same authored insets as registry pointer lookup.
use godot::classes::{Button, IButton};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(init, base=Button)]
pub(super) struct RegistryButton {
    base: Base<Button>,
    pub(super) hit_rect_insets: [f32; 4],
}

#[godot_api]
impl IButton for RegistryButton {
    fn has_point(&self, point: Vector2) -> bool {
        contains_point(self.base().get_size(), self.hit_rect_insets, point)
    }
}

fn contains_point(size: Vector2, [left, right, top, bottom]: [f32; 4], point: Vector2) -> bool {
    let origin = Vector2::new(left, top);
    let hit_size = size - Vector2::new(left + right, top + bottom);
    Rect2::new(origin, hit_size).has_point(point)
}

#[cfg(test)]
mod tests {
    use super::contains_point;
    use godot::prelude::*;

    #[test]
    fn native_button_hit_insets_extend_or_shrink_without_changing_size() {
        let size = Vector2::new(37.0, 37.0);
        let extended = [0.0, -157.0, 0.0, 0.0];
        assert!(contains_point(size, extended, Vector2::new(193.5, 20.0)));
        assert!(!contains_point(size, extended, Vector2::new(194.0, 20.0)));
        assert!(!contains_point(size, extended, Vector2::new(-0.5, 20.0)));
        let shrunk = [2.0, 3.0, 4.0, 5.0];
        assert!(contains_point(size, shrunk, Vector2::new(2.0, 4.0)));
        assert!(!contains_point(size, shrunk, Vector2::new(1.5, 4.0)));
        assert!(!contains_point(size, shrunk, Vector2::new(34.0, 4.0)));
        assert!(!contains_point(size, shrunk, Vector2::new(2.0, 3.5)));
        assert!(!contains_point(size, shrunk, Vector2::new(2.0, 32.0)));
        assert!(contains_point(size, [0.0; 4], Vector2::new(36.5, 36.5)));
        assert!(!contains_point(size, [0.0; 4], Vector2::new(37.0, 20.0)));
    }
}
