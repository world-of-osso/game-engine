# Weather and camera-underwater environment

[Scope and pins](README.md). These are missing environment inputs/consumers, not a claim that ordinary authored sky, liquid rendering or day/night lighting is absent.

## ENV1: Weather state has no precipitation or storm-light consumer

**Reference.** `W/WoW/Source/WorldClient/MapWeather.cpp:719–783` accepts weather type/intensity/transition inputs; `:584–704` updates state, chooses precipitation implementations and interpolates intensity. `:564–580` renders active rain/snow/sand. `W/WoW/Source/WorldClient/World.cpp:1570,4885–4887` reaches weather update/render. `W/WoW/Source/WorldClient/Rain.cpp:39–69` drives precipitation/mist rate from intensity and derives wind orientation from motion. `W/WoW/Source/WorldClient/DayNight.cpp:2803–2830` uses weather intensity in storm lighting and sky updates.

**Current.** Native environmental updates at `E/godot/rust/src/lib.rs:2643–2670` pass map, player position, world time and WMO fog into `WorldLighting::sync`. That consumer at `E/godot/rust/src/lighting/mod.rs:190–244` has no weather type/intensity/transition input. The ordinary Retail catalog explicitly samples `LightParamsSlot::Clear` (`E/godot/rust/src/lighting/assets.rs:146–152`); it does not blend clear/storm slots. Native world/render source has no precipitation lifecycle corresponding to the reference state and rain/snow renderers. M2 effect particles and animated water are not a weather-state consumer.

**Required change.** Produce and consume weather state for the intended game contract: precipitation type, density/intensity, transition timing, motion/wind response and relevant storm-light/sky changes. Keep the already implemented clear-sky/day-night path. The client reference does not by itself specify the project's authoritative server scheduling or a new wire schema; those inputs require an explicit project contract, not an inferred packet migration.

**Acceptance.** Feed reference-compatible rain and snow inputs at intensity 0.5 with a fixed player velocity and known transition time; verify type, nonzero density, motion response and interpolated storm lighting. Then feed clear weather and observe the prescribed fade-out. Test stationary and moving cameras, plus an ordinary clear-time-of-day control. These are source-derived fixtures, not proof of specific map weather, all precipitation families or current server policy.

## ENV2: Camera immersion does not select scene-wide fog or depth darkening

**Reference.** `W/WoW/Source/WorldClient/World.cpp:750–756` provides camera liquid identity/depth. `W/WoW/Source/WorldClient/DayNight.cpp:2143–2189,2206–2236` chooses and blends underwater versus interior fog with liquid/fog flags. `:2603–2608,2697–2711` uses camera liquid depth and the liquid's maximum darken depth/intensities to darken scene fog, ambient light and direct light.

**Current.** `E/godot/rust/src/lib.rs:2643–2670` queries camera WMO fog but no camera liquid state. `E/godot/rust/src/lighting/mod.rs:190–244` combines the catalog's ordinary fog with WMO fog. `E/godot/rust/src/lighting/assets.rs:161–173` samples underwater fog into **water material** inputs. `E/godot/shaders/water.gdshader:183–206` fogs a refracted scene sample according to surface thickness. That is a real water-surface effect, not immersion selecting scene-wide fog/light for terrain, models and the sky when viewed from below the liquid.

**Required change.** Derive camera liquid identity/depth and apply target-appropriate underwater scene-fog selection and clamped fog/ambient/direct darkening, respecting liquid and WMO fog flags. Preserve the separate surface refraction/thickness behavior. Do not key this solely off player swimming: a submerged camera and the player's feet are different observations.

**Acceptance.** Keep map, ordinary light, WMO fog and player position fixed; move only the camera through a liquid with known darken parameters. Sample at the surface, halfway to the authored maximum depth and beyond that maximum. Scene fog/light must follow the reference selection and depth clamp while terrain and models remain visible without any water-surface fragment in front of them. Test liquid flags that restore group fog and a no-liquid control. The reference uses signed submerged depth; convert axes/sign explicitly rather than assuming positive depth. Asset relevance, native pixels, liquid-depth glare and underwater audio are not established by this entry.
