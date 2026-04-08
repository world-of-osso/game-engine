use std::collections::VecDeque;
use std::time::Duration;

use bevy::prelude::*;
use bevy::ui::{AlignItems, BackgroundColor, JustifyContent, Node, PositionType, Val};
use shared::protocol::AchievementStateUpdate;

use crate::achievements::AchievementCompletionState;
use crate::status::{
    AchievementCompletionEntry, AchievementProgressEntry, AchievementsStatusSnapshot,
};

const TOAST_DURATION_SECS: f32 = 5.0;

#[derive(Resource, Debug, Default)]
pub struct AchievementToastState {
    pub queue: VecDeque<AchievementCompletionEntry>,
    active: Option<AchievementCompletionEntry>,
    timer: Option<Timer>,
}

#[derive(Component)]
struct AchievementToastRoot;

#[derive(Component)]
struct AchievementToastText;

pub struct AchievementPlugin;

impl Plugin for AchievementPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AchievementsStatusSnapshot>();
        app.init_resource::<AchievementCompletionState>();
        app.init_resource::<AchievementToastState>();
        app.add_systems(Startup, spawn_achievement_toast_overlay);
        app.add_systems(Update, update_achievement_toast_overlay);
    }
}

pub fn apply_achievement_state_update(
    status: &mut AchievementsStatusSnapshot,
    completion: &mut AchievementCompletionState,
    toast: &mut AchievementToastState,
    update: AchievementStateUpdate,
) {
    if let Some(snapshot) = update.snapshot {
        status.earned_ids = snapshot.earned_ids.clone();
        completion.earned = snapshot
            .earned_ids
            .into_iter()
            .map(|id| id as i32)
            .collect();
        status.progress = snapshot.progress.iter().map(map_progress_entry).collect();
        completion.progress = snapshot
            .progress
            .into_iter()
            .map(|entry| (entry.achievement_id as i32, (entry.current, entry.required)))
            .collect();
    }
    if let Some(completed) = update.completed {
        let entry = AchievementCompletionEntry {
            achievement_id: completed.achievement_id,
            name: completed.name,
            points: completed.points,
        };
        status.last_completed = Some(entry.clone());
        toast.queue.push_back(entry);
    }
    status.last_server_message = update.message;
    status.last_error = update.error;
}

pub fn reset_runtime(
    completion: &mut AchievementCompletionState,
    toast: &mut AchievementToastState,
) {
    *completion = AchievementCompletionState::default();
    *toast = AchievementToastState::default();
}

fn map_progress_entry(
    entry: &shared::protocol::AchievementProgressSnapshot,
) -> AchievementProgressEntry {
    AchievementProgressEntry {
        achievement_id: entry.achievement_id,
        current: entry.current,
        required: entry.required,
        completed: entry.completed,
    }
}

fn spawn_achievement_toast_overlay(mut commands: Commands) {
    commands
        .spawn((
            AchievementToastRoot,
            Visibility::Hidden,
            BackgroundColor(Color::srgba(0.08, 0.05, 0.02, 0.9)),
            Node {
                position_type: PositionType::Absolute,
                right: Val::Px(28.0),
                top: Val::Px(28.0),
                width: Val::Px(260.0),
                padding: UiRect::axes(Val::Px(18.0), Val::Px(14.0)),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                ..default()
            },
        ))
        .with_children(|parent| {
            parent.spawn((
                AchievementToastText,
                Text::new("Achievement Earned"),
                TextFont {
                    font_size: 22.0,
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.86, 0.45)),
            ));
        });
}

fn update_achievement_toast_overlay(
    time: Res<Time>,
    mut state: ResMut<AchievementToastState>,
    mut visibility_q: Query<&mut Visibility, With<AchievementToastRoot>>,
    mut text_q: Query<&mut Text, With<AchievementToastText>>,
) {
    advance_active_toast(&mut state, time.delta());
    sync_toast_visibility(&state, &mut visibility_q);
    sync_toast_text(&state, &mut text_q);
}

fn advance_active_toast(state: &mut AchievementToastState, delta: Duration) {
    if state.active.is_none() {
        activate_next_toast(state);
    }
    let Some(timer) = state.timer.as_mut() else {
        return;
    };
    timer.tick(delta);
    if !timer.is_finished() {
        return;
    }
    state.active = None;
    state.timer = None;
    activate_next_toast(state);
}

fn activate_next_toast(state: &mut AchievementToastState) {
    let Some(next) = state.queue.pop_front() else {
        return;
    };
    state.active = Some(next);
    state.timer = Some(Timer::from_seconds(TOAST_DURATION_SECS, TimerMode::Once));
}

fn sync_toast_visibility(
    state: &AchievementToastState,
    visibility_q: &mut Query<&mut Visibility, With<AchievementToastRoot>>,
) {
    let visibility = if state.active.is_some() {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut current in visibility_q.iter_mut() {
        *current = visibility;
    }
}

fn sync_toast_text(
    state: &AchievementToastState,
    text_q: &mut Query<&mut Text, With<AchievementToastText>>,
) {
    let text = state
        .active
        .as_ref()
        .map(format_toast_text)
        .unwrap_or_default();
    for mut current in text_q.iter_mut() {
        **current = text.clone();
    }
}

fn format_toast_text(entry: &AchievementCompletionEntry) -> String {
    format!(
        "Achievement Earned\n{}\n{} points",
        entry.name, entry.points
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn achievement_state_update_populates_completion_and_toast_snapshots() {
        let mut status = crate::status::AchievementsStatusSnapshot::default();
        let mut completion = crate::achievements::AchievementCompletionState::default();
        let mut toast = AchievementToastState::default();

        apply_achievement_state_update(
            &mut status,
            &mut completion,
            &mut toast,
            shared::protocol::AchievementStateUpdate {
                snapshot: Some(shared::protocol::AchievementSnapshot {
                    earned_ids: vec![1],
                    progress: vec![shared::protocol::AchievementProgressSnapshot {
                        achievement_id: 2,
                        current: 12,
                        required: 20,
                        completed: false,
                    }],
                }),
                completed: Some(shared::protocol::AchievementToastSnapshot {
                    achievement_id: 1,
                    name: "Level 10".into(),
                    points: 10,
                }),
                message: Some("achievement progress updated".into()),
                error: None,
            },
        );

        assert!(completion.earned.contains(&1));
        assert_eq!(completion.progress.get(&2), Some(&(12, 20)));
        assert_eq!(status.earned_ids, vec![1]);
        assert_eq!(status.progress.len(), 1);
        assert_eq!(status.progress[0].achievement_id, 2);
        assert_eq!(
            status
                .last_completed
                .as_ref()
                .map(|entry| entry.name.as_str()),
            Some("Level 10")
        );
        assert_eq!(toast.queue.len(), 1);
    }
}
