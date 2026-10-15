# World audio selection, scheduling and environment

[Scope and pins](README.md). Native zone music/ambience, footsteps, UI and spell audio already exist. These entries identify omitted contextual inputs and behaviors, not a missing audio engine or a claim that all sound is synthetic.

## AU1: Parsed terrain MCSE emitters never reach native playback

**Reference.** `W/WoW/Source/WorldClient/MapChunk.cpp:206–207` retains MCSE records; chunk creation invokes `CreateSoundEmitters` at `:1032`. That function at `:932–952` creates one sound-kit object per record, requests looping playback at its authored position and supplies emitter size as cone orientation. Chunk cleanup stops and removes the handles at `:146–149`.

**Current.** `E/godot/core/src/adt.rs:40,113` preserves `sound_emitters`; source record fields are at `E/godot/core/src/asset/adt_format/adt.rs:189–195`. Native terrain loading retains the parsed root (`E/godot/rust/src/terrain/assets.rs:114–133,209–242`; `terrain/streaming.rs:433–473`), but no native chunk sound-emitter consumer creates playback from those records. Native sound initialization at `E/godot/rust/src/sound_client.rs:17–31,41–73` loads zone/footstep catalogs, and its world update at `:193–197` syncs the root zone only. Retained parsed data is not discarded parsing or proof of playback.

**Required change.** Play applicable chunk emitters as authored positional looping sounds with their directional properties, and stop/release them with the owning chunk/map lifecycle. Keep spatial emitter playback distinct from the non-positional zone ambience channel. Validate modern record/kit resolution rather than treating a legacy numeric entry as a FileDataID.

**Acceptance.** A loaded native chunk with one MCSE record selects a known sound-kit entry, position and direction and produces audible looping output from that location. Moving the listener changes its spatial response; removing/resetting the chunk stops it and releases its player. An identical chunk without the record stays silent. No qualifying real chunk or audible run was sampled in this audit.

## AU2: Subarea and WMO context cannot select world music/ambience

**Reference.** `W/WoW/Source/WorldClient/AreaList.cpp:162–167` queries WMO root/group and inside state. `:80–131` selects authored intro/music/ambience from WMO group/root and area/zone records according to context, then supplies sound-system slots. `W/WoW/Source/SoundInterface/SoundInterface2ZoneSounds.cpp:324–390` resolves those choices into active ambience/music selection.

**Current.** `E/godot/rust/src/sound_client.rs:133–155` derives terrain area plus its root zone, but `:193–197` sends only that root zone to `NativeSound::sync`. `E/godot/rust/src/sound.rs:411–430` selects the two channels from that single zone and user options. The traced consumer receives neither subarea/WMO group identity nor inside state, so two different interiors in the same terrain zone cannot select their authored overrides through this path.

**Required change.** Select world music/ambience/intros using the applicable area and WMO context with source-defined precedence, preserving ordinary zone behavior. This entry concerns content selection; reverb application is AU4, not another name for ambience.

**Acceptance.** Hold terrain root zone and time fixed. Traverse exterior → WMO group A → group B with distinct authored music/ambience overrides, and a subarea with its own selection. Observe the selected kit identities and audible transitions; a context without an override follows the applicable parent rule. Do not assume every inside/outside branch uses the same precedence. Target-build records and audible impact remain unmeasured.

## AU3: Zone music is immediate cursor cycling, not the authored scheduler

**Reference.** `W/WoW/Source/SoundInterface/SoundInterface2ZoneSounds.cpp:216–297` selects music using priority slots, day/night variants, silence delay and intro eligibility. `:301–319` schedules randomized silence; `:419–429` limits intro replay. `:576–674` preserves already-playing kit identity, arbitrates competing script/camera/sequence music, sets transitions/fades and uses completion callbacks.

**Current.** `E/godot/rust/src/sound.rs:49–95` returns while the channel is playing; when it ends, it immediately selects the next per-zone cursor entry. A zone change stops the current player before starting the next. Its inputs have no world time, intro cooldown, silence bounds or priority-source state. `E/godot/core/src/sound/catalog_data.rs:9–49` supplies area-to-track mappings rather than that scheduler contract. Existing cursor cycling is real playback, but not equivalent scheduling.

**Required change.** Preserve authored scheduling metadata and source precedence: applicable day/night choice, intro eligibility/cooldown, silence timing and non-restarting/faded transitions. Keep unrelated UI/effect sounds outside music arbitration. Determine current product contracts for script/camera music before introducing new sources, rather than assuming the entire Wrath slot table must be copied.

**Acceptance.** Use two zone kits with distinct day/night entries and fixed silence bounds, plus one competing higher-priority source. Advance a controllable clock across track completion, the silence deadline, a day/night boundary, a zone change and repeated intro eligibility. Verify selected identity, silence duration, no redundant restart and prescribed transition envelope. Random silence uses a controlled seed or equal bounds. Native audio timing has not been measured.

## AU4: Contextual interior/underwater reverb is not applied

**Reference.** `W/WoW/Source/WorldClient/AreaList.cpp:80–131` selects ordinary and underwater reverb preference records and supplies inside state. `W/WoW/Source/SoundInterface/SoundInterface2ZoneSounds.cpp:68–82,96–98,109–116,133–143` maintains that context and reapplies reverb when it changes. `W/WoW/Source/SoundInterface/SoundInterface2Reverb.cpp:13–71` converts preference records into audio-engine reverb properties.

**Current.** The native context boundary remains `E/godot/rust/src/sound_client.rs:133–155,193–197` → `E/godot/rust/src/sound.rs:411–430`: root zone and volume/options only. `NativeSound`'s channel creation/playback at `E/godot/rust/src/sound.rs:23–95` does not receive or apply reverb preferences, inside status or camera underwater state. Existing WMO visual lighting and a general-purpose backend's reverb capability do not supply those missing audio inputs.

**Required change.** Resolve and apply the intended area's/WMO's dry/interior/underwater reverb environment as context changes, with target-appropriate parameters and channel routing. Do not substitute global permanent reverb or confuse this with geometric sound occlusion, which was not established by the comparison.

**Acceptance.** Play the same fixed impulse/sample through the affected world-audio route while switching between two known preference records and an underwater override, holding track and gain fixed. Check selected environment and recorded decay/early-reflection output; leaving restores the prescribed dry/exterior environment. Qualify parameter units and modern routing before a parity claim. No native acoustic recording or target-asset census was performed.
