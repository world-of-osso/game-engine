//! Native CalendarFrame lifecycle, the Forever GameTimeFrame action, and server snapshot display.
use std::collections::HashSet;

use game_engine_session::SessionScreen;
use game_engine_ui_model::calendar_data::{ACTION_TOGGLE, CalendarModel, CalendarView};
use game_engine_ui_model::calendar_frame_component::calendar_frame_texture_fdids;
use godot::classes::Time;
use godot::prelude::*;
use shared::protocol::CalendarStateUpdate;

use crate::GameClient;
use crate::frame_error::FrameError;
use crate::ui::RegistryUi;

const CALENDAR_UI_NAME: &str = "CalendarUI";

#[derive(Default)]
pub(crate) struct Calendar {
    model: CalendarModel,
    ui: Option<Gd<RegistryUi>>,
    cached_textures: HashSet<u32>,
    last_view: Option<CalendarView>,
}

impl Calendar {
    pub(crate) fn visit_uis(
        &mut self,
        visit: &mut impl FnMut(&mut Gd<RegistryUi>) -> Result<(), String>,
    ) -> Result<(), String> {
        if let Some(ui) = &mut self.ui {
            visit(ui)?;
        }
        Ok(())
    }

    fn close_window(&mut self) {
        self.model.close();
        self.last_view = None;
        if let Some(ui) = self.ui.take() {
            ui.free();
        }
    }

    pub(crate) fn reset(&mut self) {
        self.close_window();
        self.model.clear_events();
    }
}

impl GameClient {
    pub(super) fn toggle_calendar(&mut self) -> Result<(), FrameError> {
        if self.ui_layout.skin != game_engine_core::ui_layout_data::LayoutSkin::Forever {
            return Ok(());
        }
        self.calendar
            .model
            .apply_action(ACTION_TOGGLE, local_calendar_date()?)?;
        Ok(())
    }

    pub(super) fn update_calendar(&mut self) -> Result<(), FrameError> {
        if self.account.session.screen != SessionScreen::InWorld {
            self.calendar.reset();
            return Ok(());
        }
        if self.ui_layout.skin != game_engine_core::ui_layout_data::LayoutSkin::Forever {
            self.calendar.close_window();
            return Ok(());
        }
        let action = self
            .calendar
            .ui
            .as_mut()
            .map(|ui| ui.bind_mut().pop_action().to_string())
            .unwrap_or_default();
        if !action.is_empty() {
            self.calendar
                .model
                .apply_action(&action, local_calendar_date()?)?;
        }
        self.sync_calendar_ui()?;
        Ok(())
    }

    pub(crate) fn receive_calendar_update(
        &mut self,
        update: CalendarStateUpdate,
    ) -> Result<(), FrameError> {
        if let Some(snapshot) = update.snapshot.as_ref() {
            self.calendar.model.replace_events(snapshot)?;
        }
        if let Some(error) = update.error.as_deref() {
            self.add_world_error(error)?;
        }
        Ok(())
    }

    fn sync_calendar_ui(&mut self) -> Result<(), String> {
        let Some(view) = self.calendar.model.view()? else {
            self.calendar.close_window();
            return Ok(());
        };
        if self.calendar.last_view.as_ref() == Some(&view) {
            return Ok(());
        }
        self.cache_calendar_textures(&view)?;
        let scale = self.effective_ui_scale();
        if let Some(ui) = self.calendar.ui.as_mut() {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)?;
            host.set_state(view.clone())?;
            self.calendar.last_view = Some(view);
            return Ok(());
        }
        self.create_calendar_ui(view, scale)
    }

    fn cache_calendar_textures(&mut self, view: &CalendarView) -> Result<(), String> {
        let pending: Vec<_> = calendar_frame_texture_fdids(view)
            .into_iter()
            .filter(|file_data_id| !self.calendar.cached_textures.contains(file_data_id))
            .collect();
        for file_data_id in pending {
            if self.drawable_fdid(file_data_id) == 0 {
                return Err(format!(
                    "CalendarFrame texture FileDataID {file_data_id} is unavailable"
                ));
            }
            self.calendar.cached_textures.insert(file_data_id);
        }
        Ok(())
    }

    fn create_calendar_ui(&mut self, view: CalendarView, scale: f32) -> Result<(), String> {
        let mut ui = RegistryUi::new_alloc();
        ui.set_name(CALENDAR_UI_NAME);
        self.base_mut().add_child(&ui);
        let shown = {
            let mut host = ui.bind_mut();
            host.set_ui_scale(scale)
                .and_then(|()| host.show_calendar_frame(view.clone()))
        };
        if let Err(error) = shown {
            ui.free();
            return Err(error);
        }
        self.calendar.last_view = Some(view);
        self.calendar.ui = Some(ui);
        Ok(())
    }
}

fn local_calendar_date() -> Result<(i32, u32, u32), String> {
    let date = Time::singleton().get_datetime_dict_from_system();
    let read = |field: &str| {
        date.get(field)
            .map(|value| value.to::<i64>())
            .ok_or_else(|| format!("system date has no {field} field"))
    };
    let year = i32::try_from(read("year")?).map_err(|error| error.to_string())?;
    let month = u32::try_from(read("month")?).map_err(|error| error.to_string())?;
    let day = u32::try_from(read("day")?).map_err(|error| error.to_string())?;
    Ok((year, month, day))
}
