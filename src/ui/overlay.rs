//! Render-to-texture overlay for compositing an external UI (wow-ui-sim) on top of the 3D scene.
//!
//! # Architecture
//!
//! wow-ui-sim (iced) renders its UI to an offscreen RGBA buffer each frame.
//! This module accepts that buffer and displays it as a fullscreen quad on a
//! dedicated render layer, drawn after the 3D scene but before any Bevy UI.
//!
//! Input routing: mouse events first hit-test the UI overlay (via alpha > 0 at
//! cursor position). If the UI claims the input, it's forwarded to wow-ui-sim;
//! otherwise it falls through to the 3D camera controls.

use bevy::asset::RenderAssetUsages;
use bevy::camera::visibility::RenderLayers;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

/// Render layer for the iced UI overlay (separate from 3D scene layer 0 and Bevy UI layer 1).
pub const OVERLAY_RENDER_LAYER: usize = 2;

/// Bevy resource holding the latest RGBA frame from wow-ui-sim.
#[derive(Resource)]
pub struct OverlayBuffer {
    /// RGBA8 pixel data, row-major, top-left origin.
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// Incremented each time `pixels` is updated; the sync system skips
    /// texture uploads when the generation hasn't changed.
    pub generation: u64,
}

impl OverlayBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        let size = (width * height * 4) as usize;
        Self {
            pixels: vec![0; size],
            width,
            height,
            generation: 0,
        }
    }

    /// Replace the pixel buffer with new data and bump the generation counter.
    pub fn update(&mut self, pixels: Vec<u8>, width: u32, height: u32) {
        self.pixels = pixels;
        self.width = width;
        self.height = height;
        self.generation += 1;
    }

    /// Returns true if the pixel at (x, y) has alpha > 0 (UI is present there).
    pub fn hit_test(&self, x: f32, y: f32) -> bool {
        let ix = x as u32;
        let iy = y as u32;
        if ix >= self.width || iy >= self.height {
            return false;
        }
        let idx = ((iy * self.width + ix) * 4 + 3) as usize;
        self.pixels.get(idx).copied().unwrap_or(0) > 0
    }
}

/// Marker for the overlay camera.
#[derive(Component)]
pub struct OverlayCamera;

/// Marker for the fullscreen overlay quad.
#[derive(Component)]
pub struct OverlayQuad;

/// Tracks the last uploaded generation to avoid redundant texture copies.
#[derive(Resource, Default)]
struct OverlaySyncState {
    last_generation: u64,
}

/// Plugin that sets up the overlay compositing pipeline.
pub struct OverlayPlugin;

impl Plugin for OverlayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OverlaySyncState>();
        app.add_systems(Startup, setup_overlay);
        app.add_systems(Update, sync_overlay_texture);
    }
}

/// Spawn the overlay camera and fullscreen quad with a transparent texture.
fn setup_overlay(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    buffer: Option<Res<OverlayBuffer>>,
) {
    let (w, h) = buffer
        .as_ref()
        .map(|b| (b.width, b.height))
        .unwrap_or((1920, 1080));

    let image = create_transparent_image(w, h);
    let image_handle = images.add(image);

    // Fullscreen quad displaying the overlay texture.
    commands.spawn((
        Sprite {
            image: image_handle,
            custom_size: Some(Vec2::new(w as f32, h as f32)),
            ..default()
        },
        Transform::default(),
        RenderLayers::layer(OVERLAY_RENDER_LAYER),
        OverlayQuad,
    ));

    // Camera that renders only the overlay layer, after the 3D camera (order 2)
    // and after the existing UI camera (order 1).
    commands.spawn((
        Camera2d,
        Camera {
            order: 2,
            clear_color: ClearColorConfig::None,
            ..default()
        },
        RenderLayers::layer(OVERLAY_RENDER_LAYER),
        OverlayCamera,
    ));
}

/// Upload new pixel data to the overlay texture when the buffer generation changes.
fn sync_overlay_texture(
    buffer: Option<Res<OverlayBuffer>>,
    mut sync_state: ResMut<OverlaySyncState>,
    mut images: ResMut<Assets<Image>>,
    quads: Query<&Sprite, With<OverlayQuad>>,
) {
    let Some(buffer) = buffer else { return };
    if buffer.generation == sync_state.last_generation {
        return;
    }
    sync_state.last_generation = buffer.generation;

    for sprite in &quads {
        if let Some(image) = images.get_mut(&sprite.image) {
            update_image_data(image, &buffer);
        }
    }
}

/// Overwrite image pixel data from the overlay buffer, resizing if needed.
fn update_image_data(image: &mut Image, buffer: &OverlayBuffer) {
    let expected = (buffer.width * buffer.height * 4) as usize;
    if buffer.pixels.len() != expected {
        return;
    }
    if image.size().x != buffer.width || image.size().y != buffer.height {
        *image = create_transparent_image(buffer.width, buffer.height);
    }
    if let Some(data) = image.data.as_mut() {
        data.copy_from_slice(&buffer.pixels);
    }
}

fn create_transparent_image(width: u32, height: u32) -> Image {
    let pixels = vec![0u8; (width * height * 4) as usize];
    Image::new(
        Extent3d { width, height, depth_or_array_layers: 1 },
        TextureDimension::D2,
        pixels,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_buffer_hit_test() {
        let mut buf = OverlayBuffer::new(4, 4);
        // All transparent initially.
        assert!(!buf.hit_test(0.0, 0.0));

        // Set pixel (1, 2) alpha to 255.
        let idx = ((2 * 4 + 1) * 4 + 3) as usize;
        buf.pixels[idx] = 255;
        assert!(buf.hit_test(1.0, 2.0));
        assert!(!buf.hit_test(0.0, 0.0));
    }

    #[test]
    fn overlay_buffer_hit_test_out_of_bounds() {
        let buf = OverlayBuffer::new(4, 4);
        assert!(!buf.hit_test(10.0, 10.0));
        assert!(!buf.hit_test(-1.0, 0.0));
    }

    #[test]
    fn overlay_buffer_update() {
        let mut buf = OverlayBuffer::new(2, 2);
        assert_eq!(buf.generation, 0);

        let new_pixels = vec![255u8; 2 * 2 * 4];
        buf.update(new_pixels.clone(), 2, 2);
        assert_eq!(buf.generation, 1);
        assert_eq!(buf.pixels, new_pixels);
    }

    #[test]
    fn create_transparent_image_correct_size() {
        let img = create_transparent_image(8, 6);
        assert_eq!(img.size().x, 8);
        assert_eq!(img.size().y, 6);
        assert_eq!(img.data.as_ref().unwrap().len(), 8 * 6 * 4);
    }
}
