use bevy::prelude::*;

pub fn point(center: Vec3, extents: Vec3, sx: f32, sy: f32, sz: f32) -> Vec3 {
    Vec3::new(
        center.x + extents.x * sx,
        center.y + extents.y * sy,
        center.z + extents.z * sz,
    )
}

pub fn is_descendant_or_self(
    candidate: Entity,
    ancestor: Entity,
    parent_query: &Query<&ChildOf>,
) -> bool {
    if candidate == ancestor {
        return true;
    }
    let mut current = candidate;
    while let Ok(parent) = parent_query.get(current) {
        current = parent.parent();
        if current == ancestor {
            return true;
        }
    }
    false
}
