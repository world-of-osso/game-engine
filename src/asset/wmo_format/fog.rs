//! WMO MFOG fog at a camera position, ported from WebWowViewerCpp (commit 1a8cccb)
//! `WmoObject::checkFog` (`wowViewerLib/src/engine/objects/wmo/wmoObject.cpp:1631-1778`).
//! Everything here is in WMO file-local coordinates (Z up), like the reference's
//! `m_placementInvertMatrix * cameraPos`.

use super::parser::{WmoFog, WmoGroupHeader, WmoRootData};

/// SMOFog flag bits (`wmoFileHeader.h:219-228`).
const FOG_INFINITE_RADIUS: u32 = 0x1;
const FOG_VOLUME: u32 = 0x1000;
const FOG_DISABLED_WITH_VOLUME: u32 = 0x1_0000;
/// SMOGroupFlags EXTERIOR and EXTERIOR_LIT (`wmoFileHeader.h:71`, `:77`).
const GROUP_EXTERIOR: u32 = 0x8;
const GROUP_EXTERIOR_LIT: u32 = 0x40;

/// One SMOFog_Data: fog end in yards, start as a fraction of the end, authored bytes.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WmoFogData {
    pub end: f32,
    pub start_scalar: f32,
    pub color: [u8; 3],
}

/// The blended fog of an interior group the camera is in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct WmoFogBlend {
    pub fog: WmoFogData,
    pub underwater: WmoFogData,
    /// Distance to the nearest portal of the group; `f32::MAX` when it has none.
    pub dist_to_exit: f32,
}

impl WmoFogBlend {
    /// `DayNightLightHolder.cpp:494`: exterior fog at a portal, WMO fog 25 yd inside.
    pub fn weight(&self) -> f32 {
        (self.dist_to_exit * 0.04).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy)]
struct FogRecord {
    flags: u32,
    position: [f32; 3],
    smaller_radius: f32,
    larger_radius: f32,
    fog: WmoFogData,
    underwater: WmoFogData,
}

#[derive(Clone, Copy)]
struct FogGroup {
    flags: u32,
    portal_start: u16,
    portal_count: u16,
    fog_ids: [u8; 4],
}

/// MFOG records, MOGP fog references and portal polygons of one WMO.
pub struct WmoFogVolume {
    fogs: Vec<FogRecord>,
    /// Per group index; `None` for a group whose file was not loaded.
    groups: Vec<Option<FogGroup>>,
    portal_refs: Vec<u16>,
    portals: Vec<Vec<[f32; 3]>>,
}

impl WmoFogVolume {
    /// `groups` are (group index, MOGP header) pairs.
    pub fn new<'a>(
        root: &WmoRootData,
        groups: impl IntoIterator<Item = (usize, &'a WmoGroupHeader)>,
    ) -> Self {
        let mut fog_groups = vec![None; root.n_groups as usize];
        for (index, header) in groups {
            if let Some(slot) = fog_groups.get_mut(index) {
                *slot = Some(FogGroup {
                    flags: header.flags,
                    portal_start: header.portal_start,
                    portal_count: header.portal_count,
                    fog_ids: header.fog_ids,
                });
            }
        }
        Self {
            fogs: root.fogs.iter().map(fog_record).collect(),
            groups: fog_groups,
            portal_refs: root.portal_refs.iter().map(|r| r.portal_index).collect(),
            portals: root.portals.iter().map(|p| p.vertices.clone()).collect(),
        }
    }

