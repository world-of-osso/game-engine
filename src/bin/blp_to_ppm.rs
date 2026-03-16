use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let input = PathBuf::from(
        args.next()
            .expect("usage: blp_to_ppm <input.blp> <output.ppm>"),
    );
    let output = PathBuf::from(
        args.next()
            .expect("usage: blp_to_ppm <input.blp> <output.ppm>"),
    );

    let (pixels, width, height) =
        game_engine::asset::blp::load_blp_rgba(&input).expect("decode blp");
    let mut out = Vec::with_capacity((width * height * 3) as usize + 64);
    out.extend_from_slice(format!("P6\n{} {}\n255\n", width, height).as_bytes());
    for rgba in pixels.chunks_exact(4) {
        out.extend_from_slice(&rgba[..3]);
    }
    std::fs::write(&output, out).expect("write ppm");
}
