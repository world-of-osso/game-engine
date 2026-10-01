use std::collections::VecDeque;

use bevy::prelude::*;

use crate::game_state_enum::GameState;
use crate::ui::plugin::UiState;

pub use crate::automation_data::{KeyChord, UiAutomationAction, parse_key_chord};

#[derive(Resource, Default)]
pub struct UiAutomationQueue(pub VecDeque<UiAutomationAction>);

impl UiAutomationQueue {
    pub fn push(&mut self, action: UiAutomationAction) {
        self.0.push_back(action);
    }

    pub fn pop(&mut self) -> Option<UiAutomationAction> {
        self.0.pop_front()
    }

    pub fn peek(&self) -> Option<&UiAutomationAction> {
        self.0.front()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UiAutomationWait {
    State {
        target: GameState,
        timeout_secs: f32,
        started_at: f32,
    },
    Frame {
        name: String,
        timeout_secs: f32,
        started_at: f32,
    },
    Delay {
        secs: f32,
        started_at: f32,
    },
}

#[derive(Resource, Debug, Default)]
pub struct UiAutomationRunner {
    pub waiting: Option<UiAutomationWait>,
    pub last_error: Option<String>,
    pub completed: bool,
}

#[derive(Resource, Default)]
pub struct UiAutomationDumpTreeRequest;

#[derive(Resource, Default)]
pub struct UiAutomationDumpUiTreeRequest;

pub struct UiAutomationPlugin;

impl Plugin for UiAutomationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiAutomationQueue>();
        app.init_resource::<UiAutomationRunner>();
        app.add_systems(
            Update,
            process_automation_waits.run_if(automation_wait_work_pending),
        );
        app.add_systems(
            Update,
            process_automation_dump_requests.run_if(automation_dump_pending),
        );
        crate::ui::automation_inworld::register(app);
    }
}

fn automation_wait_work_pending(queue: Res<UiAutomationQueue>) -> bool {
    queue.is_changed()
        || matches!(
            queue.peek(),
            Some(
                UiAutomationAction::WaitForState(..)
                    | UiAutomationAction::WaitForFrame(..)
                    | UiAutomationAction::Wait(_)
            )
        )
}

fn automation_dump_pending(queue: Res<UiAutomationQueue>) -> bool {
    matches!(
        queue.peek(),
        Some(UiAutomationAction::DumpTree | UiAutomationAction::DumpUiTree)
    )
}

fn handle_wait_for_state(
    time: &Time,
    target: GameState,
    timeout_secs: f32,
    state: &State<GameState>,
    queue: &mut UiAutomationQueue,
    runner: &mut UiAutomationRunner,
) {
    runner.completed = false;
    if *state.get() == target {
        queue.pop();
        runner.waiting = None;
        return;
    }
    let started_at = ensure_wait_state(
        runner,
        UiAutomationWait::State {
            target,
            timeout_secs,
            started_at: time.elapsed_secs(),
        },
    );
    if time.elapsed_secs() - started_at > timeout_secs {
        runner.last_error = Some(format!(
            "timed out waiting for state {:?} after {:.2}s",
            target, timeout_secs
        ));
        queue.pop();
        runner.waiting = None;
    }
}

fn handle_wait_for_frame(
    time: &Time,
    name: String,
    timeout_secs: f32,
    ui: &UiState,
    queue: &mut UiAutomationQueue,
    runner: &mut UiAutomationRunner,
) {
    runner.completed = false;
    if ui.registry.get_by_name(&name).is_some() {
        queue.pop();
        runner.waiting = None;
        return;
    }
    let started_at = ensure_wait_state(
        runner,
        UiAutomationWait::Frame {
            name: name.clone(),
            timeout_secs,
            started_at: time.elapsed_secs(),
        },
    );
    if time.elapsed_secs() - started_at > timeout_secs {
        runner.last_error = Some(format!(
            "timed out waiting for frame '{}' after {:.2}s",
            name, timeout_secs
        ));
        queue.pop();
        runner.waiting = None;
    }
}