    /// The fog the camera at `camera` (file-local) takes inside `group`, or `None` when
    /// the WMO fog does not apply: no MFOG, an exterior(-lit) group, or disabled record 0
    /// (`wmoObject.cpp:1634`, `:1649`, `:1667-1673`; the caller also requires
    /// `insideInterior`, `DayNightLightHolder.cpp:491`).
    pub fn camera_fog(&self, group: u16, camera: [f32; 3]) -> Option<WmoFogBlend> {
        let fog0 = self.fogs.first()?;
        let group = self.groups.get(group as usize).copied().flatten()?;
        if group.flags & (GROUP_EXTERIOR | GROUP_EXTERIOR_LIT) != 0 {
            return None;
        }
        let dist_to_exit = self.distance_to_exit(group, camera);
        let volume = fog0.flags & FOG_VOLUME != 0;
        if (self.fogs.len() == 1 && !volume)
            || (volume && fog0.flags & FOG_DISABLED_WITH_VOLUME != 0)
        {
            return None;
        }
        // wmoObject.cpp:1680-1686: record 0 is the default and never referenced.
        let referenced: Vec<&FogRecord> = group
            .fog_ids
            .iter()
            .filter(|&&index| index > 0 && usize::from(index) < self.fogs.len())
            .map(|&index| &self.fogs[usize::from(index)])
            .collect();
        let (fog, underwater) = if volume {
            weighted_fog(fog0, &referenced, camera)
        } else {
            sequential_fog(fog0, &referenced, camera)
        };
        Some(WmoFogBlend {
            fog,
            underwater,
            dist_to_exit,
        })
    }

    /// `wmoObject.cpp:1648-1657`: nearest polygon among the group's MOPR portals.
    fn distance_to_exit(&self, group: FogGroup, camera: [f32; 3]) -> f32 {
        let start = usize::from(group.portal_start);
        let end = start + usize::from(group.portal_count);
        let mut distance = f32::MAX;
        for reference in start..end {
            let Some(&portal) = self.portal_refs.get(reference) else {
                break;
            };
            if let Some(polygon) = self.portals.get(usize::from(portal)) {
                distance = distance.min(distance_to_portal_polygon(camera, polygon));
            }
        }
        distance
    }
}

fn fog_record(fog: &WmoFog) -> FogRecord {
    let bytes = |color: [f32; 4]| [0, 1, 2].map(|channel| (color[channel] * 255.0).round() as u8);
    FogRecord {
        flags: fog.flags,
        position: fog.position,
        smaller_radius: fog.smaller_radius,
        larger_radius: fog.larger_radius,
        fog: WmoFogData {
            end: fog.fog_end,
            start_scalar: fog.fog_start_multiplier,
            color: bytes(fog.color_1),
        },
        underwater: WmoFogData {
            end: fog.underwater_fog_end,
            start_scalar: fog.underwater_fog_start_multiplier,
            color: bytes(fog.color_2),
        },
    }
}

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    sub(a, b).iter().map(|c| c * c).sum::<f32>().sqrt()
}

/// `wmoObject.cpp:1540-1545` wmoFogRadialWeight: 1 inside the smaller radius, 0 at the larger.
fn radial_weight(distance: f32, smaller: f32, larger: f32) -> f32 {
    let d = if distance >= 0.0 {
        distance.min(larger)
    } else {
        0.0
    };
    if d < smaller {
        1.0
    } else {
        1.0 - (d - smaller) / (larger - smaller)
    }
}

/// Records whose larger radius contains the camera, with their distance
/// (`wmoObject.cpp:1711-1713`, `:1753-1757`: F_IEBLEND records are skipped).
fn in_range<'a>(records: &[&'a FogRecord], camera: [f32; 3]) -> Vec<(&'a FogRecord, f32)> {
    records
        .iter()
        .map(|record| (*record, distance(record.position, camera)))
        .filter(|(record, dist)| {
            *dist < record.larger_radius && record.flags & FOG_INFINITE_RADIUS == 0
        })
        .collect()
}

/// `wmoObject.cpp:1691-1746` (F_FOGVOLUME): radial-weight average; record 0 fills the
/// weight up to 1. The nearest record's flags (`:1745`) are not carried: nothing reads them.
fn weighted_fog(
    fog0: &FogRecord,
    records: &[&FogRecord],
    camera: [f32; 3],
) -> (WmoFogData, WmoFogData) {
    let mut sum = 0.0;
    let mut fog = Accumulator::default();
    let mut underwater = Accumulator::default();
    for (record, dist) in in_range(records, camera) {
        let weight = radial_weight(dist, record.smaller_radius, record.larger_radius);
        sum += weight;
        fog.add(record.fog, weight);
        underwater.add(record.underwater, weight);
    }
    if sum < 1.0 {
        fog.add(fog0.fog, 1.0 - sum);
        underwater.add(fog0.underwater, 1.0 - sum);
        sum = 1.0;
    }
    (fog.average(sum), underwater.average(sum))
}

