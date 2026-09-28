//! Periodic GPU memory breakdown from the wgpu allocator (`GAME_ENGINE_GPU_MEMORY_LOG=SECONDS`),
//! also logged as soon as the allocated total moves by `JUMP_BYTES`.
//!
//! Vulkan allocations are grouped by resource label; textures are additionally summed
//! from `RenderAssets<GpuImage>` by format, since Bevy leaves image textures unlabeled.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::TextureFormat;
use bevy::render::renderer::RenderDevice;
use bevy::render::texture::GpuImage;
use bevy::render::{Render, RenderApp, RenderSystems};

const ENV: &str = "GAME_ENGINE_GPU_MEMORY_LOG";
const TOP_GROUPS: usize = 15;
const JUMP_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Resource)]
struct ReportClock {
    start: Instant,
    interval: Duration,
    next: Instant,
    last_allocated: u64,
}

pub(crate) fn configure(app: &mut App) {
    let Ok(value) = std::env::var(ENV) else {
        return;
    };
    let seconds: u64 = value
        .parse()
        .unwrap_or_else(|_| panic!("{ENV}: invalid unsigned SECONDS {value:?}"));
    let interval = Duration::from_secs(seconds.max(1));
    app.get_sub_app_mut(RenderApp)
        .expect("GPU memory report requires RenderApp")
        .insert_resource(ReportClock {
            start: Instant::now(),
            interval,
            next: Instant::now() + interval,
            last_allocated: 0,
        })
        .add_systems(Render, log_gpu_memory.in_set(RenderSystems::Cleanup));
}

fn log_gpu_memory(
    mut clock: ResMut<ReportClock>,
    device: Res<RenderDevice>,
    images: Res<RenderAssets<GpuImage>>,
) {
    let Some(report) = device.wgpu_device().generate_allocator_report() else {
        eprintln!("GPU mem: allocator report unavailable on this backend");
        return;
    };
    let now = Instant::now();
    if now < clock.next && report.total_allocated_bytes.abs_diff(clock.last_allocated) < JUMP_BYTES
    {
        return;
    }
    clock.next = now + clock.interval;
    clock.last_allocated = report.total_allocated_bytes;
    eprintln!(
        "GPU mem: t={:.1}s allocated={} reserved={} blocks={} allocations={}",
        (now - clock.start).as_secs_f64(),
        mib(report.total_allocated_bytes),
        mib(report.total_reserved_bytes),
        report.blocks.len(),
        report.allocations.len()
    );
    let named = report
        .allocations
        .iter()
        .map(|a| (label_class(&a.name), a.size));
    for group in top_groups(named, TOP_GROUPS) {
        eprintln!(
            "GPU mem label {:?}: n={} {}",
            group.key,
            group.count,
            mib(group.bytes)
        );
    }
    let textures = images.iter().map(|(_, image)| {
        let desc = &image.texture_descriptor;
        (
            format!("{:?} mips>1={}", desc.format, desc.mip_level_count > 1),
            texture_bytes(desc.format, desc.size, desc.mip_level_count),
        )
    });
    for group in top_groups(textures, TOP_GROUPS) {
        eprintln!(
            "GPU mem image {}: n={} {}",
            group.key,
            group.count,
            mib(group.bytes)
        );
    }
}

/// Label with digit runs collapsed, so per-instance labels (`hanabi:buffer:slab12:particle`) group.
fn label_class(label: &str) -> String {
    let mut class = String::with_capacity(label.len());
    for ch in label.chars() {
        if !ch.is_ascii_digit() {
            class.push(ch);
        } else if !class.ends_with('#') {
            class.push('#');
        }
    }
    class
}

#[derive(Debug, PartialEq, Eq)]
struct Group<K> {
    key: K,
    count: usize,
    bytes: u64,
}

fn top_groups<K: Eq + std::hash::Hash + Ord>(
    items: impl Iterator<Item = (K, u64)>,
    limit: usize,
) -> Vec<Group<K>> {
    let mut sums: HashMap<K, (usize, u64)> = HashMap::new();
    for (key, bytes) in items {
        let entry = sums.entry(key).or_default();
        entry.0 += 1;
        entry.1 += bytes;
    }
    let mut groups: Vec<_> = sums
        .into_iter()
        .map(|(key, (count, bytes))| Group { key, count, bytes })
        .collect();
    groups.sort_by(|a, b| b.bytes.cmp(&a.bytes).then_with(|| a.key.cmp(&b.key)));
    groups.truncate(limit);
    groups
}

/// Bytes of a texture's full mip chain, from the format's block layout.
fn texture_bytes(
    format: TextureFormat,
    size: bevy::render::render_resource::Extent3d,
    mips: u32,
) -> u64 {
    let (block_w, block_h) = format.block_dimensions();
    let block_bytes = u64::from(format.block_copy_size(None).unwrap_or(4));
    (0..mips)
        .map(|level| {
            let w = (size.width >> level).max(1).div_ceil(block_w);
            let h = (size.height >> level).max(1).div_ceil(block_h);
            u64::from(w) * u64::from(h) * block_bytes
        })
        .sum::<u64>()
        * u64::from(size.depth_or_array_layers)
}

fn mib(bytes: u64) -> String {
    format!("{:.1}MiB", bytes as f64 / (1024.0 * 1024.0))
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::render::render_resource::Extent3d;

    #[test]
    fn groups_sum_by_key_largest_first() {
        let items = [("slab", 10), ("tex", 3), ("slab", 5), ("tex", 1)].into_iter();
        assert_eq!(
            top_groups(items, 1),
            vec![Group {
                key: "slab",
                count: 2,
                bytes: 15
            }]
        );
    }

    #[test]
    fn per_slab_labels_share_a_class() {
        assert_eq!(
            label_class("hanabi:buffer:slab12:particle"),
            "hanabi:buffer:slab#:particle"
        );
    }

    #[test]
    fn bc1_texture_is_eighth_of_rgba8() {
        let size = Extent3d {
            width: 256,
            height: 256,
            depth_or_array_layers: 1,
        };
        assert_eq!(
            texture_bytes(TextureFormat::Rgba8UnormSrgb, size, 1),
            256 * 256 * 4
        );
        assert_eq!(
            texture_bytes(TextureFormat::Bc1RgbaUnormSrgb, size, 1),
            256 * 256 / 2
        );
    }
}