fn handle_wait_delay(
    time: &Time,
    secs: f32,
    queue: &mut UiAutomationQueue,
    runner: &mut UiAutomationRunner,
) {
    runner.completed = false;
    let started_at = ensure_wait_state(
        runner,
        UiAutomationWait::Delay {
            secs,
            started_at: time.elapsed_secs(),
        },
    );
    if time.elapsed_secs() - started_at >= secs {
        queue.pop();
        runner.waiting = None;
    }
}

fn process_automation_waits(
    time: Res<Time>,
    ui: Res<UiState>,
    state: Res<State<GameState>>,
    mut queue: ResMut<UiAutomationQueue>,
    mut runner: ResMut<UiAutomationRunner>,
) {
    let Some(action) = queue.peek().cloned() else {
        runner.completed = runner.last_error.is_none();
        runner.waiting = None;
        return;
    };
    match action {
        UiAutomationAction::WaitForState(target, timeout_secs) => {
            handle_wait_for_state(&time, target, timeout_secs, &state, &mut queue, &mut runner);
        }
        UiAutomationAction::WaitForFrame(name, timeout_secs) => {
            handle_wait_for_frame(&time, name, timeout_secs, &ui, &mut queue, &mut runner);
        }
        UiAutomationAction::Wait(secs) => handle_wait_delay(&time, secs, &mut queue, &mut runner),
        _ => {}
    }
}

fn ensure_wait_state(runner: &mut UiAutomationRunner, wait: UiAutomationWait) -> f32 {
    match (&runner.waiting, &wait) {
        (
            Some(UiAutomationWait::State {
                target: current_target,
                timeout_secs: current_timeout,
                started_at,
            }),
            UiAutomationWait::State {
                target,
                timeout_secs,
                ..
            },
        ) if current_target == target
            && (*current_timeout - *timeout_secs).abs() < f32::EPSILON =>
        {
            *started_at
        }
        (
            Some(UiAutomationWait::Frame {
                name: current_name,
                timeout_secs: current_timeout,
                started_at,
            }),
            UiAutomationWait::Frame {
                name, timeout_secs, ..
            },
        ) if current_name == name && (*current_timeout - *timeout_secs).abs() < f32::EPSILON => {
            *started_at
        }
        (
            Some(UiAutomationWait::Delay {
                secs: current_secs,
                started_at,
            }),
            UiAutomationWait::Delay { secs, .. },
        ) if (*current_secs - *secs).abs() < f32::EPSILON => *started_at,
        _ => {
            let started_at = match &wait {
                UiAutomationWait::State { started_at, .. } => *started_at,
                UiAutomationWait::Frame { started_at, .. } => *started_at,
                UiAutomationWait::Delay { started_at, .. } => *started_at,
            };
            runner.waiting = Some(wait);
            started_at
        }
    }
}

