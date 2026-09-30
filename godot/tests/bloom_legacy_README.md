# Bloom raster differential reference

`bloom_compute_pixels.gd` compares production compute against a test-only,
independently translated legacy raster path, after every downsample, additive
upsample, and final composite. Main owns all Vulkan runs. No native extension
is needed: `Godot --path godot/tests --script bloom_compute_pixels.gd`.

## Source pin

Cargo.lock pins `bevy_post_process` 0.19.0 (crate checksum
`57379b51aa0f1b2066c956263e1f12686d22cf8939cf9644eebf2ee42d3dd460`).
`bloom_legacy_raster.glsl` translates that crate's `src/bloom/bloom.wgsl`,
not the production compute shaders. Upstream Bevy contributors' MIT option
is included verbatim in `bloom_legacy_LICENSE-MIT`.

Source SHA-256:

- `bloom.wgsl`: `cf9eb62f3c7ec9390c554895bbf05a52590c1dd1d9a8806d575782632d978fdf`
- `upsampling_pipeline.rs`: `e631870b5f1511374ab98b0c203ab717d8281b7158a150ee6e3bcedd3e2e65cf`
- `downsampling_pipeline.rs`: `ceb7cad46da2c978f5487877823231dc123c1c1bf0a7ae1e1416af25ac80e182`

Translation specializes the configured uniform scale, full viewport, and
threshold .65/softness .1. WGSL `textureSample` becomes GLSL `texture`;
constant-offset samples become `textureOffset`. Karis weights, threshold,
clamp, 13 taps and tent arithmetic follow upstream operation order.
The fullscreen triangle preserves upstream top-left UVs, adapting clip Y for
Godot's Vulkan RD. The `BloomUniforms` fields are passed as a padded 48-byte
push block instead of a dynamic uniform buffer; no blend weight enters the
fragment shader. Attachment blending follows `upsampling_pipeline.rs`:
RGB = Constant × source + One × destination; alpha = Zero × source + One ×
destination, both Add. Dynamic blend constants are set on each upsample draw;
existing destination contents are loaded, never cleared.

## Why not CPU spatial expectations?

At revision `cee013bd`, ideal CPU bilinear spatial expectations produced eight
failures although constant full-eight-level and black fixtures agreed. The
bounded diagnostic `data/diagnostics/bloom-sampler-probe/run-cleanup-fixed.log`
reported clean exit 0: raster and compute all 13 taps/filter/threshold agreed
at first failure (22,15), while ideal CPU taps differed. CPU filtering of
actual GPU taps agreed. A 1/256 interpolation hypothesis was not exact either.
Those observations invalidate CPU ideal sampling as an exact spatial GPU
oracle; they do not establish a production shader defect or a portable vendor
sampler model.

Raster and compute now each receive their own byte-identical half-float scene
and zero-initialized packed pyramid, then recursively sample only their own
outputs. Both use R11G11B10 levels and RGBA16F final targets. Existing absolute,
relative, one-packed-ULP and one-half-ULP limits remain unchanged; alpha remains
exact, with an additional independent check against uploaded scene alpha.
No automatic truncation calibration, vendor inference, tolerance relaxation,
or alternate oracle selection occurs.

Corpus: edge/colour impulses, intensity .08 and 1, finite black, each at bounded
32-height and full 512-height/eight-level sizing; constant field at full sizing.
Constant-field CPU 1×1 golden math/packed-store quantization remains as a separate
check, without claiming to emulate spatial filtering. Standalone
`bloom_reference.gd`/its CPU self-tests are unchanged.

Engine/script errors are counted through a thread-safe Logger. Invalid RIDs,
command lists, SPIR-V compilation and readback sizes fail explicitly. Per-draw
uniforms/pipelines/framebuffers are freed before textures; fixture textures and
all persistent pipelines/shaders/sampler/local RD are released on failure and
success. Logger remains active through teardown so cleanup errors fail exit.

This changes the test oracle only. Runtime GPU differential proof remains
main-owned; it is not native compositor lifecycle, UI, or full-parity proof.
