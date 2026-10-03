use ktx2_rw::{Ktx2Texture, VkFormat};

#[test]
fn rgba_texture_roundtrip_preserves_pixels_and_metadata() {
    let pixels = [255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 64, 17, 33, 65, 0];
    let mut texture = Ktx2Texture::create(2, 2, 1, 1, 1, 1, VkFormat::R8G8B8A8Unorm)
        .expect("create RGBA texture");
    texture
        .set_image_data(0, 0, 0, &pixels)
        .expect("set pixels");
    texture
        .set_metadata("fixture", b"rgba-roundtrip")
        .expect("set metadata");

    let encoded = texture.write_to_memory().expect("serialize KTX2");
    let decoded = Ktx2Texture::from_memory(&encoded).expect("read serialized KTX2");

    assert_eq!(
        (decoded.width(), decoded.height(), decoded.depth()),
        (2, 2, 1)
    );
    assert_eq!(decoded.vk_format(), VkFormat::R8G8B8A8Unorm);
    assert_eq!(
        decoded.get_image_data(0, 0, 0).expect("decoded pixels"),
        pixels
    );
    assert_eq!(
        decoded.get_metadata("fixture").expect("decoded metadata"),
        b"rgba-roundtrip"
    );
}

#[test]
fn malformed_ktx_bytes_are_rejected() {
    assert!(Ktx2Texture::from_memory(b"not a KTX2 texture").is_err());
    let texture =
        Ktx2Texture::create(2, 2, 1, 1, 1, 1, VkFormat::R8G8B8A8Unorm).expect("create texture");
    let encoded = texture.write_to_memory().expect("serialize KTX2");
    assert!(Ktx2Texture::from_memory(&encoded[..12]).is_err());
}
