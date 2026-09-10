# Login camera startup order

Initial `OnEnter(GameState::Login)` runs before the toolkit's `Startup` camera creation. The native login spawned camera order 1, then the toolkit added another order-1 camera, producing a Bevy ambiguity and preventing reliable login capture.

## Root cause and correction

Bevy state transitions run before `PreStartup`; the toolkit creates `UiCamera` in `Startup`. `native::setup` therefore had no toolkit camera to raise during an initial login launch. Engine `675f1a7d` adds `raise_startup_ui_cameras` in `PostStartup`: while `LoginSession` exists, it records each not-yet-recorded toolkit camera order and raises it to 2. Native login remains at order 1; cleanup restores recorded orders.

The accompanying startup-order/teardown regression is unexecuted. The correction does not prove rendered ordering or menu compositing.

## Runtime evidence

At `65fd860b`, an isolated 8-second login process started the exact recorded renderer and exposed the order-1 ambiguity. IPC screenshot was requested against the exact launched PID socket, but no WebP was written before timeout. `launch.json` records the argv, environment, executable hashes, PID and elapsed time. This is failure evidence, not visual proof.

## Sources

- [native login lifecycle](../../../src/scenes/login/native.rs) — `OnEnter`, `PostStartup`, saved camera orders and cleanup
- [toolkit camera setup](../../../../ui-toolkit/src/render.rs) — toolkit order-1 camera
- [runtime launch record](../../../data/diagnostics/login-bevy-ui/runtime/launch.json) — isolated failed capture
- [final compile slice](../../../data/diagnostics/login-bevy-ui/verification/final-slice/report.md) — revision-scoped check and test evidence

## See Also

- [[ui-system]] — login rendering and diagnostics
