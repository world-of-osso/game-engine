# Benilla / game-engine Ordinary Lighting Fields

Individual ordinary Light, LightData, LightParams and LightSkybox inputs, versus Benilla's older DBC bands. Parser retention is not proof of shader use. ZoneLight/ZoneLightPoint and weather orchestration belong to the parent-owned environment page; BLP belongs to its separate page.

Snapshot: **2026-10-24**. E=`/syncthing/Sync/Projects/world-of-osso/game-engine`, native baseline `354bdb7dbf177047ed46d63168acec531f5c4426` (parent reports relevant source unchanged through `d2c42db72`); B=`/home/osso/Repos/benilla`, `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df`. Static local source inspection only: no execution, imports, build, tests, extraction or visual proof. Pins follow [[benilla-wmo-root-fields]].

Evidence shorthand: **A** E `godot/core/src/lighting_assets.rs`; **F** E `godot/core/src/retail_fog.rs`; **L** E `godot/core/src/rendering/lighting/light_lookup_data.rs`; **S** E `godot/core/src/rendering/skybox/sky_lightdata_data.rs`; **N** E `godot/rust/src/lighting/assets.rs`; **R** E `godot/rust/src/lighting/mod.rs`; **B0** B `crates/benilla-formats/src/light.rs`; **BT** B `crates/benilla-formats/src/light/tables.rs`; **BA** B `crates/benilla-formats/src/light/atmosphere.rs`.

## Modern Light.csv: named reads, not a complete raw header

A:12–41 reads numeric comma-separated columns, without DBC inch conversion. L:25–33 retains the result. All scalar coordinates/radii are finite `f32`; IDs are parsed `u32`.

| CSV column | Cooked field | Located production use |
|---|---|---|
| `ID` | `id: u32` | L default selection uses highest ID; local overlay ordering uses ID. |
| `ContinentID` | `map_id: u32` | L same-map filtering/default selection. |
| `GameCoords_0` | `position[0]: f32` | Local sphere distance and zero-position global predicate. |
| `GameCoords_1` | `position[1]: f32` | Same, independent axis. |
| `GameCoords_2` | `position[2]: f32` | Same, independent axis. |
| `GameFalloffStart` | `falloff_start: f32` | Local sphere full-weight radius. |
| `GameFalloffEnd` | `falloff_end: f32` | Local sphere admission and fade radius. |
| `LightParamsID_0` | `light_params_ids[0]: u32` | Native N:139 uses Clear. |
| `LightParamsID_1` | `light_params_ids[1]: u32` | N:200–222 samples ClearUnderwater for liquid fog. |
| `LightParamsID_2` | `light_params_ids[2]: u32` | Retained; L names Storm, not selected by the inspected N ordinary sample. |
| `LightParamsID_3` | `light_params_ids[3]: u32` | Retained; L names StormUnderwater, same bounded native limit. |
| `LightParamsID_4` | `light_params_ids[4]: u32` | Retained; L names Death, same bounded native limit. |
| `LightParamsID_5` | `light_params_ids[5]: u32` | Retained, no named slot/ordinary native selection established here. |
| `LightParamsID_6` | `light_params_ids[6]: u32` | Same retention/use boundary. |
| `LightParamsID_7` | `light_params_ids[7]: u32` | Same retention/use boundary. |

L `light_params_blend` overlays defaults, polygons, then local spheres and drops zero parameter IDs. These eight slots are circumstances, not eight fallback candidates. N checks ordinary selected keyframe availability. Debug `score_light_row` is not the native overlay algorithm.

## LightData.csv: union of TWO live projections

**Normal** = A:43–100 → `LightDataRow<[f32;3]>` → interpolated `SkyColorSet`. **Fog** = F:114–172 → `FogKeyframe` → sampled `FogResult` → `FogUniforms`. N:281–287 reads the same text into **both**; modern fog-only fields are not absent from native lighting merely because Normal omits them.

