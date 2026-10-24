# Zone-Light and Weather Input Fields

Individual spatial-light CSV inputs and the separate weather packet/state route. E baseline `354bdb7dbf177047ed46d63168acec531f5c4426`; B `8a6c74036cfe3d00275fd75ad1aeae1f4ea7d0df` (Vanilla1.12.1/build5875). Static local source/git inspection only: no execution, imports, builds, tests, extraction or runtime/visual compatibility proof. A read-only Git name diff from E baseline through `d2c42db72` was empty for `godot/core/src`, `godot/rust/src` and `godot/shaders`; documentation HEAD differs from the source baseline without changing those source trees.

**Z**=`godot/core/src/rendering/lighting/light_lookup_data.rs`; **N**=`godot/rust/src/lighting/assets.rs`; **W**=B `/home/osso/Repos/benilla/crates/benilla-world/src/weather/mod.rs`. A module alias is not a file path: Z is not `godot/core/src/light_lookup_data.rs`. General Light/LightData/LightParams inputs belong to [[benilla-lighting-fields]]. Zone CSV columns are named text inputs, not binary offsets or inferred complete exported schemas.

## ZoneLight and ZoneLightPoint: E serialized inputs

Z112–149 explicitly reads these ten named columns across two tables. `Name` is mentioned as nonnumeric but never requested by this parser; other unrequested columns are not invented. N99–108 loads the CSVs into the production catalog.

| Table / column | Parsed type and retention | Actual consumer / loss |
|---|---|---|
| ZoneLight `ID` | u32 → `ZoneLight.id` | Z133–135 joins point rows; not read by blending Z261–280 afterward |
| ZoneLight `MapID` | u32 → `map_id` | Z267–269 excludes other maps |
| ZoneLight `LightID` | u32 → `light_id` | Z224–233 resolves a Light row; requested LightParams slot contributes blend; Z271–275 uses descending ID as secondary order |
| ZoneLight `TransitionType` | i32 → **`priority`** | Z271–275 sorts ascending priority, then descending LightID; literal field is an integer, not a separately modeled transition type |
| ZoneLight `Zmin` | f32 → `z_min` | Z291–298,314–316 vertical containment/fade |
| ZoneLight `Zmax` | f32 → `z_max` | Same vertical consumer; distinct bound |
| ZoneLight `Name` | Not requested | No retained string/consumer established by this parser; bounded skip, not global absence |
| ZoneLightPoint `ZoneLightID` | u32 temporary map key | Z123–135 joins ZoneLight.ID; no separate point-record wrapper survives |
| ZoneLightPoint `PointOrder` | u32 temporary tuple member | Z125–143 sorts descending; omitted from final point list |
| ZoneLightPoint `Pos_0` | f32 → vertex `[0]` | Z304–351 polygon containment, segment/border distance |
| ZoneLightPoint `Pos_1` | f32 → vertex `[1]` | Same polygon geometry consumer |

`ZoneLight.points: Vec<[f32;2]>` is the ordered joined projection, not a new wire column. Z261–280 requires same map and more than one point. Z285–299 combines signed polygon and height distances with a50-unit band; Z277–280 computes `clamp(-distance/100,0,1)`. Calculated polygon bounds, crossing count, segment projections and weights are helpers; Z304–351 derives them from listed coordinates. Z209–245 overlays map default, zones, then local lights and removes zero LightParams IDs. N146–174,203–248 uses clear/underwater slots and samples lighting, fog and water. Source behavior is not an executed blend/parity test.

No equivalent polygon/ZoneLightPoint consumer was established in the inspected B weather/lighting route. Its older band/weather system below is not a negative inventory of every B lighting file or proof of a cross-version missing table.

## B weather packet: four distinct fields

B `crates/benilla-protocol/src/messages/parse.rs:581–586` reads SMSG_WEATHER. `messages/packet.rs:387–393`, `events.rs:481–487`, `events/decode.rs:51–61` preserve values; `crates/benilla-app/src/net/world.rs:14–26,40–54` sends `WeatherMessage`, W296–303.

| Packet offset / field | Wire → message type | Production interpretation and consumers |
|---|---|---|
| `+0 weather_type` | u32 → u32 | W23–40 maps1 Rain,2 Snow,3 Sand, other Fine; W131–155 changes kind/effect/cut state. Precipitation pool and mist select kind (`weather/precip/pool.rs:237–263`, `weather/precip/mist.rs:130–141`) |
| `+4 grade` | f32 → f32 | W131–148 clamps0..1, Fine targets0; effect channel and sky channel target separate densities. W158–163 publishes emission density and sky density; actual precipitation and lighting consumers below |
| `+8 sound_id` | u32 → u32 | Independent app `sound/weather.rs:18–27` stores `WeatherAmbience`; `sound/zone.rs:341–357` first honors forced ghost/submerged ambience, then selects nonzero weather kit outdoors, otherwise normal/interior ambience. Not used by visual state machine |
| `+12 instant` | u8 → bool (`!=0`) | W149–155 chooses snap versus retarget/ramp for both visual channels |

