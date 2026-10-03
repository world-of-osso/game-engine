//! Retail `AnimationGroup` playback for class resource points: textures, their
//! `<Alpha>` / `<FlipBook>` / `<Translation>` / `<Rotation>` animations, and the state a
//! point keeps between groups (`SetAlpha`, `Show`/`Hide`, `setToFinalAlpha`).

use super::super::inworld_unit_frames_art::{AlphaKey, AtlasArt, FlipBook, key_alpha};

/// A texture of a resource point, anchored CENTER on the point.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layer {
    pub part: &'static str,
    pub art: AtlasArt,
    /// Drawn size: the atlas crop under `useAtlasSize`, else the XML `<Size>`.
    pub size: (f32, f32),
    /// CENTER anchor offset as authored (x right, y up).
    pub offset: (f32, f32),
    /// XML `alpha` attribute.
    pub alpha: f32,
    /// Shown unless the XML (or the point's frame) starts hidden.
    pub shown: bool,
}

/// A CENTER-anchored `useAtlasSize` texture at `offset`.
pub const fn layer(part: &'static str, art: AtlasArt, offset: (f32, f32)) -> Layer {
    Layer {
        part,
        art,
        size: art.size(),
        offset,
        alpha: 1.0,
        shown: true,
    }
}

impl Layer {
    pub const fn sized(self, size: (f32, f32)) -> Self {
        Self { size, ..self }
    }

    pub const fn alpha(self, alpha: f32) -> Self {
        Self { alpha, ..self }
    }

    pub const fn hidden(self) -> Self {
        Self {
            shown: false,
            ..self
        }
    }
}

/// Animation `smoothing`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Smoothing {
    None,
    In,
    Out,
    InOut,
}

impl Smoothing {
    /// Eased progress. Retail's exact curves are not documented; these are the quadratic
    /// ease-in/out curves.
    fn apply(self, p: f32) -> f32 {
        match self {
            Self::None => p,
            Self::In => p * p,
            Self::Out => 1.0 - (1.0 - p) * (1.0 - p),
            Self::InOut if p < 0.5 => 2.0 * p * p,
            Self::InOut => 1.0 - 2.0 * (1.0 - p) * (1.0 - p),
        }
    }
}

/// A `<Translation>` (`offset` x right, y up) or `<Rotation>` (`offset.0` degrees,
/// counter-clockwise): applied only while its group plays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Motion {
    pub offset: (f32, f32),
    pub start: f32,
    pub duration: f32,
    pub smoothing: Smoothing,
}

pub const fn translate(dx: f32, dy: f32, start: f32, duration: f32) -> Motion {
    Motion {
        offset: (dx, dy),
        start,
        duration,
        smoothing: Smoothing::None,
    }
}

pub const fn rotate(degrees: f32, start: f32, duration: f32) -> Motion {
    translate(degrees, 0.0, start, duration)
}

impl Motion {
    pub const fn smooth(self, smoothing: Smoothing) -> Self {
        Self { smoothing, ..self }
    }

    fn at(&self, t: f32) -> (f32, f32) {
        if t < self.start {
            return (0.0, 0.0);
        }
        let p = if self.duration > 0.0 {
            ((t - self.start) / self.duration).clamp(0.0, 1.0)
        } else {
            1.0
        };
        let p = self.smoothing.apply(p);
        (self.offset.0 * p, self.offset.1 * p)
    }

    fn end(&self) -> f32 {
        self.start + self.duration
    }
}

/// `looping` of an `AnimationGroup`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Looping {
    None,
    Repeat,
    Bounce,
}

/// One `<AnimationGroup>`; layer indices refer to the point template's layers.
#[derive(Debug, PartialEq)]
pub struct AnimGroup {
    pub alphas: &'static [(usize, AlphaKey)],
    pub flipbooks: &'static [(usize, FlipBook)],
    pub translations: &'static [(usize, Motion)],
    pub rotations: &'static [(usize, Motion)],
    pub looping: Looping,
    pub set_to_final_alpha: bool,
}

/// A non-looping `setToFinalAlpha` group of alphas and flipbooks.
pub const fn group(
    alphas: &'static [(usize, AlphaKey)],
    flipbooks: &'static [(usize, FlipBook)],
) -> AnimGroup {
    AnimGroup {
        alphas,
        flipbooks,
        translations: &[],
        rotations: &[],
        looping: Looping::None,
        set_to_final_alpha: true,
    }
}

impl AnimGroup {
    pub const fn moving(self, translations: &'static [(usize, Motion)]) -> Self {
        Self {
            translations,
            ..self
        }
    }