Types below are parser/cooked types, not an authenticated DB2 wire schema. **C** means decimal signed-or-unsigned packed color parsed as `i64`, range checked to `[i32::MIN,u32::MAX]`, cast to `u32`, high byte discarded, RGB bytes normalized and sRGB-decoded into linear `[f32;3]` (A:159–186). **f** means finite `f32`. “—” means omitted by that projection, not globally unsupported. All named reads are enumerated; no full raw CSV header/count is claimed.

| Literal column | Type | Normal retention | Fog retention | Native boundary |
|---|---|---|---|---|
| `LightParamID` | `u32` | Map key | Map key | N selected parameter joins both maps. |
| `Time` | f | `time` | `time` | Sorted time keys; cyclic sampling. |
| `DirectColor` | C | `direct_color` | — | N:469–476 → retail direct lighting. |
| `AmbientColor` | C | `ambient_color` | — | Retail ambient lighting. |
| `SkyTopColor` | C | `sky_top` | — | R:54–62,200–208 dome/cubemap stop. |
| `SkyMiddleColor` | C | `sky_middle` | — | Same sky binding. |
| `SkyBand1Color` | C | `sky_band1` | — | Same sky binding. |
| `SkyBand2Color` | C | `sky_band2` | — | Same sky binding. |
| `SkySmogColor` | C | `sky_smog` | — | Same sky binding. |
| `SkyFogColor` | C | `fog_color` | `sky_fog_color` | Sky horizon and scene fog; N:157–160 leans horizon toward end color. |
| `SunColor` | C | `sun_color` | — | N:166 liquid specular AND ordinary sun/moon disc tint through R:214–230,316–331; chain below. |
| `CloudSunColor` | C | `sun_halo_color` | — | Native DEBUG dome halo lighting; exact bindings below, not inworld proof. |
| `CloudEmissiveColor` | C | `cloud_emissive_color` | — | Native DEBUG cloud additive emissive color. |
| `CloudLayer1AmbientColor` | C | `cloud_layer1_ambient_color` | — | Native DEBUG cloud ambient mix endpoint. |
| `CloudLayer2AmbientColor` | C | `cloud_layer2_ambient_color` | — | Native DEBUG cloud ambient mix endpoint. |
| `OceanCloseColor` | C | `ocean_close_color` | — | N:164 → R:125–149 ocean shallow RGBA. |
| `OceanFarColor` | C | `ocean_far_color` | — | Ocean deep RGBA. |
| `RiverCloseColor` | C | `river_close_color` | — | River shallow RGBA. |
| `RiverFarColor` | C | `river_far_color` | — | River deep RGBA. |
| `HorizonAmbientColor` | C | `horizon_ambient_color` | — | N retail horizon ambient. |
| `GroundAmbientColor` | C | `ground_ambient_color` | — | N retail ground ambient. |
| `FogEnd` | f | `fog_end` | `fog_end` | Normal derived start/density handling; Fog normalization, not direct modern far-clip distance. |
| `FogScaler` | f | `fog_scaler`, also `fog_start = FogEnd * FogScaler` | `fog_scaler` | Modern fog range start; underwater S `retail_fog`. |
| `FogDensity` | f | `fog_density` | `fog_density` | Modern density; underwater density ×0.0005. |
| `SunFogStrength` | f | `glow` | `sun_fog_strength` | Fog sun scattering; Normal name `glow` does not prove bloom use. |
| `CloudDensity` | f | `cloud_density` | — | Native DEBUG `cloud_params.x`, threshold/mask; not ordinary inworld proof. |
| `Field_10_0_0_44649_042` | f | `unk1` | — | Retained/interpolated, semantics/use unestablished here. |
| `Field_12_0_0_63854_043` | f | `unk2` | — | Same bounded unknown. |
| `FogHeight` | f | — | `fog_height` | Height-fog uniform. |
| `FogHeightScaler` | f | — | `fog_height_scaler` | Height-fog range/rate derivation. |
| `FogHeightDensity` | f | — | `fog_height_density` | Height-fog density. |
| `FogZScalar` | f | — | `fog_z_scalar` | Fog vertical scalar. |
| `MainFogStartDist` | f | — | `main_fog_start` | Artistic main range. |
| `MainFogEndDist` | f | — | `main_fog_end` | Artistic main range. |
| `SunFogAngle` | f | — | `sun_fog_angle` | Sun scattering angle. |
| `EndFogColorDistance` | f | — | `end_fog_color_distance` | End-color range and N sky-fog blend. |
| `FogStartOffset` | f | — | `fog_start_offset` | Fog start offset. |
| `EndFogColor` | C | — | `end_fog_color` | End-color uniform and N horizon adjustment. |
| `SunFogColor` | C | — | `sun_fog_color` | Sun-scattering uniform. |
| `FogHeightColor` | C | — | `fog_height_color` | Height-color uniform. |
| `EndFogHeightColor` | C | — | `end_fog_height_color` | Height-end-color uniform. |
| `FogHeightCoefficients_0` | f | — | `height_coefficients[0]` | Constant coefficient; shader order reversed. |
| `FogHeightCoefficients_1` | f | — | `height_coefficients[1]` | x coefficient. |
| `FogHeightCoefficients_2` | f | — | `height_coefficients[2]` | x² coefficient. |
| `FogHeightCoefficients_3` | f | — | `height_coefficients[3]` | x³ coefficient. |
| `MainFogCoefficients_0` | f | — | `main_coefficients[0]` | Constant coefficient; shader order reversed. |
| `MainFogCoefficients_1` | f | — | `main_coefficients[1]` | x coefficient. |
| `MainFogCoefficients_2` | f | — | `main_coefficients[2]` | x² coefficient. |
| `MainFogCoefficients_3` | f | — | `main_coefficients[3]` | x³ coefficient. |
| `HeightDensityFogCoeff_0` | f | — | `height_density_coefficients[0]` | Constant coefficient; shader order reversed. |
| `HeightDensityFogCoeff_1` | f | — | `height_density_coefficients[1]` | x coefficient. |
| `HeightDensityFogCoeff_2` | f | — | `height_density_coefficients[2]` | x² coefficient. |
| `HeightDensityFogCoeff_3` | f | — | `height_density_coefficients[3]` | x³ coefficient. |

