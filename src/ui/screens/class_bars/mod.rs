//! Retail player class resource bars: each bar's point template, its mixin logic (which
//! animation group plays on each power change) and its layout, resolved per frame into
//! the textures the unit frame draws.

pub mod anim;
mod arcane;
mod druid;
mod essence;
mod monk;
mod paladin;
mod rogue;
mod runes;
mod shards;

use super::inworld_unit_frames_art::AtlasArt;
use crate::status::{ClassBar, ClassBarResource};
use anim::{LayerView, PointTemplate, PointVisual};

/// One drawn texture of the bar, `rect` `(x, y, width, height)` from the bar's top-left.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureView {
    pub name: String,
    pub art: AtlasArt,
    pub rect: (f32, f32, f32, f32),
    pub alpha: f32,
    /// Counter-clockwise degrees about the texture's centre.
    pub rotation: f32,
    pub shown: bool,
    /// A Cooldown frame: the fraction of its cooldown elapsed, swiped clockwise.
    pub swipe: Option<f32>,
}

/// The class bar at one instant: its size, Retail `(topPadding, leftPadding)` inside the
/// player frame's bottom container (both already in the container's units) and textures.
#[derive(Clone, Debug, PartialEq)]
pub struct ClassBarView {
    pub bar: ClassBar,
    pub size: (f32, f32),
    pub padding: (f32, f32),
    pub textures: Vec<TextureView>,
}

/// Textures of `visual` for a point frame whose top-left is `origin`, scaled by `scale`.
fn point_textures(
    name: &str,
    visual: &PointVisual,
    (origin, scale): ((f32, f32), f32),
    size: (f32, f32),
    now: f64,
) -> Vec<TextureView> {
    let template = visual.template();
    template
        .layers
        .iter()
        .zip(visual.view(now))
        .map(|(layer, view): (_, LayerView)| {
            let centre_x = size.0 / 2.0 + layer.offset.0 + view.offset.0;
            let centre_y = size.1 / 2.0 - (layer.offset.1 + view.offset.1);
            let (width, height) = layer.size;
            TextureView {
                name: format!("{name}{}", layer.part),
                art: view.art,
                rect: (
                    origin.0 + (centre_x - width / 2.0) * scale,
                    origin.1 + (centre_y - height / 2.0) * scale,
                    width * scale,
                    height * scale,
                ),
                alpha: view.alpha,
                rotation: view.rotation,
                shown: view.shown,
                swipe: None,
            }
        })
        .collect()
}

pub(crate) fn pip_name(index: usize) -> String {
    format!("PlayerSecondaryResourcePip{index}")
}

/// Retail `HorizontalLayoutFrame` of equal point frames `spacing` apart, left to right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Row {
    pub cell: (f32, f32),
    pub spacing: f32,
    /// `(topPadding, leftPadding)`.
    pub padding: (f32, f32),
    /// Frame `scale` (the rune frame's 0.95).
    pub scale: f32,
}

impl Row {
    fn view<'a>(
        &self,
        bar: ClassBar,
        points: impl ExactSizeIterator<Item = &'a PointVisual>,
        now: f64,
    ) -> ClassBarView {
        let count = points.len() as f32;
        let (cell_w, cell_h) = self.cell;
        let width = count * cell_w + (count - 1.0).max(0.0) * self.spacing;
        let textures = points
            .enumerate()
            .flat_map(|(index, visual)| {
                let x = index as f32 * (cell_w + self.spacing) * self.scale;
                point_textures(
                    &pip_name(index),
                    visual,
                    ((x, 0.0), self.scale),
                    self.cell,
                    now,
                )
            })
            .collect();
        ClassBarView {
            bar,
            size: (width * self.scale, cell_h * self.scale),
            padding: (self.padding.0 * self.scale, self.padding.1 * self.scale),
            textures,
        }
    }
}

/// `SetAlpha` or `Show`/`Hide` of one texture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Set {
    Alpha(usize, f32),
    Shown(usize, bool),
}

fn apply(visual: &mut PointVisual, sets: &[Set]) {
    for set in sets {
        match *set {
            Set::Alpha(layer, alpha) => visual.set_alpha(layer, alpha),
            Set::Shown(layer, shown) => visual.set_shown(layer, shown),
        }
    }
}

