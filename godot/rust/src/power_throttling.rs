//! Windows power throttling (EcoQoS). On a hybrid CPU Windows may run a process it does
//! not consider foreground at low quality of service: efficiency cores at reduced clocks.
//! Two Windows launches at one Stormwind pose (i7-13700, Balanced plan; frame traces in
//! fps-diagnostic-frametrace-2026-11-06) differed 1.6x in main, render and GPU time.
//! The process opts out once at startup; Godot never sets a thread's throttling state, so
//! every thread inherits the process state.

/// Asks Windows never to throttle this process's execution speed. A failure is logged and
/// the client continues at whatever quality of service Windows chooses.
#[cfg(windows)]
pub(crate) fn opt_out() {
    if let Err(error) = windows::disable_execution_speed_throttling() {
        godot::global::godot_error!("Windows power throttling opt-out failed: {error}");
    }
}

#[cfg(not(windows))]
pub(crate) fn opt_out() {}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;

    type Handle = *mut c_void;
    /// `PROCESS_INFORMATION_CLASS::ProcessPowerThrottling`.
    const PROCESS_POWER_THROTTLING: i32 = 4;
    const PROCESS_POWER_THROTTLING_CURRENT_VERSION: u32 = 1;
    const PROCESS_POWER_THROTTLING_EXECUTION_SPEED: u32 = 0x1;

    /// `PROCESS_POWER_THROTTLING_STATE`.
    #[repr(C)]
    struct PowerThrottlingState {
        version: u32,
        control_mask: u32,
        state_mask: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetCurrentProcess() -> Handle;
        fn GetLastError() -> u32;
        fn SetProcessInformation(
            process: Handle,
            class: i32,
            information: *const c_void,
            size: u32,
        ) -> i32;
    }

    /// The execution-speed bit is controlled by the process and cleared: high quality of
    /// service whatever the window state.
    pub(super) fn disable_execution_speed_throttling() -> Result<(), String> {
        let state = PowerThrottlingState {
            version: PROCESS_POWER_THROTTLING_CURRENT_VERSION,
            control_mask: PROCESS_POWER_THROTTLING_EXECUTION_SPEED,
            state_mask: 0,
        };
        // SAFETY: the pseudo handle names this process and needs no close; `state` is a
        // live PROCESS_POWER_THROTTLING_STATE of the size passed.
        let set = unsafe {
            SetProcessInformation(
                GetCurrentProcess(),
                PROCESS_POWER_THROTTLING,
                (&raw const state).cast(),
                size_of::<PowerThrottlingState>() as u32,
            )
        };
        if set == 0 {
            // SAFETY: reads the calling thread's last error, set by the failed call above.
            return Err(format!("SetProcessInformation error {}", unsafe {
                GetLastError()
            }));
        }
        Ok(())
    }
}