fn process_automation_dump_requests(
    mut commands: Commands,
    mut queue: ResMut<UiAutomationQueue>,
    mut runner: ResMut<UiAutomationRunner>,
) {
    let Some(action) = queue.peek() else {
        return;
    };
    match action {
        UiAutomationAction::DumpTree => {
            commands.insert_resource(UiAutomationDumpTreeRequest);
            queue.pop();
            runner.completed = false;
        }
        UiAutomationAction::DumpUiTree => {
            commands.insert_resource(UiAutomationDumpUiTreeRequest);
            queue.pop();
            runner.completed = false;
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::event::EventBus;

    fn make_automation_app() -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins);
        app.add_plugins(bevy::state::app::StatesPlugin);
        app.add_plugins(UiAutomationPlugin);
        app.insert_resource(UiState {
            registry: crate::ui::registry::FrameRegistry::new(1920.0, 1080.0),
            event_bus: EventBus::new(),
            focused_frame: None,
        });
        app.init_state::<GameState>();
        app
    }

    #[test]
    fn idle_automation_does_not_access_ui_or_dirty_runner() {
        let mut app = make_automation_app();
        app.update();
        assert!(app.world().resource::<UiAutomationRunner>().completed);
        app.world_mut().remove_resource::<UiState>();
        app.world_mut().clear_trackers();
        for _ in 0..3 {
            app.update();
        }
        assert!(!app.world().is_resource_changed::<UiAutomationRunner>());
    }

    #[test]
    fn active_wait_times_out_without_queue_changes() {
        let mut app = make_automation_app();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
        app.world_mut()
            .resource_mut::<UiAutomationQueue>()
            .push(UiAutomationAction::WaitForFrame(
                "MissingFrame".into(),
                0.15,
            ));
        app.update();
        app.update();
        assert!(
            app.world()
                .resource::<UiAutomationRunner>()
                .waiting
                .is_some()
        );
        app.update();
        assert!(app.world().resource::<UiAutomationQueue>().is_empty());
        app.update();
        let runner = app.world().resource::<UiAutomationRunner>();
        assert!(runner.waiting.is_none());
        assert!(!runner.completed);
        assert!(runner.last_error.as_ref().unwrap().contains("MissingFrame"));
    }

    #[test]
    fn wait_holds_the_next_action_until_its_delay_passes_without_error() {
        let mut app = make_automation_app();
        app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            std::time::Duration::from_millis(100),
        ));
        {
            let mut queue = app.world_mut().resource_mut::<UiAutomationQueue>();
            queue.push(UiAutomationAction::Wait(0.25));
            queue.push(UiAutomationAction::DumpUiTree);
        }
        for _ in 0..3 {
            app.update();
        }
        assert_eq!(
            app.world().resource::<UiAutomationQueue>().peek(),
            Some(&UiAutomationAction::Wait(0.25))
        );
        for _ in 0..2 {
            app.update();
        }
        assert_ne!(
            app.world().resource::<UiAutomationQueue>().peek(),
            Some(&UiAutomationAction::Wait(0.25))
        );
        assert!(
            app.world()
                .resource::<UiAutomationRunner>()
                .last_error
                .is_none()
        );
    }

    #[test]
    fn dump_actions_drain_in_order_and_complete() {
        let mut app = make_automation_app();
        app.world_mut()
            .resource_mut::<UiAutomationQueue>()
            .push(UiAutomationAction::DumpTree);
        app.world_mut()
            .resource_mut::<UiAutomationQueue>()
            .push(UiAutomationAction::DumpUiTree);
        app.update();
        assert!(
            app.world()
                .contains_resource::<UiAutomationDumpTreeRequest>()
        );
        assert!(
            !app.world()
                .contains_resource::<UiAutomationDumpUiTreeRequest>()
        );
        app.update();
        assert!(
            app.world()
                .contains_resource::<UiAutomationDumpUiTreeRequest>()
        );
        app.update();
        assert!(app.world().resource::<UiAutomationRunner>().completed);
    }

    #[test]
    fn queue_preserves_action_order() {
        let mut queue = UiAutomationQueue::default();
        queue.push(UiAutomationAction::ClickFrame("UsernameInput".to_string()));
        queue.push(UiAutomationAction::TypeText("alice".to_string()));

        assert_eq!(
            queue.pop(),
            Some(UiAutomationAction::ClickFrame("UsernameInput".to_string()))
        );
        assert_eq!(
            queue.pop(),
            Some(UiAutomationAction::TypeText("alice".to_string()))
        );
        assert!(queue.is_empty());
    }

    #[test]
    fn wait_for_state_blocks_until_target_state() {
        let mut app = make_automation_app();
        app.world_mut()
            .resource_mut::<UiAutomationQueue>()
            .push(UiAutomationAction::WaitForState(GameState::CharSelect, 1.0));

        app.update();
        assert!(matches!(
            app.world().resource::<UiAutomationRunner>().waiting,
            Some(UiAutomationWait::State {
                target: GameState::CharSelect,
                ..
            })
        ));

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::CharSelect);
        app.update();
        app.update();

        assert!(app.world().resource::<UiAutomationQueue>().is_empty());
        assert!(
            app.world()
                .resource::<UiAutomationRunner>()
                .waiting
                .is_none()
        );
    }

    #[test]
    fn dump_tree_action_sets_request_resource() {
        let mut app = make_automation_app();
        app.world_mut()
            .resource_mut::<UiAutomationQueue>()
            .push(UiAutomationAction::DumpTree);

        app.update();

        assert!(
            app.world()
                .contains_resource::<UiAutomationDumpTreeRequest>()
        );
    }
}
