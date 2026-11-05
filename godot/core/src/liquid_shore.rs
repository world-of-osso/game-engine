//! Baked bottom-depth map for the per-surface liquid shore fade.
//!
//! Retail70688 G10: the shore's stage-6 input is a per-surface map in the surface's own
//! parameter space, combined directly with the interpolated surface height and scaled by
//! `1/Float_11`. It is not the scene depth buffer: the fade is physical water depth
//! (`surface - bottom`), so it must not depend on the camera. The client's own map is not
//! recoverable statically, so this module bakes an equivalent one from the geometry under
//! each liquid and caches it with the instance.
//!
//! Consumers: the ProcFel shore (and later the water family) sample `depth` in yards and
//! scale it by `1/Float_11`.

/// Texels per yard of the baked map. The Forge row's fade is 2.6 yd, so half a yard per
/// texel resolves the band without a visible staircase.
pub const TEXELS_PER_YARD: f32 = 2.0;
/// Upper bound per axis; the Forge pool is about 196 x 188 yd.
pub const MAX_RESOLUTION: usize = 512;

/// One liquid surface's baked bottom-depth map, in the surface's own XZ frame.
#[derive(Clone, Debug, PartialEq)]
pub struct ShoreDepthMap {
    /// Lower XZ corner the map covers.
    pub min: [f32; 2],
    /// Upper XZ corner the map covers.
    pub max: [f32; 2],
    /// `(width, height)` texels, x fastest.
    pub resolution: [usize; 2],
    /// Water depth in yards at each texel centre, clamped at zero.
    pub depth: Vec<f32>,
    /// Texels with no geometry underneath, kept at the deepest baked value so an open
    /// edge reads as open water rather than as a shoreline.
    pub open: usize,
}

impl ShoreDepthMap {
    /// Bake a map over `[min, max]` for a liquid whose surface is `surface`.
    ///
    /// `bottom(x, z)` returns the highest solid surface below `surface` at that column, or
    /// `None` when nothing is under it. The returned depth is `surface - bottom`, so it is
    /// zero at the shoreline and grows into open water.
    pub fn bake<F>(min: [f32; 2], max: [f32; 2], surface: f32, bottom: F) -> Self
    where
        F: FnMut(f32, f32) -> Option<f32>,
    {
        let span = [max[0] - min[0], max[1] - min[1]];
        let resolution = [
            ((span[0] * TEXELS_PER_YARD).ceil() as usize).clamp(2, MAX_RESOLUTION),
            ((span[1] * TEXELS_PER_YARD).ceil() as usize).clamp(2, MAX_RESOLUTION),
        ];
        let mut bottom = bottom;
        let mut depth = vec![0.0; resolution[0] * resolution[1]];
        let mut deepest: f32 = 0.0;
        let mut bare = Vec::new();
        for row in 0..resolution[1] {
            for column in 0..resolution[0] {
                let index = row * resolution[0] + column;
                let (x, z) = texel_centre(min, span, resolution, column, row);
                match bottom(x, z).map(|height| (surface - height).max(0.0)) {
                    Some(value) => {
                        depth[index] = value;
                        deepest = deepest.max(value);
                    }
                    None => bare.push(index),
                }
            }
        }
        for index in bare.iter().copied() {
            depth[index] = deepest;
        }
        Self {
            min,
            max,
            resolution,
            depth,
            open: bare.len(),
        }
    }

    /// Depth in yards at the texel containing `(x, z)`.
    pub fn sample(&self, x: f32, z: f32) -> f32 {
        let span = [self.max[0] - self.min[0], self.max[1] - self.min[1]];
        if span[0] <= 0.0 || span[1] <= 0.0 {
            return 0.0;
        }
        let to_texel = |value: f32, min: f32, span: f32, count: usize| {
            (((value - min) / span) * count as f32)
                .floor()
                .clamp(0.0, (count - 1) as f32) as usize
        };
        let column = to_texel(x, self.min[0], span[0], self.resolution[0]);
        let row = to_texel(z, self.min[1], span[1], self.resolution[1]);
        self.depth[row * self.resolution[0] + column]
    }

