# Native GDScript completion

## Contract

- Native fixture acceptance requires successful process exit, exactly one anchored `ALL_CASES_DONE done/expected` line with both numbers equal to the invocation's independently specified expected count, and no `SCRIPT ERROR`, `ERROR:`, or `FAIL ` diagnostic on either complete output stream. No startup/import error exceptions are allowed by this contract. Import runs are separate from acceptance runs.
- A case counts only after its behavior checks and awaited steps return explicit success. A failed check, early return, incomplete count, or timeout must fail the run; assertions inside awaited coroutines are not a completion mechanism.
- Existing geometry, input, pixel, state, and authority checks remain unchanged. Count success does not expand a fixture's authority claim.
- Standalone fixtures print the final completion marker once and quit zero only after successful completion. Menu/sound lifecycle fixtures print it before invoking the actual production Exit action; the runner still requires that action to terminate the process successfully.

## Scope

The migration covers campsite selected hover, Communities fidelity, dropdown caption centering, guild control, guild member notes, guild tabard, House Access, HUD edit controls, loot hit area, minimap zoom, PvP ready placement, ReadyCheck difficulty/portrait/title bounds, and registration. Random Bonus is included after its master landing, preserving32 physical-action receipts and4 completed cases. The shared native-input menu/sound runners also enforce this contract; other legacy fixtures are not silently reclassified.

## Proof

`game-engine-network` tests `fixture_completion::tests` exercise a real nested awaited assertion that continues and exits zero, checked nested failure with nonzero exit, successful completion, duplicate/missing/short markers, nonzero exit and errors on either stream. `gdscript_completion_fixture ROOT NAME OUTPUT --headless|--window` validates complete streams and fixed expected counts; `--check` is parse-only and never behavioral acceptance. Build/test through `scripts/engine-build.py` under `agent-run gdcontract`. Rendered screenshot/pixel fixtures require a real window, and registration/member-note fixtures require private server scheduling.