### Parsing loss and derived/native boundaries

Input census remains **53/53 LightData union names**: Normal28, Fog32, overlap7 (Normal-only21, Fog-only25). This is named-reader coverage, not raw-schema completeness.

S:128–190 distinguishes normal cooked fog from A's raw retention and F's full retail projection. A retains `fog_start = FogEnd * FogScaler`; S `row_colors` divides both `fog_end` and `fog_start` by `LIGHT_DATA_FOG_UNITS_PER_YARD = 36`, and clamps raw scaler to [-1,1] before interpolation. `keyframe_fog_density` retains positive authored density; otherwise sets `far = min(FOG_FAR_CLIP,700)-200`, `end = max(raw fog_end,10)`, `difference = end-end*clamp(raw scaler,-1,1)`. Density becomes 1.5 when difference exceeds far or is nonpositive, otherwise `(1-difference/far)*5.5+1.5`. After interpolation, `floor_fog` floors density at **0.9**; scaler at **-0.2** if either bracket row authored positive density, otherwise **0**. These derived normal `SkyColorSet` values are not extra CSV inputs or F's far-clip-based range.

Ordinary disc tint is statically wired: A:78 `SunColor` → linear `sun_color` → N:166 `WaterLight.specular = linear_to_authored_rgb(sky.sun_color)` → R:214–230 retained water sample / `sync_planets(..., values.3.specular)` → R:316–331 `Planets::sync` → `lighting/planets.rs:70–85` shown-disc material `color` → `godot/shaders/sky_planet.gdshader:9–10,29–31` `ALBEDO = gamma_to_linear(texel.rgb * color)`. This establishes sun/moon disc tint wiring, not pixel proof.

Native **DEBUG**, not ordinary inworld: `skybox_debug/environment.rs:99–111` parses the same Normal LightData and samples LightParams12 at fixed noon. Conditional dome creation (:69–96) loads `skybox_debug_procedural.gdshader` (:209–221). `bind_dome_colors` (:225–245) binds `sun_halo_color`, `cloud_emissive_color`, `cloud_layer1_ambient_color`, `cloud_layer2_ambient_color` individually; `cloud_density` binds `cloud_params.x` (:218–220). Shader :13–19,44–58 uses density for threshold/mask, layer2/layer1 for ambient interpolation, halo color for sun-aligned halo, emissive color additively (and SunColor for highlight). These five located cloud consumers do not establish ordinary inworld cloud rendering.

