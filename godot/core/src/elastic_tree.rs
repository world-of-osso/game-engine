//! Pure model-local tree contact math. Coordinates use engine axes (x, z, -y).
use glam::{DVec3, Quat, Vec3};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capsule {
    pub start: [f32; 3],
    pub end: [f32; 3],
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BranchAnnotation {
    pub name: String,
    pub pivot: [f32; 3],
    pub tip: [f32; 3],
    pub radius: f32,
    pub stiffness: f32,
    pub damping: f32,
    pub max_angle: f32,
    pub region_min: [f32; 3],
    pub region_max: [f32; 3],
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TreeAnnotation {
    pub model_fdid: u32,
    pub trunk: Capsule,
    pub branches: Vec<BranchAnnotation>,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct BranchState {
    pub rotation: Vec3,
    pub angular_velocity: Vec3,
}

#[derive(Debug, Clone, Copy)]
pub struct Contact {
    pub fraction: f32,
    pub point: Vec3,
    pub normal: Vec3,
}

/// Deserialize and validate an annotation before passing it to contact math.
pub fn parse_annotation(json: &str) -> Result<TreeAnnotation, String> {
    let tree: TreeAnnotation = serde_json::from_str(json).map_err(|error| error.to_string())?;
    if tree.model_fdid == 0 {
        return Err("model_fdid must be nonzero".into());
    }
    validate_capsule(&tree.trunk).map_err(|error| format!("trunk: {error}"))?;
    if tree.branches.len() > 2 {
        return Err("prototype supports at most two branches".into());
    }
    for (index, branch) in tree.branches.iter().enumerate() {
        validate_branch(branch).map_err(|error| format!("branch {index}: {error}"))?;
        if tree.branches[..index]
            .iter()
            .any(|other| other.name == branch.name)
        {
            return Err(format!("duplicate branch name: {}", branch.name));
        }
    }
    Ok(tree)
}

fn validate_capsule(capsule: &Capsule) -> Result<(), String> {
    let start = Vec3::from_array(capsule.start);
    let end = Vec3::from_array(capsule.end);
    let length_squared = start.distance_squared(end);
    if !start.is_finite() || !end.is_finite() || !length_squared.is_finite() || length_squared <= 0.
    {
        return Err("endpoints must be finite and distinct".into());
    }
    if !capsule.radius.is_finite() || capsule.radius <= 0. {
        return Err("radius must be finite and positive".into());
    }
    Ok(())
}

fn validate_branch(branch: &BranchAnnotation) -> Result<(), String> {
    validate_capsule(&Capsule {
        start: branch.pivot,
        end: branch.tip,
        radius: branch.radius,
    })?;
    if branch.name.trim().is_empty() {
        return Err("name must not be empty".into());
    }
    if !branch.stiffness.is_finite()
        || branch.stiffness <= 0.
        || !branch.damping.is_finite()
        || branch.damping < 0.
        || !branch.max_angle.is_finite()
        || branch.max_angle <= 0.
        || branch.max_angle > std::f32::consts::PI
    {
        return Err(
            "stiffness must be positive, damping nonnegative, max_angle in (0, pi]; all finite"
                .into(),
        );
    }
    let min = Vec3::from_array(branch.region_min);
    let max = Vec3::from_array(branch.region_max);
    if !min.is_finite() || !max.is_finite() || !min.cmplt(max).all() {
        return Err("region bounds must be finite and strictly ordered".into());
    }
    Ok(())
}

impl TreeAnnotation {
    /// AABB membership selects vertices; smoothstep of axial leverage weights them.
    /// Trunk capsule membership always overrides branch regions. Shared regions
    /// normalize total weight to at most one, leaving the remaining weight rigid.
    pub fn vertex_weights(&self, position: Vec3) -> [f32; 2] {
        let trunk_start = Vec3::from_array(self.trunk.start);
        let trunk_end = Vec3::from_array(self.trunk.end);
        let trunk_point = closest_segment_point(position, trunk_start, trunk_end);
        if position.distance_squared(trunk_point) <= self.trunk.radius * self.trunk.radius {
            return [0.; 2];
        }
        let mut weights = [0.; 2];
        for (weight, branch) in weights.iter_mut().zip(&self.branches) {
            if position.cmpge(Vec3::from_array(branch.region_min)).all()
                && position.cmple(Vec3::from_array(branch.region_max)).all()
            {
                let limb = Vec3::from_array(branch.tip) - Vec3::from_array(branch.pivot);
                let leverage = ((position - Vec3::from_array(branch.pivot)).dot(limb)
                    / limb.length_squared())
                .clamp(0., 1.);
                *weight = leverage * leverage * (3. - 2. * leverage);
            }
        }
        let total: f32 = weights.iter().sum();
        if total > 1. {
            weights.iter_mut().for_each(|weight| *weight /= total);
        }
        weights
    }
}

fn closest_segment_point(point: Vec3, start: Vec3, end: Vec3) -> Vec3 {
    let axis = end - start;
    start + axis * ((point - start).dot(axis) / axis.length_squared()).clamp(0., 1.)
}

impl BranchState {
    /// Apply an angular impulse without changing the current pose. Tangential
    /// contact motion bends the limb; axial motion and pivot contact do not.
    pub fn apply_contact(
        &mut self,
        branch: &BranchAnnotation,
        local_contact: Vec3,
        local_motion: Vec3,
    ) {
        let pivot = Vec3::from_array(branch.pivot);
        let limb = Vec3::from_array(branch.tip) - pivot;
        let leverage = ((local_contact - pivot).dot(limb) / limb.length_squared()).clamp(0., 1.);
        let impulse = limb.as_dvec3().cross(local_motion.as_dvec3()) * f64::from(leverage)
            / (f64::from(limb.length_squared()) * f64::from(branch.stiffness));
        let speed_limit = f64::from(branch.max_angle) * f64::from(branch.stiffness).sqrt();
        self.angular_velocity = (self.angular_velocity.as_dvec3() + impulse)
            .clamp_length_max(speed_limit)
            .as_vec3();
    }

    /// Exact free evolution of rotation'' + damping*rotation' + stiffness*rotation = 0.
    /// Closed-form coefficients remain stable even when a frame spans many seconds.
    /// `delta` must be finite and nonnegative; annotation must be validated.
    pub fn advance(&mut self, branch: &BranchAnnotation, delta: f32) {
        let (position_factor, velocity_factor, derivative_factor) = spring_factors(branch, delta);
        let position = self.rotation.as_dvec3();
        let velocity = self.angular_velocity.as_dvec3();
        self.rotation = (position * position_factor + velocity * velocity_factor).as_vec3();
        self.angular_velocity = (position * (-f64::from(branch.stiffness) * velocity_factor)
            + velocity * derivative_factor)
            .as_vec3();
        let angle = self.rotation.length();
        if angle > branch.max_angle {
            let normal = self.rotation / angle;
            self.rotation = normal * branch.max_angle;
            self.angular_velocity = deflect_motion(self.angular_velocity, -normal, 1.);
        }
    }

    pub fn rotation_quat(&self) -> Quat {
        Quat::from_scaled_axis(self.rotation)
    }

    pub fn collider(&self, branch: &BranchAnnotation) -> Capsule {
        let pivot = Vec3::from_array(branch.pivot);
        let tip = pivot + self.rotation_quat() * (Vec3::from_array(branch.tip) - pivot);
        Capsule {
            start: branch.pivot,
            end: tip.to_array(),
            radius: branch.radius,
        }
    }
}

fn spring_factors(branch: &BranchAnnotation, delta: f32) -> (f64, f64, f64) {
    let time = f64::from(delta);
    let half_damping = f64::from(branch.damping) * 0.5;
    let stiffness = f64::from(branch.stiffness);
    let discriminant = half_damping * half_damping - stiffness;
    let (decayed_cosine, decayed_sine) = if discriminant < 0. {
        let frequency = (-discriminant).sqrt();
        let decay = (-half_damping * time).exp();
        (
            decay * (frequency * time).cos(),
            decay * (frequency * time).sin() / frequency,
        )
    } else if discriminant > 0. {
        let frequency = discriminant.sqrt();
        // Rationalized slow root avoids cancellation in strongly overdamped limbs.
        let slow = (-stiffness / (half_damping + frequency) * time).exp();
        let fast = (-(half_damping + frequency) * time).exp();
        ((slow + fast) * 0.5, (slow - fast) / (2. * frequency))
    } else {
        let decay = (-half_damping * time).exp();
        (decay, time * decay)
    };
    (
        decayed_cosine + half_damping * decayed_sine,
        decayed_sine,
        decayed_cosine - half_damping * decayed_sine,
    )
}

/// Preserve tangential and outward motion. Resistance clamps to [0, 1]; one
/// removes only the inward component (rigid trunk), not the entire trajectory.
pub fn deflect_motion(motion: Vec3, normal: Vec3, resistance: f32) -> Vec3 {
    let normal = normal.normalize_or_zero();
    motion - normal * motion.dot(normal).min(0.) * resistance.clamp(0., 1.)
}

/// Sweep a sphere against a stationary capsule, returning the first entering
/// contact. `point` lies on the stationary capsule, not on its inflated surface.
/// Initial overlap only blocks inward motion; escaping/tangent motion is free.
/// Inputs must be finite, with nonnegative mover radius and a validated capsule.
pub fn sweep_capsule(
    start: Vec3,
    end: Vec3,
    mover_radius: f32,
    capsule: &Capsule,
) -> Option<Contact> {
    let start = start.as_dvec3();
    let motion = end.as_dvec3() - start;
    let base = Vec3::from_array(capsule.start).as_dvec3();
    let tip = Vec3::from_array(capsule.end).as_dvec3();
    let axis = tip - base;
    let axis_squared = axis.length_squared();
    let radius = f64::from(capsule.radius) + f64::from(mover_radius);
    let offset = start - base;
    let axial_position = offset.dot(axis) / axis_squared;
    let closest = base + axis * axial_position.clamp(0., 1.);
    let separation = start - closest;
    if separation.length_squared() <= radius * radius {
        return initial_overlap_contact(separation, closest, motion, capsule.radius);
    }
    let cylinder_fraction = sweep_cylinder_fraction(offset, motion, axis, radius);
    let cap_fraction = sweep_endcap_fraction(start, motion, base, tip, radius);
    let fraction = cylinder_fraction.min(cap_fraction);
    if !fraction.is_finite() {
        return None;
    }
    let center = start + motion * fraction;
    let contact = capsule_surface_contact(center, base, axis, capsule.radius, fraction);
    Some(contact)
}

fn initial_overlap_contact(
    separation: DVec3,
    closest: DVec3,
    motion: DVec3,
    capsule_radius: f32,
) -> Option<Contact> {
    let normal = separation.normalize_or_zero();
    (motion.dot(normal) < 0.).then(|| Contact {
        fraction: 0.,
        normal: normal.as_vec3(),
        point: (closest + normal * f64::from(capsule_radius)).as_vec3(),
    })
}

fn sweep_cylinder_fraction(offset: DVec3, motion: DVec3, axis: DVec3, radius: f64) -> f64 {
    let axis_squared = axis.length_squared();
    let axial_position = offset.dot(axis) / axis_squared;
    let axial_motion = motion.dot(axis) / axis_squared;
    let radial_offset = offset - axis * axial_position;
    let radial_motion = motion - axis * axial_motion;
    if let Some(time) = entering_root(
        radial_motion.length_squared(),
        radial_offset.dot(radial_motion),
        radial_offset.length_squared() - radius * radius,
    ) {
        let height = axial_position + time * axial_motion;
        if (0. ..=1.).contains(&height) {
            return time;
        }
    }
    f64::INFINITY
}

fn sweep_endcap_fraction(start: DVec3, motion: DVec3, base: DVec3, tip: DVec3, radius: f64) -> f64 {
    let mut fraction = f64::INFINITY;
    for center in [base, tip] {
        let offset = start - center;
        if let Some(time) = entering_root(
            motion.length_squared(),
            offset.dot(motion),
            offset.length_squared() - radius * radius,
        ) {
            fraction = fraction.min(time);
        }
    }
    fraction
}

fn capsule_surface_contact(
    center: DVec3,
    base: DVec3,
    axis: DVec3,
    capsule_radius: f32,
    fraction: f64,
) -> Contact {
    let height = ((center - base).dot(axis) / axis.length_squared()).clamp(0., 1.);
    let closest = base + axis * height;
    let normal = (center - closest).normalize_or_zero();
    Contact {
        fraction: fraction as f32,
        normal: normal.as_vec3(),
        point: (closest + normal * f64::from(capsule_radius)).as_vec3(),
    }
}

// Quadratic coefficients use a*t² + 2*b*t + c. Rationalized entry root avoids
// loss of precision when the start is close to the surface at high speed.
fn entering_root(a: f64, b: f64, c: f64) -> Option<f64> {
    if a <= 0. || b >= 0. {
        return None;
    }
    let discriminant = b * b - a * c;
    if discriminant < 0. {
        return None;
    }
    let time = c / (-b + discriminant.sqrt());
    (0. ..=1.).contains(&time).then_some(time)
}
