# game-engine - Plan

## Active TODO

## Parked

- [ ] NPC / mob AI and pathfinding.

# Pending world height/coords calculation fixes
- [ ] Refactor app startup: each screen should own its full App configuration instead of main.rs building a monolithic app with all plugins always registered. Debug scenes (particledebug, debugcharacter) should not load game networking, login UI, etc.
- [ ] Cross-tile ADT stitching (border chunks differ by up to 50 units)
- [ ] Find the root cause of the white/grey terrain bands in the in-world ADT renderer