A splits on commas, requires requested columns, ignores blank rows, rejects missing/unparseable requested cells, nonfinite floats and out-of-range packed colors. Unrequested columns, row identity not used as a key, original textual spelling, high color byte and unknown bytes are not preserved. Normal rejects an empty keyframe map; Fog itself does not. Both sort by time, losing authored row order. The native `read_keyframes` requires both projections to parse successfully.

F:174 onward fixes zero end/height colors, clamps scaler/ranges, derives missing density and interpolates over a 2880-unit day. `FogResult` reverses polynomial coefficients to shader order and derives legacy/artistic selection and sun-angle blending. Those are helpers/derived values, not extra CSV fields. Modern far clip is the code constant 1000 yards. F's module header explicitly leaves map flag2 density override, skybox sun-direction override and map minimum fog-distance handling unported.

R `bind_fog` (:505 onward) publishes color, end/height/sun colors, ranges, densities, height/rate, z/legacy scalars, polynomial vectors and sun-scattering parameters to RenderingServer globals. `godot/shaders/retail_fog.gdshaderinc` is the corresponding shader contract. R `bind_water` (:125–150) binds actual water RGBA/specular and underwater fog uniforms. Underwater sampling N:200–222 uses **Normal** keyframes and S `retail_fog`, not the full FogKeyframe polynomial projection. This is a specific pipeline difference, not absence of modern surface fog.

## Modern LightParams / LightSkybox CSV

N:400–454 requests exactly seven LightParams columns. All seven are first parsed as `f32`; ID/Flags/LightSkyboxID then cast to `u32` (precision, fractions and special float handling therefore differ from strict A ID parsing). Four alphas are retained as floats without A's finite check. Duplicate keys overwrite; zero skybox is omitted.

| Literal column | Retention | Production use |
|---|---|---|
| `ID` | `f32 → u32` map key | Parameter lookup/blend. |
| `WaterShallowAlpha` | `river[0]: f32` | N water shallow RGBA → R water material. |
| `WaterDeepAlpha` | `river[1]: f32` | River deep RGBA. |
| `OceanShallowAlpha` | `ocean[0]: f32` | Ocean shallow RGBA. |
| `OceanDeepAlpha` | `ocean[1]: f32` | Ocean deep RGBA. |
| `Flags` | `f32 → u32` | Native body visibility/position and F fog flag policy. |
| `LightSkyboxID` | `f32 → u32`, nonzero join | N:378–391 requires matching skybox row. |

N:363–374 LightSkybox CSV requests **`ID: u32`**, **`SkyboxFileDataID: u32`**, **`Flags: u32`** through `csv_util::read_db2_rows`; it does not use the name column. It retains `(FDID, Flags)` keyed by ID, then rekeys by LightParams ID. `sky_bodies.rs:155–189` derives weighted FDID draws; same-FDID draws merge by maximum weight while previous draws fade. R's skybox update consumes those draws. Raw CSV row/string data is not retained.

### Named flag helpers versus ordinary and DEBUG consumers

`light_lookup_types.rs:6–30,50–74` wraps raw `u32`: `empty()` is zero, `from_bits`/`bits` preserve it, `contains(other)` means `(bits & other.bits) == other.bits`; bitwise OR combines masks. The following names are literal declarations, not inferred original-client semantics.

