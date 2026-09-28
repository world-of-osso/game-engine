use super::*;
use bevy::render::render_resource::TextureFormat;

/// A WMO texture with no layer composited onto it (strmwnd_crenlatn_burned, a Stormwind
/// wall) reaches the GPU block-compressed, not decoded to RGBA8.
#[test]
fn uncomposited_wmo_texture_stays_block_compressed() {
    let path = crate::asset::asset_cache::texture(464811).expect("strmwnd_crenlatn_burned");
    let mut images = Assets::<Image>::default();

    let handle = load_wmo_material_image(&path, 0, 0, 0, &mut images).expect("texture loads");

    let image = images.get(&handle).unwrap();
    let (width, height) = (image.width() as usize, image.height() as usize);
    let bytes = image.data.as_ref().unwrap().len();
    match image.texture_descriptor.format {
        TextureFormat::Bc1RgbaUnormSrgb => assert_eq!(bytes, width * height / 2),
        TextureFormat::Bc3RgbaUnormSrgb => assert_eq!(bytes, width * height),
        other => panic!("464811 uploaded as {other:?}, {bytes} bytes"),
    }
}
