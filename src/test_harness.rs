use bevy::prelude::*;

/// Build a minimal headless Bevy app for tests and benchmarks.
///
/// This intentionally avoids windowing and renderer plugins. Callers can add
/// only the extra plugins/resources/systems they need on top.
pub fn headless_app() -> App {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app
}

/// Build a headless Bevy app and let the caller attach only the relevant test
/// plugins, resources, and systems.
pub fn headless_app_with(configure: impl FnOnce(&mut App)) -> App {
    let mut app = headless_app();
    configure(&mut app);
    app
}

/// Advance the app by an explicit number of update cycles.
pub fn run_updates(app: &mut App, cycles: usize) {
    for _ in 0..cycles {
        app.update();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Resource, Default)]
    struct TickCount(u32);

    fn tick(mut count: ResMut<TickCount>) {
        count.0 += 1;
    }

    #[test]
    fn run_updates_advances_the_requested_number_of_cycles() {
        let mut app = headless_app_with(|app| {
            app.init_resource::<TickCount>();
            app.add_systems(Update, tick);
        });

        run_updates(&mut app, 3);

        assert_eq!(app.world().resource::<TickCount>().0, 3);
    }

    #[test]
    fn headless_app_starts_without_asset_server_or_image_assets() {
        let app = headless_app();

        assert!(app.world().get_resource::<AssetServer>().is_none());
        assert!(app.world().get_resource::<Assets<Image>>().is_none());
    }
}