| Wrapper | Named mask | Located consumer |
|---|---|---|
| `LightParamsFlags` | `NO_DARKEN_DEPTH` 0x1 | Declaration only; native consumer unestablished in inspected Rust. |
| `LightParamsFlags` | `DONT_INHERIT_SKYBOX` 0x2 | DEBUG celestial suppression predicate. |
| `LightParamsFlags` | `HIDE_SUN` 0x4 | DEBUG suppression; ordinary sun-disc hide blend; F separately no-sun-fog. |
| `LightParamsFlags` | `HIDE_MOON` 0x8 | DEBUG suppression; ordinary both-moon hide blend. |
| `LightParamsFlags` | `HIDE_STARS` 0x10 | DEBUG suppression; ordinary star visibility. |
| `LightParamsFlags` | `OVERRIDE_CELESTIAL_SPHERE` 0x20 | DEBUG suppression. |
| `LightParamsFlags` | `HEIGHT_FOG_ABOVE_PLANE` 0x40 | DEBUG procedural fog predicate. |
| `LightParamsFlags` | `HIDE_CELESTIAL_OBJECT` 0x80 | DEBUG suppression. |
| `LightSkyboxFlags` | `FULL_DAY_SKYBOX` 0x1 | Ordinary time-of-day animation fraction. |
| `LightSkyboxFlags` | `COMBINE_PROCEDURAL_AND_SKYBOX` 0x2 | DEBUG procedural baseline predicate. |
| `LightSkyboxFlags` | `PROCEDURAL_FOG_COLOR_BLEND` 0x4 | DEBUG procedural fog predicate; ordinary final-fog cone. |
| `LightSkyboxFlags` | `FORCE_SUNSHAFTS` 0x8 | Declaration only; native consumer unestablished in inspected Rust. |
| `LightSkyboxFlags` | `DISABLE_USE_SUN_FOG_COLOR` 0x10 | Declaration only; native consumer unestablished in inspected Rust. |

DEBUG `skybox_debug.rs:75–107`: `suppresses_celestial_visibility` is ANY of Params0x2/0x4/0x8/0x10/0x20/0x80. `procedural_baseline_enabled` = Skybox0x2 (`None` → true) AND NOT that suppression (`None` Params → false). `procedural_fog_enabled` = Skybox0x4 (`None` → true) OR Params0x40 (`None` → false). `Composition::from_source` (:50–70) feeds `environment::create` (:208–211,257–260); environment :69–96 gates dome/reference plane and plane fog. Verify mode bypasses predicates: black clear, no baseline/fog/reference objects. Cached flags reach this path through `skybox_debug/source.rs:23–39,70–85` and `skybox_debug_data.rs:19–51`.

Ordinary `sky_bodies.rs:37–41,93–100` also names `LIGHT_PARAMS_SUN_POSITION = 0x100`: blended value >=0.5 suppresses **all** planet draws. This is distinct from F:9–11's unported fog sun-direction override0x200. Ordinary R:232–236,326–350 selects `LIGHT_SKYBOX_FINAL_FOG`0x4 if ANY collected draw carries it; R:269–275 uses Skybox0x1 to hold animation at `minutes.rem_euclid(2880)/2880`. These ordinary consumers do not prove the DEBUG composition policy is used inworld.

### Separate cached WDC5 path

E `godot/core/src/skybox_debug_data.rs:10–53` reads cached DB2 bytes, not the CSV catalog: LightSkybox FDID1308501, accepted hashes `9D4956FF/407FEBCF/D466A5C2`, row ID from section ID list, field1 Flags and field2 model FDID. LightParams FDID1334669, layout `CAE394E7`, field3 skybox ID, field10 flags. Values decode to `u32` and retain raw flags; a zero model FDID errors. Its helper `rendering/lighting/light_lookup_wdc5.rs` projects fixed records/field storage, including common/pallet decoding, not arbitrary raw table preservation. This narrow path is not the seven-column CSV reader and does not decode liquid alphas.

#### WDC5 reader metadata and literal helper fields

All offsets below describe **this helper's reads**, not new LightParams/LightSkybox schema columns. Source `godot/core/src/rendering/lighting/light_lookup_wdc5.rs` (W):4–19,48–67,200–314. LE means little endian. Magic at bytes0..4 must be `WDC5`; header read base H=136; section base Q=`section_offset + section_index*40`; storage base T=`fields_offset + index*24`.

