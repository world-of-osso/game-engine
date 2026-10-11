//! Retail Timer.lua / Timer.xml PlayerCountdown presentation, identical in both skins.
use crate::{
    group_countdown::CountdownView,
    ui::{
        strata::FrameStrata,
        widgets::font_string::{FontColor, GameFont},
    },
};
use ui_toolkit::{rsx, widget_def::Element};

const NUMBERS: u32 = 516951;
const GLOW: u32 = 516952;
const HALF_WIDTHS: [f32; 10] = [35.0, 14.0, 33.0, 32.0, 36.0, 32.0, 33.0, 29.0, 31.0, 31.0];

pub fn countdown_frame(view: Option<&CountdownView>) -> Element {
    let Some(view) = view else {
        return Element::default();
    };
    if view.bar_visible() {
        return countdown_bar(view);
    }
    let size = if view.large() {
        [256.0, 170.0]
    } else {
        [128.0, 85.0]
    };
    let digits = countdown_digits(view, size);
    let top = if view.large() { "50%" } else { "155" };
    let translate_y = if view.large() { "-50%" } else { "0" };
    rsx! {
        r#frame {
            name: "TimerTrackerTimer1", width: {size[0]*2.0}, height: {size[1]},
            strata: FrameStrata::High, pos_type: "absolute", left: "50%", top,
            translate_x: "-50%", translate_y,
            {digits}
        }
    }
}

fn countdown_digits(view: &CountdownView, size: [f32; 2]) -> Element {
    let number = view.seconds();
    let digits = if number >= 10 {
        vec![number % 10, (number / 10) % 10]
    } else {
        vec![number]
    };
    let (scale, alpha, glow) = pulse(view.pulse_elapsed);
    let widths: Vec<f32> = digits
        .iter()
        .map(|n| HALF_WIDTHS[*n as usize] / 128.0 * size[0] * scale)
        .collect();
    let mut center = size[0] + widths.iter().sum::<f32>();
    digits
        .iter()
        .enumerate()
        .flat_map(|(index, digit)| {
            center -= widths[index];
            let x = center - size[0] * scale / 2.0;
            let y = (size[1] - size[1] * scale) / 2.0;
            center -= widths[index];
            let left = (digit % 4) as f32 * 0.25;
            let top = (digit / 4) as f32 * 170.0 / 512.0;
            let coords = format!("{left},{},{top},{}", left + 0.25, top + 170.0 / 512.0);
            [
                digit_texture(
                    index,
                    NUMBERS,
                    [x, y, size[0] * scale, size[1] * scale],
                    &coords,
                    alpha,
                ),
                digit_texture(
                    index,
                    GLOW,
                    [x, y, size[0] * scale, size[1] * scale],
                    &coords,
                    glow,
                ),
            ]
        })
        .flatten()
        .collect()
}

fn digit_texture(index: usize, fdid: u32, rect: [f32; 4], coords: &str, alpha: f32) -> Element {
    let suffix = if fdid == NUMBERS { "Digit" } else { "Glow" };
    rsx! { texture {
        name: {DynName(format!("TimerTrackerTimer1{suffix}{}",index+1))},
        texture_fdid: fdid, tex_coords: coords, alpha,
        width: {rect[2]}, height: {rect[3]}, pos_type: "absolute", left: {rect[0]}, top: {rect[1]},
        strata: FrameStrata::High,
    } }
}

/// Timer.xml: grow .25→1 over .3s; hold until .9s; grow 1→1.2 and fade over .1s.
fn pulse(elapsed: f32) -> (f32, f32, f32) {
    if elapsed < 0.3 {
        let progress = (elapsed / 0.3).clamp(0.0, 1.0);
        let ease_out = 1.0 - (1.0 - progress).powi(2);
        return (0.25 + 0.75 * ease_out, 1.0, 1.0 - progress.powi(2));
    }
    let fade = ((elapsed - 0.9) / 0.1).clamp(0.0, 1.0);
    (1.0 + 0.2 * fade, 1.0 - fade, 0.0)
}

fn countdown_bar(view: &CountdownView) -> Element {
    let width = 195.0 * view.update.time_remaining_secs / view.update.duration_secs as f32;
    let seconds = view.update.time_remaining_secs.floor() as u32;
    let text = format!("{}:{:02}", seconds / 60, seconds % 60);
    rsx! { r#frame {
        name: "TimerTrackerTimer1", width: 206.0, height: 26.0, strata: FrameStrata::High,
        pos_type: "absolute", left: "50%", top: 155.0, translate_x: "-50%",
        r#frame { name: "TimerTrackerTimer1StatusBar", width: 195.0, height: 13.0,
            background_color: "0,0,0,0.5", pos_type: "absolute", left: 5.5, top: 2.0,
            r#frame { name: "TimerTrackerTimer1StatusBarFill", width, height: 13.0, background_color: "1,0,0,1" }
            fontstring { name: "TimerTrackerTimer1TimeText", width: 195.0, height: 13.0,
                text: {text.as_str()}, font: GameFont::FrizQuadrata, font_size: 12.0,
                font_color: {FontColor::new(1.0,1.0,1.0,1.0)}, justify_h: "CENTER",
                strata: FrameStrata::High, frame_level: 2.0,
                pos_type: "absolute", left: 0.0, top: 0.0,
            }
        }
        texture { name: "TimerTrackerTimer1Border", texture_fdid: 130874, width: 256.0, height: 64.0,
            pos_type: "absolute", left: -25.0, top: -23.0, strata: FrameStrata::High,
        }
    } }
}
struct DynName(String);
