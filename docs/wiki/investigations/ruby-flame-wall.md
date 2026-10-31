# Ruby Life Pools flame wall

GO377194 is the flame wall behind Kokia Blazehoof. In the older client it remained visible after the encounter because the open state did not play the authored door animation.

## Evidence and fix

Server `09097eb682b17d3336b8f4a2b1a5a0a07e90f03d` and engine `22f165203..36f107199` corrected the door state and animation. On engine `ecf59d4fb`, the Forever client showed the wall gone after Kokia from a matched 40-yard view, consistent with the retail 19:00 reference. GO state changed 1→0, collision true→false, and the model played Open 148 then Opened 149. Live particle quads fell from 101–120 to zero about 1.83 seconds after Open and stayed at zero for 487 sampled Opened frames.

Mirrored 12-yard views showed full-height flames on both sides. A fixed-line-of-sight 12–60-yard sweep was consistent with perspective; it did not establish a particle-size or LOD defect. Kokia's separate upright-corpse cause and the inherited-animation coverage limits are recorded in [animation](../systems/animation.md#skpd-parent-animation-banks--2026-10-31).

## Sources

- Captures: `/syncthing/AgentShared/2026-10-31/ruby-flame-wall/`.
- Trace: `data/diagnostics/ruby-flame-wall-2026-10-31/wall-open-live.jsonl`.
- [M2 animation and attachment specification](../../specs/m2-animation-inheritance-attachments.md).