| Owner / literal field | Read or derivation / type | Retention and use |
|---|---|---|
| `Wdc5Header.record_size` | H+8 LE u32 → usize | Record stride. |
| `Wdc5Header.total_field_count` | H+40 LE u32 → usize | Field count and storage/data offsets. |
| `Wdc5Header.section_offset` | Derived H+68, usize | Section-array start, not a wire read. |
| `Wdc5Header.layout_hash` | H+20 LE u32 → u32 | Validated against accepted layout(s). |
| `Wdc5Header.pallet_data_size` | H+56 LE u32 → usize | Derives common-data base. |
| `Wdc5Header.section_count` | H+64 LE u32 → usize | Section loop and field-storage location. |
| `Wdc5Section.file_offset` | Q+8 LE u32 → usize | Record-data base. |
| `Wdc5Section.record_count` | Q+12 LE u32 → usize | Row enumeration and ID-list derivation. |
| local `string_table_size` | Q+16 LE u32 → usize | Used only to derive ID-list offset. |
| local `_id_list_size` | Q+24 LE u32 → usize | Read, discarded; not retained/validated against row count. |
| `Wdc5Section.id_list_offset` | Derived `file_offset + record_count*record_size + string_table_size`, usize | Row IDs read LE u32 at this base + row_index*4. |
| `Wdc5FieldStorage.offset_bits` | T+0 LE u16 → u16 | Raw bit position; byte range and shift. |
| `Wdc5FieldStorage.size_bits` | T+2 LE u16 → u16 | Extraction width/mask; zero yields index0. |
| `Wdc5FieldStorage.additional_data_size` | T+4 LE u32 → u32 | Common/pallet cumulative per-field byte offsets. |
| `Wdc5FieldStorage.storage_type` | T+8 LE u32 → u32 | Storage decode dispatch. |
| `Wdc5RowRef.section_index` | Derived enumeration index, usize | Selects retained section. |
| `Wdc5RowRef.row_index` | Derived enumeration index, usize | Record/ID-list addressing. |
| `ParsedWdc5Db2.bytes` | Borrowed `&'a [u8]` | Internal original bytes for decoding; NOT returned by outer table readers. |
| `ParsedWdc5Db2.record_size` | Header copy, usize | Row byte stride. |
| `ParsedWdc5Db2.sections` | Parsed `Vec<Wdc5Section>` | Retained section metadata. |
| `ParsedWdc5Db2.pallet_offset` | `fields_offset + total_field_count*24`, usize | Pallet-data base. |
| `ParsedWdc5Db2.pallet_offsets` | Derived `Vec<usize>` | Prefix sums of additional sizes for storage3/4 only. |
| `ParsedWdc5Db2.common_offset` | `pallet_offset + pallet_data_size`, usize | Common-data base. |
| `ParsedWdc5Db2.common_offsets` | Derived `Vec<usize>` | Prefix sums of additional sizes for storage2 only. |
| `ParsedWdc5Db2.fields` | Parsed `Vec<Wdc5FieldStorage>` | Only four members per storage entry retained. |
| local `fields_offset` | `section_offset + section_count*40 + total_field_count*4`, usize | Skips section headers and four-byte-per-field area to storage entries. |

**Storage stride is 24 bytes, not 20** (W:296–314). Four reads consume bytes0..11 (12 bytes); bytes12..23 are **12 unread tail bytes**, not six additional bytes or authenticated additional schema fields. Section stride is40; unrequested header/section/storage bytes are not semantically inventoried here.

W:84–165 decodes a record value index from borrowed bytes using bit offset/width; storage0/1/5 returns index as u32, storage3/4 reads a pallet u32 at base + per-field offset + index*4, storage2 scans `(row ID,value)` u32 pairs over `additional_data_size`, returning zero if unmatched. Other storage types panic. Internal byte retention is temporary reader state; `read_light_skybox` / `read_light_params_skybox` outer results retain only their cooked u32 metadata. This inventory does not claim general WDC5 compatibility.

## Benilla: Vanilla DBC wire fields and cooked projection

B0:183–225 declares older schemas; :231–350 loads records. These are not modern CSV column aliases. `Light.dbc` has twelve 32-bit fields; the five params are not E's eight slots.