    pub const fn turning(self, rotations: &'static [(usize, Motion)]) -> Self {
        Self { rotations, ..self }
    }

    pub const fn looping(self, looping: Looping) -> Self {
        Self { looping, ..self }
    }

    pub const fn keep_alpha(self, set_to_final_alpha: bool) -> Self {
        Self {
            set_to_final_alpha,
            ..self
        }
    }

    /// Seconds from start to the end of its last animation.
    pub fn duration(&self) -> f32 {
        let alphas = self.alphas.iter().map(|(_, key)| key.start + key.duration);
        let books = self.flipbooks.iter().map(|(_, book)| book.duration);
        let motions = self
            .translations
            .iter()
            .chain(self.rotations)
            .map(|(_, motion)| motion.end());
        alphas.chain(books).chain(motions).fold(0.0, f32::max)
    }

    fn keys(&self, layer: usize) -> impl Iterator<Item = AlphaKey> + '_ {
        self.alphas
            .iter()
            .filter(move |(index, _)| *index == layer)
            .map(|(_, key)| *key)
    }

    /// The alpha `layer` shows `t` seconds in, `None` while it has no started key.
    fn alpha(&self, layer: usize, t: f32) -> Option<f32> {
        let keys: Vec<AlphaKey> = self.keys(layer).collect();
        keys.iter()
            .any(|key| key.start <= t)
            .then(|| key_alpha(&keys, t))
    }

    fn local_time(&self, elapsed: f32) -> f32 {
        let duration = self.duration();
        if duration <= 0.0 {
            return elapsed;
        }
        match self.looping {
            Looping::None => elapsed,
            Looping::Repeat => elapsed % duration,
            Looping::Bounce => {
                let phase = elapsed % (2.0 * duration);
                if phase > duration {
                    2.0 * duration - phase
                } else {
                    phase
                }
            }
        }
    }
}

/// A point template: its textures in draw order and animation groups.
#[derive(Debug, PartialEq)]
pub struct PointTemplate {
    pub layers: &'static [Layer],
    pub groups: &'static [AnimGroup],
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Playing {
    group: usize,
    /// Start in the caller's monotonic clock, and local animation time at that start.
    origin: f64,
    offset: f32,
    speed: f32,
}

impl Playing {
    fn elapsed(&self, now: f64) -> f32 {
        (self.offset + (now - self.origin) as f32 * self.speed).max(0.0)
    }

    fn end_time(&self, duration: f32) -> f64 {
        if self.speed == 0.0 {
            f64::INFINITY
        } else {
            self.origin + f64::from((duration - self.offset) / self.speed)
        }
    }
}

/// What one texture draws at an instant.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayerView {
    pub art: AtlasArt,
    pub alpha: f32,
    /// Translation from the authored anchor (x right, y up).
    pub offset: (f32, f32),
    /// Counter-clockwise degrees.
    pub rotation: f32,
    pub shown: bool,
}

/// One resource point's texture state and running groups: Retail region alphas and
/// visibility persist between groups; a finished `setToFinalAlpha` group leaves its final
/// alphas, a stopped one the alphas it had reached.
#[derive(Clone, Debug, PartialEq)]
pub struct PointVisual {
    template: &'static PointTemplate,
    alpha: Vec<f32>,
    shown: Vec<bool>,
    /// Flipbook frame a texture keeps after its group ends.
    frames: Vec<Option<AtlasArt>>,
    playing: Vec<Playing>,
}

impl PointVisual {
    pub fn new(template: &'static PointTemplate) -> Self {
        Self {
            template,
            alpha: template.layers.iter().map(|layer| layer.alpha).collect(),
            shown: template.layers.iter().map(|layer| layer.shown).collect(),
            frames: vec![None; template.layers.len()],
            playing: Vec::new(),
        }
    }

