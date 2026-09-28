use game_engine_core::sky_cubemap_data::{
    ENV_MAP_SIZE, build_sky_cubemap, cubemap_direction, sky_band_at_elevation, sky_dome_profile,
    sky_gradient_color,
};

const STOPS: [[f32; 3]; 7] = [
    [0.0, 0.0, 0.0],
    [0.125, 0.0, 0.0],
    [0.25, 0.0, 0.0],
    [0.375, 0.0, 0.0],
    [0.5, 0.0, 0.0],
    [0.625, 0.0, 0.0],
    [0.75, 0.0, 0.0],
];

fn pixel(data: &[u8], face: usize, x: usize, y: usize) -> [f32; 4] {
    let offset = (face * 32 * 32 + y * 32 + x) * 8;
    std::array::from_fn(|channel| {
        let at = offset + channel * 2;
        half::f16::from_le_bytes([data[at], data[at + 1]]).to_f32()
    })
}

#[test]
fn profile_and_band_reach_authored_ring_boundaries() {
    let profile = sky_dome_profile();
    let expected_degrees = [90.0, 15.842, 8.5, 2.158, 0.223, -1.634, -90.0];
    for (index, (point, degrees)) in profile.iter().zip(expected_degrees).enumerate() {
        assert!((point.elevation().to_degrees() - degrees).abs() < 0.01);
        assert!((sky_band_at_elevation(degrees.to_radians()) - index as f32).abs() < 0.01);
        assert!((sky_gradient_color(&STOPS, index as f32)[0] - index as f32 / 8.0).abs() < 1e-6);
    }
    assert!(sky_band_at_elevation(-5f32.to_radians()) > 5.0);
    assert_eq!(sky_gradient_color(&STOPS, -1.0), STOPS[0]);
    assert_eq!(sky_gradient_color(&STOPS, 7.0), STOPS[6]);
}

#[test]
fn face_directions_use_pixel_centers_and_original_face_order() {
    let centers = [
        [1.0, -1.0 / 32.0, -1.0 / 32.0],
        [-1.0, -1.0 / 32.0, 1.0 / 32.0],
        [1.0 / 32.0, 1.0, 1.0 / 32.0],
        [1.0 / 32.0, -1.0, -1.0 / 32.0],
        [1.0 / 32.0, -1.0 / 32.0, 1.0],
        [-1.0 / 32.0, -1.0 / 32.0, -1.0],
    ];
    for (face, expected) in centers.into_iter().enumerate() {
        let dir = cubemap_direction(face as u32, 16, 16);
        let length = (expected.iter().map(|v| v * v).sum::<f32>()).sqrt();
        for channel in 0..3 {
            assert!((dir[channel] - expected[channel] / length).abs() < 1e-6);
        }
    }
    let top_left = cubemap_direction(4, 0, 0);
    let bottom_right = cubemap_direction(4, 31, 31);
    let corner: f32 = 31.0 / 32.0;
    let length = (1.0 + 2.0 * corner * corner).sqrt();
    assert!((top_left[0] + corner / length).abs() < 1e-6);
    assert!((top_left[1] - corner / length).abs() < 1e-6);
    assert!((bottom_right[0] - corner / length).abs() < 1e-6);
    assert!((bottom_right[1] + corner / length).abs() < 1e-6);
    assert!((top_left[2] - 1.0 / length).abs() < 1e-6);
}

#[test]
fn six_faces_are_contiguous_half_float_linear_rgba_without_mip_levels() {
    let data = build_sky_cubemap(&STOPS);
    let face_bytes = (ENV_MAP_SIZE * ENV_MAP_SIZE * 8) as usize;
    assert_eq!(data.len(), face_bytes * 6);
    for face in 0..6 {
        for (x, y) in [(0, 0), (16, 16), (31, 31)] {
            let rgba = pixel(&data, face, x, y);
            let dir = cubemap_direction(face as u32, x as u32, y as u32);
            let expected = sky_gradient_color(&STOPS, sky_band_at_elevation(dir[1].asin()));
            assert_eq!(
                rgba[0],
                half::f16::from_f32(expected[0]).to_f32(),
                "face {face}, ({x},{y})"
            );
            assert_eq!(&rgba[1..], &[0.0, 0.0, 1.0]);
        }
    }
    assert!((0.625..0.75).contains(&pixel(&data, 3, 16, 16)[0]));
    assert!(pixel(&data, 2, 16, 16)[0] < 0.125);
    assert!(pixel(&data, 2, 0, 0)[0] < 0.125);

    let constant = build_sky_cubemap(&[[0.5, 0.25, 0.125]; 7]);
    for face in 0..6 {
        let start = face * face_bytes;
        assert_eq!(
            &constant[start..start + 8],
            &[0, 0x38, 0, 0x34, 0, 0x30, 0, 0x3c],
            "face {face} first pixel is little-endian linear RGBA16F"
        );
        assert_eq!(pixel(&constant, face, 31, 31), [0.5, 0.25, 0.125, 1.0]);
    }
}