| DBC schema member | Wire type/index | Retention/loss |
|---|---|---|
| `ID` | u32 /0 | `Light.id`; fallback identity. |
| `continent` | u32 /1 | `Light.map`; map selection. |
| `x` | f32 /2 | Transformed into world position, not retained raw. |
| `y` | f32 /3 | Same. |
| `z` | f32 /4 | Same. |
| `falloffStart` | f32 /5 | Divided by36, sphere blend start. |
| `falloffEnd` | f32 /6 | Divided by36, sphere admission/end. |
| `param0` | u32 /7 | Clear slot. |
| `param1` | u32 /8 | Clear underwater. |
| `param2` | u32 /9 | Storm. |
| `param3` | u32 /10 | Storm underwater. |
| `param4` | u32 /11 | Death. |

Raw zero position is tested before transform. For nonglobal records, B0 `dbc_to_world` computes `[17066.666-z/36, 17066.666-x/36, y/36]`. Missing required base fields skip a record; missing param reads become zero. Raw inches/axis coordinates and source bytes do not survive this projection.

| Vanilla LightParams member | Wire type/index | Retention / consumer |
|---|---|---|
| `ID` | u32 /0 | Map keys. |
| `highlightSky` | u32 /1 | Normalized to 0/1 float; sky dawn/dusk warp. |
| `lightSkyboxID` | u32 /2 | Nonzero map entry; ghost sky model lookup. |
| `cloudTypeID` | u32 /3 | Schema decoded, not projected into catalog maps. |
| `glow` | f32 /4 | Raw glow map → Atmosphere → FFX composite gain. Not modern `SunFogStrength`. |
| `waterShallowAlpha` | f32 /5 | River shallow endpoint. |
| `waterDeepAlpha` | f32 /6 | River deep endpoint. |
| `oceanShallowAlpha` | f32 /7 | Ocean shallow endpoint. |
| `oceanDeepAlpha` | f32 /8 | Ocean deep endpoint. |

B0:323–339 LightSkybox schema is `ID: u32` and `Name: String` (DBC string-table reference resolved by `str_at`); stores normalized model chain path via `models::model_path`. It is not a modern FDID/Flags record. String offset/original spelling is lost in the cooked catalog.

### LightIntBand / LightFloatBand: actual cells versus named channel projections

BT:21–32 declares each 136-byte record: `ID u32` field0, `num u32` field1, `time0`…`time15` u32 fields2…17, `value0`…`value15` fields18…33 (`u32` for IntBand, `f32` for FloatBand). Each indexed cell has that type independently; these array indices are wire key slots, **not** the channel numbers below. BT:95–136 retains a map ID and parallel vectors, caps `num` to16, drops cells beyond num and original count, and uses `filter_map` for missing cells. IntBand values remain packed u32 until sampling; sampled color discards high byte, normalizes RGB and interpolates **sRGB**, unlike E linear-color interpolation. File-order cyclic bracketing is preserved, not time-sorted. Zero-key sampling returns None and B0 projects black/zero constants.

For parameter P, band channel b has row ID `(P-1)*18+b+1` (IntBand), `(P-1)*6+b+1` (FloatBand). All rows load into maps; only these individual channels project into `Atmosphere` (BA:4–27; B0:703–756):

| IntBand channel | Cooked projection | Production consumer |
|---|---|---|
| 0 | `sun_diffuse` | Global diffuse. |
| 1 | `ambient` | Global ambient. |
| 2 | `sky[0]` | Zenith sky stop. |
| 3 | `sky[1]` | Sky stop. |
| 4 | `sky[2]` | Sky stop. |
| 5 | `sky[3]` | Sky stop. |
| 6 | `sky[4]` | Horizonward sky stop. |
| 7 | `fog_color` | Scene fog. |
| 8 | No Atmosphere projection located | Retained band data; semantics not assigned here. |
| 9 | `sun_color` | Celestial tint and specular. |
| 10 | `cloud_colors[0]` | Cloud sun-glow tint. |
| 11 | `cloud_colors[1]` | Cloud gradient slope. |
| 12 | `cloud_colors[2]` | Cloud gradient base. |
| 13 | No Atmosphere projection located | Retained band data; semantics not assigned here. |
| 14 | `water_ocean[0]` | Ocean shallow tint. |
| 15 | `water_ocean[1]` | Ocean deep tint. |
| 16 | `water_river[0]` | River shallow tint. |
| 17 | `water_river[1]` | River deep tint. |

