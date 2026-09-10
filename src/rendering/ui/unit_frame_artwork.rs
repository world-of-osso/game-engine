use std::collections::VecDeque;
use std::path::Path;

pub(super) struct Aperture {
    pub(super) x: u32,
    pub(super) y: u32,
    pub(super) mask: image::RgbaImage,
}

/// Dark translucent shadows belong inside the opening; colored paint forms its border.
fn is_frame_paint(pixel: &image::Rgba<u8>) -> bool {
    pixel[0].max(pixel[1]).max(pixel[2]) > 35 && pixel[3] > 80
}

fn flood_opening(shell: &image::RgbaImage, seed: (u32, u32)) -> Vec<bool> {
    let width = shell.width() as usize;
    let height = shell.height() as usize;
    let mut inside = vec![false; width * height];
    let mut queue = VecDeque::from([seed.1 as usize * width + seed.0 as usize]);
    while let Some(index) = queue.pop_front() {
        let (x, y) = (index % width, index / width);
        if inside[index] || is_frame_paint(shell.get_pixel(x as u32, y as u32)) {
            continue;
        }
        inside[index] = true;
        if x > 0 {
            queue.push_back(index - 1);
        }
        if x + 1 < width {
            queue.push_back(index + 1);
        }
        if y > 0 {
            queue.push_back(index - width);
        }
        if y + 1 < height {
            queue.push_back(index + width);
        }
    }
    inside
}

fn opening_bounds(
    inside: &[bool],
    width: u32,
    height: u32,
) -> Result<(u32, u32, u32, u32), String> {
    let (mut left, mut top, mut right, mut bottom) = (width, height, 0, 0);
    for (index, &included) in inside.iter().enumerate() {
        if included {
            let (x, y) = (index as u32 % width, index as u32 / width);
            left = left.min(x);
            top = top.min(y);
            right = right.max(x);
            bottom = bottom.max(y);
        }
    }
    if left > right
        || top > bottom
        || left == 0
        || top == 0
        || right + 1 == width
        || bottom + 1 == height
    {
        return Err("player-frame artwork opening is empty or not enclosed".into());
    }
    Ok((left, top, right - left + 1, bottom - top + 1))
}

fn extract_aperture(shell: &image::RgbaImage, seed: (u32, u32)) -> Result<Aperture, String> {
    if seed.0 >= shell.width() || seed.1 >= shell.height() {
        return Err("player-frame aperture seed lies outside artwork".into());
    }
    let inside = flood_opening(shell, seed);
    let (x, y, width, height) = opening_bounds(&inside, shell.width(), shell.height())?;
    let mask = image::RgbaImage::from_fn(width, height, |px, py| {
        let included = inside[((y + py) * shell.width() + x + px) as usize];
        image::Rgba([255, 255, 255, if included { 255 } else { 0 }])
    });
    Ok(Aperture { x, y, mask })
}

fn cache_bar_mask(shell: &image::RgbaImage, seed: (u32, u32), path: &Path) -> Result<(), String> {
    if path.is_file() {
        return Ok(());
    }
    let aperture = extract_aperture(shell, seed)?;
    aperture
        .mask
        .save(path)
        .map_err(|error| format!("save {}: {error}", path.display()))
}

pub(super) fn load_portrait_aperture_and_cache_bar_masks() -> Result<Aperture, String> {
    let directory = game_engine::paths::shared_data_path("ui/unitframes");
    let path = directory.join("player-frame-shell.png");
    let shell = image::open(&path)
        .map_err(|error| format!("read {}: {error}", path.display()))?
        .to_rgba8();
    cache_bar_mask(
        &shell,
        (200, 70),
        &directory.join("player-health-aperture-v1.png"),
    )?;
    cache_bar_mask(
        &shell,
        (200, 104),
        &directory.join("player-mana-aperture-v1.png"),
    )?;
    let portrait = extract_aperture(&shell, (73, 70))?;
    if (
        portrait.x,
        portrait.y,
        portrait.mask.width(),
        portrait.mask.height(),
    ) != (18, 13, 111, 113)
    {
        return Err("portrait artwork opening no longer matches player-frame layout".into());
    }
    Ok(portrait)
}

pub(super) fn fit_portrait_to_aperture(
    image: image::RgbaImage,
    aperture: &Aperture,
) -> image::RgbaImage {
    let mut fitted = image::DynamicImage::ImageRgba8(image)
        .resize_to_fill(
            aperture.mask.width(),
            aperture.mask.height(),
            image::imageops::FilterType::Lanczos3,
        )
        .to_rgba8();
    for (pixel, mask) in fitted.pixels_mut().zip(aperture.mask.pixels()) {
        pixel[3] = ((pixel[3] as u16 * mask[3] as u16) / 255) as u8;
    }
    fitted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artwork_masks_cover_actual_openings_including_portrait_lower_right() {
        let path = game_engine::paths::shared_data_path("ui/unitframes/player-frame-shell.png");
        let shell = image::open(path).unwrap().to_rgba8();
        for (seed, expected) in [
            ((73, 70), (18, 13, 111, 113)),
            ((200, 70), (135, 52, 249, 40)),
            ((200, 104), (135, 94, 249, 20)),
        ] {
            let aperture = extract_aperture(&shell, seed).unwrap();
            assert_eq!(
                (
                    aperture.x,
                    aperture.y,
                    aperture.mask.width(),
                    aperture.mask.height()
                ),
                expected
            );
            let local = (seed.0 - aperture.x, seed.1 - aperture.y);
            assert_eq!(aperture.mask.get_pixel(local.0, local.1)[3], 255);
        }
        let portrait = extract_aperture(&shell, (73, 70)).unwrap();
        let source = image::RgbaImage::from_pixel(64, 64, image::Rgba([30, 90, 150, 180]));
        let fitted = fit_portrait_to_aperture(source, &portrait);
        assert_eq!(fitted.get_pixel(99, 101).0, [30, 90, 150, 180]);
        assert_eq!(fitted.get_pixel(0, 0)[3], 0);
    }

    #[test]
    fn unenclosed_artwork_opening_reports_an_error() {
        let shell = image::RgbaImage::from_pixel(8, 8, image::Rgba([0, 0, 0, 0]));
        assert!(extract_aperture(&shell, (4, 4)).is_err());
    }
}
