use super::{apply_pipelining_policy, pipelining_enabled};
use bevy::app::{App, Main, PluginGroup, PluginGroupBuilder, SubApp};
use bevy::ecs::schedule::{ScheduleLabel, SingleThreadedExecutor};
use bevy::prelude::*;
use bevy::render::RenderApp;
use bevy::render::pipelined_rendering::PipelinedRenderingPlugin;
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, ThreadId};
use std::time::Duration;

struct PipelineGroup;

impl PluginGroup for PipelineGroup {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>().add(PipelinedRenderingPlugin)
    }
}

fn probe_render_thread() -> (App, Receiver<ThreadId>) {
    let (sender, receiver) = mpsc::sync_channel(2);
    let mut render_app = SubApp::new();
    render_app.update_schedule = Some(Main.intern());
    render_app.add_systems(Main, move || {
        sender
            .send(thread::current().id())
            .expect("render probe receiver dropped");
    });

    render_app.edit_schedule(Main, |schedule| {
        schedule.set_executor(SingleThreadedExecutor::new());
    });
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.insert_sub_app(RenderApp, render_app);
    (app, receiver)
}

#[test]
fn disabled_pipelining_keeps_render_frames_on_the_calling_thread() {
    let calling_thread = thread::current().id();
    let (mut app, receiver) = probe_render_thread();
    app.add_plugins(apply_pipelining_policy(
        PipelineGroup::build(PipelineGroup),
        pipelining_enabled(&["--no-pipelined-rendering".to_owned()]),
    ));
    app.finish();
    app.cleanup();
    app.update();

    assert_eq!(
        receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("render frame was not delivered"),
        calling_thread
    );
}

#[test]
fn enabled_pipelining_delivers_render_frames_from_a_different_thread() {
    let calling_thread = thread::current().id();
    let (mut app, receiver) = probe_render_thread();
    app.add_plugins(apply_pipelining_policy(
        PipelineGroup::build(PipelineGroup),
        pipelining_enabled(&[]),
    ));
    app.finish();
    app.cleanup();
    app.update();

    assert_ne!(
        receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("render frame was not delivered"),
        calling_thread
    );
}