| FloatBand channel | Cooked projection | Production consumer / loss |
|---|---|---|
| 0 | `fog_end`, sampled value /36 | Scene fog distance; raw inch value not in Atmosphere. |
| 1 | `fog_start_frac` | Unclamped fraction, including negative storm values. |
| 2 | No Atmosphere projection located | Retained band, no semantics assigned here. |
| 3 | `cloud_density` | Cloud coverage threshold input. |
| 4 | No Atmosphere projection located | Retained band, no semantics assigned here. |
| 5 | No Atmosphere projection located | Same. |

B production bridge `crates/benilla-world/src/lighting/resolve.rs:124–156,253–309` copies Atmosphere to scene lighting: ambient/diffuse/spec, derived fog range, five sky stops, four water tints/alphas, quantized glow, highlight-dependent warp, celestial tint and cloud density/palette. Consumers include `lighting/global_light.rs`, `sky.rs`, `liquid/mod.rs`, `ffx_glow.rs` and `skybox.rs`; these are actual native paths, not format-library-only retention. Terminal cloud/celestial anchors: `crates/benilla-world/src/clouds/mod.rs:100–104` reads `light.cloud_density` and `light.cloud_colors[0..2]` into density / `CloudFrame.sun`, `.slope`, `.gbase`; `sun/follow.rs:281,369` reads and quantizes `light.celestial_tint` for sun/moon paths. These close FloatBand3 and IntBand9–12 beyond the bridge. Their exact shader math/full visual equivalence is outside this static field inventory.

## Coverage and bounded gaps

Covered: E's ordinary named Light reads; union of both live LightData parsers; exact seven LightParams/three LightSkybox CSV reads; separate WDC5 metadata subset; B's older Light/Params/Skybox schemas, every band channel and per-key wire layout; located native bridge/bindings.

Not certified: complete raw modern CSV header or DB2 schema, unrequested CSV columns/bytes, original-client semantics of `Field_10_0_0_44649_042` / `Field_12_0_0_63854_043`, ordinary inworld E cloud rendering, consumers for the three declaration-only masks identified above, WDC5 general-format completeness, pixel equivalence. Zone polygons/weather and BLP remain separate scopes. Parent owns index/log integration; only this page is authored by this task.

## Sources

- E `godot/core/src/lighting_assets.rs:12–186` — exact named CSV reads, numeric/color conversion.
- E `godot/core/src/retail_fog.rs:114–245` and subsequent sampling/uniform helpers — second live projection and derived fog.
- E `godot/core/src/rendering/lighting/light_lookup_data.rs` — actual Light record/slot and spatial blend implementation.
- E `godot/core/src/rendering/skybox/sky_lightdata_data.rs` — normal retained/interpolated channels and underwater retail fog.
- E `godot/rust/src/lighting/assets.rs:83–287,363–476`; `lighting/mod.rs:54–68,125–150,200–228,505–545` — native loading, sampling and bindings.
- E `godot/core/src/{skybox_debug_data,sky_bodies}.rs`; `rendering/lighting/{light_lookup_types,light_lookup_wdc5}.rs` — distinct cached wire reader, helpers and sky draws.
- B `crates/benilla-formats/src/light.rs:183–350,703–756`; `light/{tables,atmosphere}.rs` — Vanilla declarations, conversion and channel projection.
- B `crates/benilla-world/src/lighting/resolve.rs:124–156,253–309` — production Atmosphere consumption.

## See Also

- [[benilla-zone-weather-fields]] — separate zone polygon CSV joins and weather state/consumers.
- [[benilla-blp-fields]] — texture serialized fields and native upload boundaries.
- [[benilla-asset-field-inventory]] — parent inventory and environment integration.
- [[benilla-wmo-root-fields]] — shared pins/evidence vocabulary and root material comparison.
- [[benilla-wmo-root-aux-fields]] — WMO ambient/local-light/fog records, distinct from ordinary Light tables.
- [[open-source-wow-clients]] — version-specific comparison limits.
