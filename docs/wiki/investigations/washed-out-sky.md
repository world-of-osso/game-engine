# Washed-Out Sky

The noon InWorld sky rendered near-white and cream instead of deep blue. Four defects combined: LightData colours were decoded with swapped channels and fed as linear, the dome gradient put the pale horizon bands across most of the sky, and the world fog ignored the LightData units and fog colour. The sky also always used LightParams 12, whatever the player's light zone.

## Symptoms

At Northshire and the Stormwind Trade District at noon, the sky was cream or near-white from the horizon to about 60°, with a brownish tint towards the zenith. Distant terrain did not fog: noon fog started at 4,500 yd.

Evidence: `data/diagnostics/sky-20260925/before-*.webp` (master `616123bd`) and `after-*.webp` (`87980917`). Both sets use the same positions, camera yaw 0, and pitch 10/35.

## Root causes

1. **Swapped channels.** `decode_bgr32` took red from the low byte. LightData colours are `0x00RRGGBB` integers (value semantics; the CSV stores decimal integers). Noon LightParams 12 SkyTop `8009 = 0x001F49` is (0,31,73) deep blue; the engine produced (73,31,0), which is brown. The same swap turned Middle (82,127,167) and Band1 (153,220,245) into tan and cream.
2. **Linear bytes.** The bytes were passed to `Color::linear_rgb(byte/255)`. They are sRGB-encoded authored colours. `decode_light_color` now uses `Color::srgb_u8`. Reference: solarityclient `crates/asset/src/database/light/sampling.rs` `unpack_color`.
3. **Dome band heights.** The shader mixed Smog through Band2, Band1 and Middle between 0° and 63°, so SkyTop showed only near the zenith. The client dome has two poles and five rings. Their latitudes (0, 0.17, 0.2, 0.23, 0.24, 0.25, 1 half-turn) use the client's cubic cosine, and the dome is lowered by cos 45° (solarityclient `crates/rendering/src/weather/sky.rs`). The rings sit at 15.8°, 8.5°, 2.2°, 0.2° and -1.6°. The colour stops are SkyTop at the zenith, Middle, Band1, Band2, Smog, then SkyFogColor on the -1.6° ring and the bottom pole. Everything above 16° blends only SkyTop and Middle.
4. **Fog units and colour.** World fog used `FogEnd` raw with Smog/Band2 colours. wowdev DB/LightData calls `FogEnd` "Fog distance multiplied by 36", and solarityclient scales it by 1/36. Noon LightParams 12 (`FogEnd` 18000, `FogScaler` 0.25) now fogs from 125 to 500 yd. The fog colour is SkyFogColor, the same colour as the dome's horizon ring. There is no directional sun glow, because LightData authors no sun fog colour for this row.
5. **Fixed light zone.** `SkyPlugin` loaded only LightParams 12. The sky and fog now sample a blend. It starts with the map's global Light row at weight 1: the last zero-position row, or Light ID 1 if the map has none. Every local light whose `GameFalloffEnd` contains the player is then overlaid, farthest first. A local light's weight is 1 inside `GameFalloffStart` and falls linearly to 0 at `GameFalloffEnd`. Coincident lights (closer than 1/3 yd) put the larger inner radius first. Sources: wowdev DB/Light and solarityclient `sampling.rs` (`candidates`, `local_weight`, `overlay`).

Northshire (-8949.95, -132.49, 83.53) and the Trade District (-8830, 630, 94.5) lie outside every local light on map 0. Both use only Light 1 → LightParams 12, so their visible change comes from causes 1–4. Stormwind Lights 51 and 52 (LightParams 62) overlap at (-8405.36, 548.28, 80.92), with weights 0.504 and 0.605.

## Tonemapping

TonyMcMapface stays. In the corrected Trade District capture, rendered sky pixels match the authored stops within a few levels. SkyMiddle (82,127,167) renders as (82,126,166), and Band1 (153,220,245) as (157,220,240). Nothing shows highlight compression that would justify changing tonemapping or the bloom threshold.

## Proof

- Decode: `light_color_decodes_packed_rgb_as_srgb_bytes` and `light_params_12_noon_row_decodes_authored_colors`. RED produced (146,98,0) for SkyTop.
- Dome: `sky_gradient` tests (ring elevations, band per ring, stop mapping). `environment_map_follows_dome_bands_for_noon_light_params_12`: RED showed 44° IBL at linear (0.91, 0.80, 0.55). The GPU test `sky_dome_renders_band_colors_at_ring_elevations` matches the Rust band data within 1 level at 60°…-20°; RED against the master shader rendered 60° as (90,137,174) instead of (43,76,111).
- Fog: `noon_light_params_12_fog_range_is_in_yards` and the world and weather fog tests.
- Blend: the `light_lookup` fixture tests (weights, ordering, coincident lights, fallback to Light ID 1), the real-data Northshire/Trade District and Lights 51/52 tests, `light_blend_overlays_each_local_on_the_result_so_far`, and `inworld_light_blend_follows_the_local_player_into_overlapping_lights`.

## Open gaps

- **Game time.** The server replicates no game clock, so `GameTime` stays at fixed noon (1440).
- **Sun-facing highlight.** The client's azimuthal sky highlight (`update_packed` `DAY`/`AROUND` curves) is not implemented.
- **Retail ZoneLight.** Retail also has polygon ZoneLight rows. Only `Light` volumes are blended.
- **Byte-space interpolation.** The client interpolates LightData bytes (sRGB space). The engine interpolates keyframes and light blends in linear space.
- **Darker world (resolved).** With correct direct and ambient colours, Bevy PBR rendered characters and foliage dark: its illuminance/exposure calibration had been tuned against the wrong colours. World materials now use Retail's light model instead; see [[retail-lighting]].

## Sources

- `src/rendering/skybox/sky_lightdata.rs`: `decode_light_color`, fog units, `sample_light_blend`.
- `src/rendering/skybox/sky_gradient.rs`: dome profile and band mapping.
- `src/rendering/skybox/mod.rs`, `assets/shaders/sky.wgsl`: dome mesh band attribute, far depth, fog, and the `LightKeyframes` blend.
- `src/rendering/lighting/light_lookup.rs`: `resolve_clear_light_params_blend`.
- [DB/LightData (wowdev)](https://wowdev.wiki/DB/LightData), [DB/Light (wowdev)](https://wowdev.wiki/DB/Light).

## See Also

- [[skybox]]: sky selection and rendering
- [[procedural-sky-dome-visibility]]: the earlier dome winding and late-material fix
