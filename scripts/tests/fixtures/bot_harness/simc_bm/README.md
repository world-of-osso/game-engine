# BM projection fixtures

`hunter_beast_mastery.simc` is unmodified SimulationCraft source at the commit/raw URL/SHA-256 in `source.json`; upstream license is GPL-3.0. `class-source.json` pins the associated hunter implementation: `howl_summon.ready` checks native buff471878/472324/472325, not a fabricated cooldown condition. Howl cooldown is471877; Cobra Fang1299389 is a tier-set buff. Nature's Ally rank3 is a selected specialization talent, absent from this character's received entries.

`captured-native.json` is character47's first retained `native-bm-tame1/253/run-001/native-samples.jsonl` observation, with actor ID joined from its account observation. The CSVs retain only its actual known spells and authoritative selected trait rows from the active native DB2 export. They test real kit/trait resolution, not maximum-level talent behavior. Tests add explicit synthetic charge clocks for predicate arithmetic; those numbers do not claim Retail recharge values.

No source predicates are relaxed for level32: absent Howl cooldown and KC max1 make the final Kill Command conjunction false. Auto Shot requests use native75; request acceptance alone does not prove native repeating ranged shots or complete SimC coverage. Native live runs must establish that separately.
