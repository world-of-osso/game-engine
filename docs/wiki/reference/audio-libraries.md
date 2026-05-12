# Audio Libraries & Tools

## AudioNimbus

**Repository:** https://github.com/MaxenceMaire/audionimbus

Rust wrapper around Valve's Steam Audio engine for spatial audio in games and VR.

**Capabilities:**
- Sound propagation modeling (distance attenuation, obstacle interactions)
- Reflections and reverb via geometric simulation
- HRTF processing (head-related transfer functions for directional/distance cues)
- Ambisonics support (multi-channel spatial audio)

**Integration:** Works with FMOD Studio, Wwise, and Bevy game engine.

**Status:** Interesting for future audio implementation — provides physics-based spatial audio without building from scratch.
