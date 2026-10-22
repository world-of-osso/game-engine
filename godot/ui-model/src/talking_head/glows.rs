//! Retail70448 TalkingHeadUI.xml TalkingHeadsInAnim and atlas604 geometry.
#[derive(Clone, Copy)]
pub struct Glow {
    pub name: &'static str,
    pub atlas: &'static str,
    pub size: [f32; 2],
    pub position: [f32; 2],
    pub alpha: f32,
    pub scale: [f32; 2],
    pub origin: [f32; 2],
}

pub fn sample(elapsed_ms: f64) -> [Glow; 5] {
    let side_y = 9.5 + 10.0 * progress(elapsed_ms, 350.0, 800.0);
    let side_scale = scale(elapsed_ms, 350.0, 700.0, 0.5, 1.6);
    [
        Glow {
            name: "TalkingHeadGlowTopBar",
            atlas: "TalkingHeads-Glow-TopBarGlow",
            size: [81.0, 23.0],
            position: [36.0, 5.5],
            alpha: pulse(elapsed_ms, 150.0, 250.0, 400.0, 500.0),
            scale: [scale(elapsed_ms, 150.0, 250.0, 0.25, 1.5), 1.0],
            origin: [0.5, 0.5],
        },
        Glow {
            name: "TalkingHeadGlowLeftBar",
            atlas: "TalkingHeads-Glow-SideBarGlow",
            size: [23.0, 86.0],
            position: [4.5, side_y],
            alpha: pulse(elapsed_ms, 350.0, 250.0, 850.0, 250.0),
            scale: [1.0, side_scale],
            origin: [0.5, 0.0],
        },
        Glow {
            name: "TalkingHeadGlowRightBar",
            atlas: "TalkingHeads-Glow-SideBarGlow",
            size: [23.0, 86.0],
            position: [125.5, side_y],
            alpha: pulse(elapsed_ms, 350.0, 250.0, 950.0, 250.0),
            scale: [1.0, side_scale],
            origin: [0.5, 0.0],
        },
        Glow {
            name: "TalkingHeadSheen",
            atlas: "TalkingHeads-Glow-Sheen",
            size: [262.0, 34.0],
            position: [102.0, 19.0],
            alpha: pulse(elapsed_ms, 500.0, 500.0, 1000.0, 500.0),
            scale: [scale(elapsed_ms, 500.0, 250.0, 0.25, 1.0), 1.0],
            origin: [0.0, 0.5],
        },
        Glow {
            name: "TalkingHeadTextSheen",
            atlas: "TalkingHeads-Glow-TextSheen",
            size: [251.0, 71.0],
            position: [102.0, 45.0],
            alpha: pulse(elapsed_ms, 750.0, 500.0, 1250.0, 500.0),
            scale: [scale(elapsed_ms, 750.0, 250.0, 0.25, 1.0), 1.0],
            origin: [0.0, 0.5],
        },
    ]
}

fn progress(elapsed: f64, start: f64, duration: f64) -> f32 {
    ((elapsed - start) / duration).clamp(0.0, 1.0) as f32
}

fn pulse(elapsed: f64, start: f64, rise: f64, fall_start: f64, fall: f64) -> f32 {
    if elapsed < start {
        return 0.01;
    }
    if elapsed < fall_start {
        return 0.7 * progress(elapsed, start, rise);
    }
    0.7 * (1.0 - progress(elapsed, fall_start, fall))
}

fn scale(elapsed: f64, start: f64, duration: f64, from: f32, to: f32) -> f32 {
    // Group completion restores transforms; setToFinalAlpha retains only alpha.
    if elapsed < start || elapsed >= 1750.0 {
        return 1.0;
    }
    from + (to - from) * progress(elapsed, start, duration)
}