The fields have concrete B wire provenance, not CSV columns to be guessed for E. B client-side `weatherDensity` is a separate local setting, not packet input.

## B state/helper representations, not additional packet fields

W44–83's private `Channel` has four individual helper members, not packet fields or a binary record layout:

| Member | Type / derivation and mutation | Actual consumer |
|---|---|---|
| `from` | f32, initialized0; retarget copies previous `to` (not current interpolated value), snap assigns target, W47,57,74,81 | W66–68 duration denominator/interpolation origin |
| `to` | f32, initialized0; retarget/snap assigns target, W48,58,75,82 | W66–68 interpolation endpoint; W74 next retarget origin; completion instrumentation W275–276 |
| `start` | f64, initialized0; retarget sets real-time `now`; snap leaves it unchanged, W50,59,76 | W67 elapsed-time numerator; W228 supplies `Time<Real>` elapsed seconds |
| `span_scale` | f32 constructor argument, W51,60; channel instances use1/4, W120–121 | W66 duration `abs((to-from)*span_scale+0.001)*10`; W67 clamps elapsed fraction0..1 |

W86–110's `WeatherState` members below hold selected/derived values. Its two Channel instances are not eight additional wire inputs.

| Member | Type/role | Actual consumer / limit |
|---|---|---|
| `kind` | crate-visible (`pub(crate)`) `WeatherKind`, latest selected type | W141–145 transition comparison; distinct active effect kind below |
| `effect_kind` | public `WeatherKind` | Precipitation kind/pool/mist selection as above |
| `intensity` | private `Channel`, A | W158–163 resolves intensity and derives effect density |
| `sky` | private `Channel`, B | W146–163 caps target at0.25 and resolves sky density |
| `intensity_a` | f32 published within crate | W158–163 intermediate + module instrumentation; no additional direct consumer established |
| `effect_density` | f32 | `weather/precip/pool.rs:247–263` and `weather/precip/mist.rs:141` scale emission; `weather/precip/mod.rs:317–332` is diagnostics, not spawn-rate math; formula `max((A-0.25)*4/3,0)` |
| `sky_density` | f32 | `lighting/resolve.rs:229–247` blends clear/storm LightParams via storm blend; resulting atmosphere feeds scene fog251–253 and constructed lighting277–319, committed320–323 |
| `weather_density` | u8, default3 | W102–105,166–169 clamps gain index to3; pool/mist emission uses gain. App `video.rs:290` updates from CVar registered `cvars/table.rs:679–689` |
| `cut_seq` | u32 wrapping counter | W141–143 changes on kind switch; precipitation notices type-change cut at `weather/precip/mod.rs:281–282` |

B `weather/precip/mod.rs:113–114` rain effect-fog constants70/75 also reach particle rendering `particles/render.rs:137–138`; this effect override is distinct from the weather-driven lighting atmosphere/fog blend. App sound documentation describes indoor ambience replacing weather with a5-second crossfade; grade logging is not grade-scaled sound intensity.

## E selected native weather boundary

Static searches of Rust under `godot/`, then core `rendering/lighting`, native `lighting`, native Rust and `godot/network/src` for weather names and packet/state terms did not establish an explicit native `WeatherMessage`, SMSG_WEATHER, `weather_type`, `weather_density` or weather-driven lighting input. Matches included unrelated pet-battle weather UI and terrain sound IDs. N99–110 loads Light/LightData/ZoneLight/ZoneLightPoint/LightParams, **not** a weather table; N146 chooses Clear and203–223 separately chooses ClearUnderwater.

This supports a **bounded unknown/counterpart not found** for the four B packet fields and local/derived weather state in the named E native path. It does not prove absence across GDScript, external services, generated/runtime bindings or every subsystem. Ordinary E environment lighting exists; no missing CSV schema or implementation work is inferred. Zone polygon weights are not themselves weather state.

## Sources

- Z99–149,203–351; N99–110,130–174,203–248 — named CSV readers, joins, spatial/slot consumers and native samples.
- B protocol `messages/parse.rs:581–586`, `messages/packet.rs:387–393`, `events.rs:481–487`, `events/decode.rs:51–61`; app `net/world.rs:14–26,40–54` — wire/dispatch provenance.
- W23–169,183,296–303; B world `lighting/resolve.rs:171–185,229–253,277–311`, `weather/precip/{mod,pool,mist}.rs` anchors above; `particles/render.rs:137–138` — selected weather and actual visual/fog consumers.
- B app `sound/weather.rs:1–27`, `sound/zone.rs:341–357`, `video.rs:290`, `cvars/table.rs:679–689` — independent sound route and local density setting.

## See Also

- [[benilla-lighting-fields]] — ordinary light/band/skybox and fog-projection inputs.
- [[benilla-blp-fields]] — texture wire inputs, not environment state.
- [[benilla-wmo-root-aux-fields]] — separate WMO local fog/light fields.
- [[benilla-asset-field-inventory]] — full comparison scope and static evidence limits.