/// A point mixin whose `SetActive` resets the point and restarts `activate` or
/// `deactivate` (ArcaneChargeMixin, DruidComboPointMixin, MonkLightEnergyMixin).
#[derive(Debug, PartialEq)]
pub(crate) struct SetActive {
    pub template: &'static PointTemplate,
    /// `ResetVisuals`: the `fxTextures` (alpha 0) and any other resets.
    pub reset: &'static [Set],
    pub activate: (usize, &'static [Set]),
    pub deactivate: (usize, &'static [Set]),
}

/// Points of a `ClassResourceBarTemplate` pool driven by `SetActive(i <= power)`.
#[derive(Debug)]
pub(crate) struct ActivePoints {
    mixin: &'static SetActive,
    points: Vec<(PointVisual, Option<bool>)>,
}

impl ActivePoints {
    pub fn new(mixin: &'static SetActive) -> Self {
        Self {
            mixin,
            points: Vec::new(),
        }
    }

    /// `UpdateMaxPower` re-acquires every point when the max changes (`Setup`: state nil
    /// and `ResetVisuals`), then `UpdatePower` (ClassResourceBarTemplate.lua:118-150).
    pub fn power(&mut self, current: u8, max: u8, now: f64) {
        if self.points.len() != usize::from(max) {
            self.points = (0..max)
                .map(|_| {
                    let mut visual = PointVisual::new(self.mixin.template);
                    self.reset(&mut visual, now);
                    (visual, None)
                })
                .collect();
        }
        for index in 0..self.points.len() {
            self.set_active(index, index < usize::from(current), now);
        }
    }

    pub fn tick(&mut self, now: f64) {
        for (visual, _) in &mut self.points {
            visual.finish(now);
        }
    }

    fn reset(&self, visual: &mut PointVisual, now: f64) {
        visual.stop_all(now);
        apply(visual, self.mixin.reset);
    }

    fn set_active(&mut self, index: usize, active: bool, now: f64) {
        if self.points[index].1 == Some(active) {
            return;
        }
        let mut visual = self.points[index].0.clone();
        self.reset(&mut visual, now);
        let (group, sets) = if active {
            self.mixin.activate
        } else {
            self.mixin.deactivate
        };
        apply(&mut visual, sets);
        visual.restart(group, now);
        self.points[index] = (visual, Some(active));
    }

    pub fn visuals(&self) -> impl ExactSizeIterator<Item = &PointVisual> {
        self.points.iter().map(|(visual, _)| visual)
    }
}

/// A Retail class bar object: `power` runs its `UpdatePower` on a power event
/// (`UNIT_POWER_FREQUENT`, `UNIT_MAXPOWER`, combat changes), `tick` ends finished groups and
/// runs their `OnFinished` chains, `view` resolves what it draws.
trait BarLogic {
    fn power(&mut self, resource: &ClassBarResource, now: f64);
    fn tick(&mut self, now: f64);
    fn view(&self, resource: &ClassBarResource, now: f64) -> ClassBarView;
}

fn new_bar(bar: ClassBar) -> Box<dyn BarLogic> {
    match bar {
        ClassBar::ArcaneCharges => Box::new(arcane::Bar::default()),
        ClassBar::DruidComboPoints => Box::new(druid::Bar::default()),
        ClassBar::Chi => Box::new(monk::Bar::default()),
        ClassBar::RogueComboPoints => Box::new(rogue::Bar::default()),
        ClassBar::SoulShards => Box::new(shards::Bar::default()),
        ClassBar::Essence => Box::new(essence::Bar::default()),
        ClassBar::Runes => Box::new(runes::Bar::default()),
        ClassBar::HolyPower => Box::new(paladin::Bar::default()),
    }
}

/// The player's class bar across frames: the Retail bar object and its points' state.
#[derive(Default)]
pub struct ClassBarAnimator {
    owner: Option<u64>,
    bar: Option<(ClassBarResource, Box<dyn BarLogic>)>,
}

impl std::fmt::Debug for ClassBarAnimator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let resource = self.bar.as_ref().map(|(resource, _)| resource);
        f.debug_struct("ClassBarAnimator")
            .field("resource", &resource)
            .finish()
    }
}

impl ClassBarAnimator {
    /// Feeds an observed client snapshot on the monotonic animator clock.
    pub fn update_received(
        &mut self,
        owner: u64,
        resource: Option<&ClassBarResource>,
        now: f64,
    ) -> Option<ClassBarView> {
        if self.owner != Some(owner) {
            self.owner = Some(owner);
            self.bar = None;
        }
        let Some(resource) = resource else {
            return self.update(None, now);
        };
        let mut received = resource.clone();
        received.dynamics.received_at = self
            .bar
            .as_ref()
            .filter(|(last, _)| same_recharge_sample(last, resource))
            .map_or(now, |(last, _)| last.dynamics.received_at);
        self.update(Some(&received), now)
    }

    /// Feeds the bar's power at `now` (seconds, monotonic) and returns what it draws.
    /// The bar is created on first sight (`Setup`) and dropped when hidden; its
    /// `UpdatePower` runs only when the power (or combat state) changed.
    pub fn update(
        &mut self,
        resource: Option<&ClassBarResource>,
        now: f64,
    ) -> Option<ClassBarView> {
        let Some(resource) = resource.filter(|resource| resource.max > 0) else {
            self.bar = None;
            return None;
        };
        let same_bar = self
            .bar
            .as_ref()
            .is_some_and(|(last, _)| last.bar == resource.bar);
        if !same_bar {
            let mut logic = new_bar(resource.bar);
            logic.power(resource, now);
            self.bar = Some((resource.clone(), logic));
        }
        let (last, logic) = self.bar.as_mut()?;
        logic.tick(now);
        if last != resource {
            *last = resource.clone();
            logic.power(resource, now);
        }
        Some(logic.view(resource, now))
    }
}

/// Combat/spec events redraw the bar but do not re-receive unchanged power timing.
fn same_recharge_sample(last: &ClassBarResource, next: &ClassBarResource) -> bool {
    if last.bar != next.bar {
        return false;
    }
    if next.bar == ClassBar::Runes {
        return last.dynamics.runes == next.dynamics.runes;
    }
    let power = (last.bar, last.current, last.max, last.tenths)
        == (next.bar, next.current, next.max, next.tenths);
    let timing = last.dynamics.partial == next.dynamics.partial
        && last.dynamics.regen_per_sec == next.dynamics.regen_per_sec
        && last.dynamics.runes == next.dynamics.runes;
    power && timing
}

/// The bar once every animation for `resource` has run its course.
pub fn settled_view(resource: &ClassBarResource) -> Option<ClassBarView> {
    let mut animator = ClassBarAnimator::default();
    animator.update(Some(resource), 0.0);
    animator.update(Some(resource), 3600.0)
}
