use std::path::PathBuf;

// Light.csv stores multiple LightParams circumstances, not a list of fallback
// skybox candidates. The authored skybox resolver should pick an explicit slot,
// not scavenge whichever row happens to resolve through LightSkybox.db2.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LightParamsSlot {
    Clear,
    ClearUnderwater,
    Storm,
    StormUnderwater,
    Death,
}

impl LightParamsSlot {
    pub(crate) const fn index(self) -> usize {
        match self {
            Self::Clear => 0,
            Self::ClearUnderwater => 1,
            Self::Storm => 2,
            Self::StormUnderwater => 3,
            Self::Death => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LightParamsFlags(u32);

impl LightParamsFlags {
    pub const NO_DARKEN_DEPTH: Self = Self(1 << 0);
    pub const DONT_INHERIT_SKYBOX: Self = Self(1 << 1);
    pub const HIDE_SUN: Self = Self(1 << 2);
    pub const HIDE_MOON: Self = Self(1 << 3);
    pub const HIDE_STARS: Self = Self(1 << 4);
    pub const OVERRIDE_CELESTIAL_SPHERE: Self = Self(1 << 5);
    pub const HEIGHT_FOG_ABOVE_PLANE: Self = Self(1 << 6);
    pub const HIDE_CELESTIAL_OBJECT: Self = Self(1 << 7);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for LightParamsFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for LightParamsFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LightSkyboxFlags(u32);

impl LightSkyboxFlags {
    pub const FULL_DAY_SKYBOX: Self = Self(1 << 0);
    pub const COMBINE_PROCEDURAL_AND_SKYBOX: Self = Self(1 << 1);
    pub const PROCEDURAL_FOG_COLOR_BLEND: Self = Self(1 << 2);
    pub const FORCE_SUNSHAFTS: Self = Self(1 << 3);
    pub const DISABLE_USE_SUN_FOG_COLOR: Self = Self(1 << 4);

    pub const fn empty() -> Self {
        Self(0)
    }

    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }
}

impl std::ops::BitOr for LightSkyboxFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for LightSkyboxFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct LightSkyboxMetadata {
    pub(super) fdid: u32,
    pub(super) flags: LightSkyboxFlags,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedLightSkyboxModel {
    pub light_params_id: Option<u32>,
    pub light_params_flags: Option<LightParamsFlags>,
    pub light_skybox_id: u32,
    pub fdid: u32,
    pub wow_path: &'static str,
    pub local_path: PathBuf,
    pub flags: LightSkyboxFlags,
}