    /// The map as 32-bit float texels, one channel, row-major.
    pub fn image_bytes(&self) -> Vec<u8> {
        self.depth
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    /// `(scale.xy, offset.zw)` mapping a local Godot XZ position to this map's UV. The
    /// shader builds it from `VERTEX.xz`, so it is independent of the liquid texture UV and
    /// of the placement transform.
    pub fn local_transform(&self) -> [f32; 4] {
        let span = [self.max[0] - self.min[0], self.max[1] - self.min[1]];
        if span[0] <= 0.0 || span[1] <= 0.0 {
            return [0.0, 0.0, 0.0, 0.0];
        }
        [
            1.0 / span[0],
            1.0 / span[1],
            -self.min[0] / span[0],
            -self.min[1] / span[1],
        ]
    }
}

fn texel_centre(
    min: [f32; 2],
    span: [f32; 2],
    resolution: [usize; 2],
    column: usize,
    row: usize,
) -> (f32, f32) {
    (
        min[0] + (column as f32 + 0.5) / resolution[0] as f32 * span[0],
        min[1] + (row as f32 + 0.5) / resolution[1] as f32 * span[1],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A flat bottom 5 yd below the surface must read as depth 5 everywhere.
    #[test]
    fn flat_bottom_reads_as_constant_depth() {
        let map = ShoreDepthMap::bake([0.0; 2], [10.0; 2], 100.0, |_, _| Some(95.0));
        assert_eq!(map.resolution, [20, 20]);
        assert!(map.depth.iter().all(|depth| (*depth - 5.0).abs() < 1e-6));
        assert_eq!(map.open, 0);
        assert!((map.sample(5.0, 5.0) - 5.0).abs() < 1e-6);
    }

    /// A bottom that rises to the surface must read as zero depth: that is the shoreline.
    #[test]
    fn a_bottom_touching_the_surface_is_the_shore() {
        let map = ShoreDepthMap::bake([0.0; 2], [8.0; 2], 50.0, |x, _| {
            // Depth falls linearly from 4 yd at x=0 to 0 yd at x>=8.
            Some(50.0 - (4.0 - x * 0.5).max(0.0))
        });
        assert!((map.sample(0.2, 4.0) - 4.0).abs() < 0.3);
        // Near the rim the depth is a fraction of a yard, so the 2.6 yd fade reads as shore.
        assert!(
            map.sample(7.75, 4.0) < 0.25,
            "rim depth {}",
            map.sample(7.75, 4.0)
        );
    }

    /// Columns with no geometry below keep the deepest baked value, not a fake shoreline.
    #[test]
    fn open_columns_take_the_deepest_baked_depth() {
        let map = ShoreDepthMap::bake([0.0; 2], [6.0; 2], 20.0, |x, _| {
            if x < 2.0 { Some(17.0) } else { None }
        });
        assert!(map.open > 0);
        assert!(map.depth.iter().all(|depth| (*depth - 3.0).abs() < 1e-6));
    }

    /// `VERTEX.xz` of the local liquid mesh must map onto the baked bounds.
    #[test]
    fn local_transform_maps_local_xz_onto_the_baked_bounds() {
        let map = ShoreDepthMap::bake([-100.0, 200.0], [100.0, 400.0], 30.0, |_, _| Some(30.0));
        let [sx, sy, ox, oy] = map.local_transform();
        assert!(((-100.0) * sx + ox - 0.0).abs() < 1e-5);
        assert!((200.0 * sy + oy - 0.0).abs() < 1e-5);
        assert!((100.0 * sx + ox - 1.0).abs() < 1e-5);
        assert!((400.0 * sy + oy - 1.0).abs() < 1e-5);
    }

    #[test]
    fn image_bytes_are_little_endian_floats() {
        let map = ShoreDepthMap::bake([0.0; 2], [2.0; 2], 10.0, |_, _| Some(7.5));
        let bytes = map.image_bytes();
        assert_eq!(bytes.len(), map.depth.len() * 4);
        assert_eq!(&bytes[..4], &2.5f32.to_le_bytes());
    }
}
