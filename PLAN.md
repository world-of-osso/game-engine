# Goal

- [ ] Migrate login screen to Bevy UI in this isolated worktree; preserve appearance, interactions, authentication and other screens.

## Isolation

- [x] Create branch `bevy-ui-login` at engine `4a6f62cd` in `/home/osso/.worktrees/game-engine-bevy-ui-login`.
- [x] Confirm canonical checkout remains clean on `master`.
- [x] Reuse the existing shared Cargo target directory to avoid unnecessary dependency recompilation. Coordinate Cargo ownership with the canonical session; final binaries are shared, so record the exact built revision before execution. Source edits remain worktree-isolated.
- [x] Replace four shared sibling symlinks with real `*-bevy-ui-login` dependency worktrees at verified source revisions.
- [x] Update engine and toolkit Cargo manifests to resolve through dependency worktrees; commit `d7dae698` and `40097e9`.
- [x] Independently verify four real worktrees and Linux Cargo resolution; shared target/canonical manifests/lock unchanged. Dev check passes; global fmt retains104unchanged vendor differences. Evidence: dependency-worktrees/verification/report.md.
- [ ] Establish worktree-local asset access without writing canonical data or credentials/settings.

## Active caret addition

- [x] Implement user-requested blinking native insertion caret aligned to shaped text and existing password masking (`d0622b69` through `f70a6f87`).
- [ ] Verify editing/navigation/focus/modal/blink behavior; preserve accepted physical input and live auth.
- [ ] Independently verify caret and capture rendered result within authorized limits.

## Migration

- [x] Inventory login controls, input/focus/auth/automation and visual contracts; map native Bevy UI primitives and gaps.
- [ ] Record login-only migration spec and behavioral/visual baseline; do not claim visual equivalence from CPU tests.
- [x] Implement and commit native form, view, lifecycle/input/auth and semantic diagnostics; remove old login renderer.
- [x] Verify 57 distinct focused cases through `ae941bac`; see `data/diagnostics/login-bevy-ui/verification/focused-report.md`.
- [x] Execute setup-failure, startup-order and tint regressions; 62 distinct focused tests recorded.
- [x] Build exact renderer and capture settled standalone login at 12/30 seconds plus menu overlay; all diagnostic clients stopped.
- [ ] Inspect layout, physical input, modal layering and real authentication; fix only reproduced migration failures.
- [ ] Run final fmt/check after source finality; camera session currently owns shared target.
- [ ] Update tracked documentation and obtain independent focused verification and bounded visual proof.

## Limits

- [x] Prior native capture terminated after8seconds; no screenshot, camera collision reproduced.
- [ ] User authorized one NEW60-second isolated native-client window via Runtime proof question; capture login/menu/input and terminate within60seconds. Compile time separate.
- [ ] No implicit migration of other screens, canonical source changes, production deployment, or measured CPU-gain claims.