#[derive(Default)]
struct Accumulator {
    end: f32,
    start_scalar: f32,
    color: [f32; 3],
}

impl Accumulator {
    fn add(&mut self, data: WmoFogData, weight: f32) {
        self.end += data.end * weight;
        self.start_scalar += data.start_scalar * weight;
        for (sum, byte) in self.color.iter_mut().zip(data.color) {
            *sum += f32::from(byte) / 255.0 * weight;
        }
    }

    /// `wmoObject.cpp:1552-1561` vec3ToImVector rounds the colour back to bytes.
    fn average(&self, sum: f32) -> WmoFogData {
        let inverse = 1.0 / sum;
        WmoFogData {
            end: self.end * inverse,
            start_scalar: self.start_scalar * inverse,
            color: self
                .color
                .map(|c| ((c * inverse).clamp(0.0, 1.0) * 255.0 + 0.5) as u8),
        }
    }
}

/// `wmoObject.cpp:1746-1776`: containing records lerped into record 0, farthest first.
fn sequential_fog(
    fog0: &FogRecord,
    records: &[&FogRecord],
    camera: [f32; 3],
) -> (WmoFogData, WmoFogData) {
    let mut candidates = in_range(records, camera);
    candidates.sort_by(|a, b| b.1.total_cmp(&a.1));
    let (mut fog, mut underwater) = (fog0.fog, fog0.underwater);
    for (record, dist) in candidates {
        let weight = radial_weight(dist, record.smaller_radius, record.larger_radius);
        fog = lerp_fog(fog, record.fog, weight);
        underwater = lerp_fog(underwater, record.underwater, weight);
    }
    (fog, underwater)
}

fn lerp_fog(current: WmoFogData, target: WmoFogData, weight: f32) -> WmoFogData {
    WmoFogData {
        end: (target.end - current.end) * weight + current.end,
        start_scalar: (target.start_scalar - current.start_scalar) * weight + current.start_scalar,
        color: lerp_bytes(current.color, target.color, weight),
    }
}

