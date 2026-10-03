pub const DAWN_MINUTES: f32 = 720.0;
pub const DUSK_MINUTES: f32 = 2160.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcSchedule {
    DayOnly,
    NightOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcVisibilityPolicy {
    Always,
    Hidden,
    DeadOnly,
    Scheduled(NpcSchedule),
}

pub fn npc_visibility_policy(template_id: u32) -> NpcVisibilityPolicy {
    match template_id {
        6491 => NpcVisibilityPolicy::DeadOnly, // Spirit Healer
        918 => NpcVisibilityPolicy::Scheduled(NpcSchedule::NightOnly), // Osborne the Night Man
        12783 => NpcVisibilityPolicy::Scheduled(NpcSchedule::DayOnly), // Lieutenant Karter
        32820 => NpcVisibilityPolicy::Hidden,  // Wild Turkey clutter near spawn
        26724 | 26738 | 26739 | 26740..=26745 | 26747..=26759 | 26765 | 33252 => {
            NpcVisibilityPolicy::Hidden // [DND] TAR pedestals and other debug vendors
        }
        _ => NpcVisibilityPolicy::Always,
    }
}

pub fn schedule_is_active(schedule: NpcSchedule, minutes: f32) -> bool {
    match schedule {
        NpcSchedule::DayOnly => (DAWN_MINUTES..DUSK_MINUTES).contains(&minutes),
        NpcSchedule::NightOnly => !(DAWN_MINUTES..DUSK_MINUTES).contains(&minutes),
    }
}

pub fn npc_should_be_visible(
    policy: NpcVisibilityPolicy,
    local_alive: bool,
    game_minutes: f32,
) -> bool {
    match policy {
        NpcVisibilityPolicy::Always => true,
        NpcVisibilityPolicy::Hidden => false,
        NpcVisibilityPolicy::DeadOnly => !local_alive,
        NpcVisibilityPolicy::Scheduled(schedule) => schedule_is_active(schedule, game_minutes),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NpcVisibilityDayPhase {
    Day,
    Night,
}

pub fn npc_visibility_day_phase(minutes: f32) -> NpcVisibilityDayPhase {
    if schedule_is_active(NpcSchedule::DayOnly, minutes) {
        NpcVisibilityDayPhase::Day
    } else {
        NpcVisibilityDayPhase::Night
    }
}
