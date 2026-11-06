# Healer ground rings

Client-only health rings drawn on the ground under party members, so a healer can read the
party's health without looking at the party frames. No server message or rule changes: the
rings read the group state the client already receives.

## Scope

- Party members only. The local player's own ring is not drawn: Retail party frames exclude
  the player, and a ring under the camera's own character adds nothing at the feet.
- In world only. The rings do not draw on any other screen.
- Raid members follow the raid rule below.

## Health arc

- Arc length is the primary cue. At 100% health the ring is a full circle. As health drops,
  a gap grows clockwise from 12 o'clock, so the visible arc always ends at 12 o'clock and
  its start advances clockwise.
- The ring is hidden at or above 92% health. It fades in over the next 2%, fully opaque at
  90%.
- A colour ramp is the secondary cue. The default ramp is colour-blind safe (blue at full
  health through amber to orange at empty). A palette option selects the classic
  red/yellow/green health ramp instead.

## Dispel ring

- A separate, thinner ring outside the health arc pulses whenever the member carries a
  harmful aura whose dispel school the player's current specialization can remove.
- Only the four player-dispellable schools are shown: magic, curse, disease, poison. The
  strongest present school in that order sets the colour; the colour comes from the existing
  debuff-type colours (the graphics colour-blind setting still applies).
- Schools are derived from DB2, never from spell id lists: the active specialization's
  healer role comes from `ChrSpecialization.Role`; the schools come from
  `SpellEffect.Effect` 38 rows (`EffectMiscValue_0`) for the spells the server says the
  player knows.
- The dispel ring does not wait for the health arc's fade threshold: a dispellable debuff is
  actionable at full health.

## Off-screen chevron

- A member below the health threshold whose feet are outside the viewport, or behind the
  camera, gets a chevron on the nearest viewport edge, rotated toward the member. A member
  at or above the threshold gets none.

## Click to target

- Clicking the drawn part of a ring targets that member. The empty gap of the health arc and
  the hole inside it are not clickable. The pulsing dispel ring is clickable around its whole
  circle.

## Ground contact

- The ring lies on the highest visible render surface under it, using the same raycast as
  foot IK and ground spell-effect draping, so it follows stairs and carpets. It re-drapes
  when the member moves and once shortly after it appears; a still ring samples no ground.

## Options (HUD page, both skins)

Persisted under `hud.healerGroundRings`:

| Option | Default | Behaviour |
| --- | --- | --- |
| Enabled | on | Master switch for the feature. |
| Ground Rings for All Roles | off | Opt-in: non-healer specializations also get rings in a party. |
| Colour-blind Health Ramp | on | Secondary cue palette (off selects the classic ramp). |
| Raid Ground Rings (Lowest N) | 0 | Raids show no rings at 0; otherwise only the N lowest-health members. |

Scope rules: with Enabled on, a healer specialization in a party shows every member; any
other role shows none unless opted in. A raid never uses the party rule; it shows nothing
unless Lowest N is above zero, and then only that many lowest-health members. Members at or
above the health threshold still draw no health arc.

## Acceptance

- Unit tests: arc direction and length, the 92%/90% fade band, palette ramps, scope defaults,
  lowest-N selection, dispel filtering from real DB2 rows, chevron edge placement and the
  ring ray hit test (annulus plus drawn arc).
- Forever captures from the offline preview: mixed health, a removable magic debuff beside a
  non-removable poison, the off-screen chevron, and a ring draped across staircase treads.
  Each capture is diffed against an identical no-ring frame, and single pixels inside and
  outside the arc are compared.
- The feature's own per-frame cost is reported from the client and compared with the same
  frames with the rings suspended.