/// `wmoObject.cpp:1564-1581` lerpImVector: per-byte fixed-point lerp.
fn lerp_bytes(current: [u8; 3], target: [u8; 3], alpha: f32) -> [u8; 3] {
    let a = (alpha * 255.0 + 0.5) as i32;
    if a <= 0 {
        return current;
    }
    if a >= 255 {
        return target;
    }
    [0, 1, 2].map(|channel| {
        let (d, s) = (i32::from(current[channel]), i32::from(target[channel]));
        (d + ((a * (s - d)) >> 8)) as u8
    })
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// `wmoObject.cpp:1588-1622` distanceToPortalPolygon: plane distance when the
/// projection falls inside the polygon, otherwise the distance to its nearest edge.
pub fn distance_to_portal_polygon(point: [f32; 3], polygon: &[[f32; 3]]) -> f32 {
    if polygon.len() < 3 {
        return f32::MAX;
    }
    let normal = cross(sub(polygon[1], polygon[0]), sub(polygon[2], polygon[0]));
    let length = dot(normal, normal).sqrt();
    if length < 1e-6 {
        return f32::MAX;
    }
    let normal = normal.map(|c| c / length);
    let plane_distance = dot(normal, sub(point, polygon[0]));
    let projected = sub(point, normal.map(|c| c * plane_distance));
    let edges = || polygon.iter().zip(polygon.iter().cycle().skip(1));
    let inside = edges().all(|(&a, &b)| dot(cross(sub(b, a), sub(projected, a)), normal) >= 0.0);
    if inside {
        return plane_distance.abs();
    }
    edges()
        .map(|(&a, &b)| {
            let ab = sub(b, a);
            let l2 = dot(ab, ab);
            let t = if l2 > 0.0 {
                (dot(sub(point, a), ab) / l2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            distance(
                point,
                [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t],
            )
        })
        .fold(f32::MAX, f32::min)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEAL: WmoFogData = WmoFogData {
        end: 578.0,
        start_scalar: 0.129,
        color: [21, 80, 99],
    };
    const RED: WmoFogData = WmoFogData {
        end: 100.0,
        start_scalar: 0.5,
        color: [200, 0, 0],
    };

    fn record(flags: u32, position: [f32; 3], radii: (f32, f32), fog: WmoFogData) -> FogRecord {
        FogRecord {
            flags,
            position,
            smaller_radius: radii.0,
            larger_radius: radii.1,
            fog,
            underwater: fog,
        }
    }

    fn interior(fog_ids: [u8; 4], portal_count: u16) -> Option<FogGroup> {
        Some(FogGroup {
            flags: 0x2000,
            portal_start: 0,
            portal_count,
            fog_ids,
        })
    }

    /// A 10x10 doorway in the plane x = 0, y and z in 0..10.
    fn doorway() -> Vec<[f32; 3]> {
        vec![
            [0.0, 0.0, 0.0],
            [0.0, 10.0, 0.0],
            [0.0, 10.0, 10.0],
            [0.0, 0.0, 10.0],
        ]
    }

    fn volume(fogs: Vec<FogRecord>, groups: Vec<Option<FogGroup>>) -> WmoFogVolume {
        WmoFogVolume {
            fogs,
            groups,
            portal_refs: vec![0],
            portals: vec![doorway()],
        }
    }

    #[test]
    fn fog_volume_record_zero_applies_fully_deep_inside_a_portalless_group() {
        let fogs = volume(
            vec![record(FOG_VOLUME, [0.0; 3], (0.0, 0.0), TEAL)],
            vec![interior([0; 4], 0)],
        );
        let blend = fogs.camera_fog(0, [-200.0, 5.0, 3.0]).unwrap();
        assert_eq!(blend.fog, TEAL);
        assert_eq!(blend.dist_to_exit, f32::MAX);
        assert_eq!(blend.weight(), 1.0);
    }

    #[test]
    fn exit_distance_is_the_portal_plane_or_its_nearest_edge() {
        let fogs = volume(
            vec![record(FOG_VOLUME, [0.0; 3], (0.0, 0.0), TEAL)],
            vec![interior([0; 4], 1)],
        );
        // Facing the doorway 10 yd in: weight 0.4.
        let facing = fogs.camera_fog(0, [-10.0, 5.0, 5.0]).unwrap();
        assert_eq!(facing.dist_to_exit, 10.0);
        assert!((facing.weight() - 0.4).abs() < 1e-6);
        // Beside it: distance to the edge y = 10 at (0, 10, 5) is 5.
        let beside = fogs.camera_fog(0, [-3.0, 14.0, 5.0]).unwrap();
        assert_eq!(beside.dist_to_exit, 5.0);
    }

    #[test]
    fn exterior_and_exterior_lit_groups_take_no_wmo_fog() {
        let mut groups = vec![interior([0; 4], 0); 2];
        groups[0].as_mut().unwrap().flags = 0x8;
        groups[1].as_mut().unwrap().flags = 0x2040;
        let fogs = volume(vec![record(FOG_VOLUME, [0.0; 3], (0.0, 0.0), TEAL)], groups);
        assert_eq!(fogs.camera_fog(0, [-5.0, 5.0, 5.0]), None);
        assert_eq!(fogs.camera_fog(1, [-5.0, 5.0, 5.0]), None);
    }

    #[test]
    fn single_plain_record_and_disabled_volume_take_no_wmo_fog() {
        let plain = volume(
            vec![record(0, [0.0; 3], (0.0, 0.0), TEAL)],
            vec![interior([0; 4], 0)],
        );
        assert_eq!(plain.camera_fog(0, [-5.0, 5.0, 5.0]), None);
        let disabled = volume(
            vec![record(
                FOG_VOLUME | FOG_DISABLED_WITH_VOLUME,
                [0.0; 3],
                (0.0, 0.0),
                TEAL,
            )],
            vec![interior([0; 4], 0)],
        );
        assert_eq!(disabled.camera_fog(0, [-5.0, 5.0, 5.0]), None);
    }

    #[test]
    fn fog_volume_averages_a_referenced_record_by_radial_weight() {
        // Camera 30 yd from record 1 with radii 20..40: weight 0.5, record 0 fills 0.5.
        let fogs = volume(
            vec![
                record(FOG_VOLUME, [0.0; 3], (0.0, 0.0), TEAL),
                record(0, [-30.0, 5.0, 5.0], (20.0, 40.0), RED),
            ],
            vec![interior([1, 0, 0, 0], 0)],
        );
        let blend = fogs.camera_fog(0, [-60.0, 5.0, 5.0]).unwrap();
        assert!((blend.fog.end - 339.0).abs() < 1e-3);
        assert!((blend.fog.start_scalar - 0.3145).abs() < 1e-6);
        // ((21 + 200) / 2, 80 / 2, 99 / 2) rounded: (111, 40, 50).
        assert_eq!(blend.fog.color, [111, 40, 50]);
    }

    #[test]
    fn unreferenced_and_out_of_range_records_do_not_blend() {
        let fogs = volume(
            vec![
                record(FOG_VOLUME, [0.0; 3], (0.0, 0.0), TEAL),
                record(0, [-30.0, 5.0, 5.0], (20.0, 40.0), RED),
                record(FOG_INFINITE_RADIUS, [-60.0, 5.0, 5.0], (20.0, 40.0), RED),
            ],
            vec![interior([0, 0, 0, 2], 0), interior([1, 0, 0, 0], 0)],
        );
        assert_eq!(fogs.camera_fog(0, [-60.0, 5.0, 5.0]).unwrap().fog, TEAL);
        assert_eq!(fogs.camera_fog(1, [-80.0, 5.0, 5.0]).unwrap().fog, TEAL);
    }

    #[test]
    fn plain_records_lerp_into_record_zero_farthest_first() {
        let blue = WmoFogData {
            end: 300.0,
            start_scalar: 0.0,
            color: [0, 0, 255],
        };
        let fogs = volume(
            vec![
                record(0, [0.0; 3], (0.0, 0.0), TEAL),
                record(0, [-30.0, 5.0, 5.0], (20.0, 40.0), RED),
                record(0, [-60.0, 5.0, 5.0], (0.0, 10.0), blue),
            ],
            vec![interior([1, 2, 0, 0], 0)],
        );
        // Record 1 (30 yd, weight 0.5) first, then record 2 (0 yd, weight 1) wins.
        assert_eq!(fogs.camera_fog(0, [-60.0, 5.0, 5.0]).unwrap().fog, blue);
        // 10 yd from record 1 (inside its smaller radius) and 20 from record 2: record 1 only.
        assert_eq!(fogs.camera_fog(0, [-40.0, 5.0, 5.0]).unwrap().fog, RED);
        // sqrt(900 + 400) = 36.06 yd from record 1 and 20 from record 2: weight 0.197.
        let partial = fogs.camera_fog(0, [-60.0, 5.0, 25.0]).unwrap();
        let weight = radial_weight(1300f32.sqrt(), 20.0, 40.0);
        assert!((weight - 0.197_224).abs() < 1e-5);
        assert!((partial.fog.end - (578.0 + (100.0 - 578.0) * weight)).abs() < 1e-3);
        // Byte lerp alpha (0.197 * 255 + 0.5) as int = 50, arithmetic shift as in C++:
        // 21 + (8950 >> 8) = 55, 80 + (-4000 >> 8) = 64, 99 + (-4950 >> 8) = 79.
        assert_eq!(partial.fog.color, [55, 64, 79]);
        assert_eq!(lerp_bytes([21, 80, 99], [200, 0, 0], 0.5), [110, 40, 49]);
    }
}