    pub fn template(&self) -> &'static PointTemplate {
        self.template
    }

    /// `SetAtlas` on every texture: another template with the same layers and groups.
    pub fn set_template(&mut self, template: &'static PointTemplate) {
        debug_assert_eq!(template.layers.len(), self.template.layers.len());
        self.template = template;
        self.frames = vec![None; template.layers.len()];
    }

    pub fn set_alpha(&mut self, layer: usize, alpha: f32) {
        self.alpha[layer] = alpha;
    }

    pub fn set_shown(&mut self, layer: usize, shown: bool) {
        self.shown[layer] = shown;
    }

    pub fn is_playing(&self, group: usize) -> bool {
        self.playing.iter().any(|playing| playing.group == group)
    }

    /// `AnimationGroup:Restart(false, offset)` at `speed` (`SetAnimationSpeedMultiplier`).
    pub fn play(&mut self, group: usize, now: f64, offset: f32, speed: f32) {
        self.stop(group, now);
        self.playing.push(Playing {
            group,
            origin: now,
            offset,
            speed,
        });
    }

    pub fn restart(&mut self, group: usize, now: f64) {
        self.play(group, now, 0.0, 1.0);
    }

    /// Seconds into `group`, if playing.
    pub fn elapsed(&self, group: usize, now: f64) -> Option<f32> {
        self.playing
            .iter()
            .find(|playing| playing.group == group)
            .map(|playing| playing.elapsed(now))
    }

    /// `AnimationGroup:Stop()`: textures hold the frame the group had reached and, under
    /// `setToFinalAlpha`, its alpha; otherwise they return to the alpha set before it.
    pub fn stop(&mut self, group: usize, now: f64) {
        let Some(index) = self.playing.iter().position(|p| p.group == group) else {
            return;
        };
        let playing = self.playing.remove(index);
        let anim = &self.template.groups[group];
        let t = anim.local_time(playing.elapsed(now));
        self.hold(group, t, anim.set_to_final_alpha);
    }

    /// `AnimationGroup:GetProgress()`: 0..1 of its duration, 0 when not playing.
    pub fn progress(&self, group: usize, now: f64) -> f32 {
        let duration = self.template.groups[group].duration();
        self.elapsed(group, now)
            .map_or(0.0, |elapsed| (elapsed / duration).min(1.0))
    }

    pub fn stop_all(&mut self, now: f64) {
        while let Some(playing) = self.playing.first().copied() {
            self.stop(playing.group, now);
        }
    }

    fn hold(&mut self, group: usize, t: f32, keep_alpha: bool) {
        let anim = &self.template.groups[group];
        if keep_alpha {
            for layer in 0..self.alpha.len() {
                if let Some(alpha) = anim.alpha(layer, t) {
                    self.alpha[layer] = alpha;
                }
            }
        }
        for (layer, book) in anim.flipbooks {
            let art = self.template.layers[*layer].art;
            self.frames[*layer] = Some(book.frame_art(&art, t));
        }
    }

    /// Ends non-looping groups whose time is up; returns each with the time it ended so a
    /// caller can chain an `OnFinished` group from that instant.
    pub fn finish(&mut self, now: f64) -> Vec<(usize, f64)> {
        let groups = self.template.groups;
        let (done, running): (Vec<Playing>, Vec<Playing>) =
            self.playing.iter().partition(|playing| {
                let anim = &groups[playing.group];
                anim.looping == Looping::None && playing.elapsed(now) >= anim.duration()
            });
        self.playing = running;
        done.into_iter()
            .map(|playing| {
                let anim = &groups[playing.group];
                self.hold(playing.group, f32::INFINITY, anim.set_to_final_alpha);
                (playing.group, playing.end_time(anim.duration()))
            })
            .collect()
    }

    pub fn view(&self, now: f64) -> Vec<LayerView> {
        let template = self.template;
        (0..template.layers.len())
            .map(|index| {
                let mut view = LayerView {
                    art: self.frames[index].unwrap_or(template.layers[index].art),
                    alpha: self.alpha[index],
                    offset: (0.0, 0.0),
                    rotation: 0.0,
                    shown: self.shown[index],
                };
                for playing in &self.playing {
                    let anim = &template.groups[playing.group];
                    let t = anim.local_time(playing.elapsed(now));
                    apply_group(&mut view, anim, index, t, template.layers[index].art);
                }
                view
            })
            .collect()
    }
}

fn apply_group(view: &mut LayerView, anim: &AnimGroup, index: usize, t: f32, art: AtlasArt) {
    if let Some(alpha) = anim.alpha(index, t) {
        view.alpha = alpha;
    }
    for (_, book) in anim.flipbooks.iter().filter(|(layer, _)| *layer == index) {
        view.art = book.frame_art(&art, t);
    }
    for (_, motion) in anim
        .translations
        .iter()
        .filter(|(layer, _)| *layer == index)
    {
        let (dx, dy) = motion.at(t);
        view.offset = (view.offset.0 + dx, view.offset.1 + dy);
    }
    for (_, motion) in anim.rotations.iter().filter(|(layer, _)| *layer == index) {
        view.rotation += motion.at(t).0;
    }
}
