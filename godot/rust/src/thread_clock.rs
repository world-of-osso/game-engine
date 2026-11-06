//! Thread CPU clocks for benchmarks: a frame splits into each thread's work and its
//! waiting. Windows charges thread time per scheduler tick (about 15.6 ms), so read
//! these over many frames rather than per frame.

/// The calling thread's CPU time in milliseconds.
#[cfg(target_os = "linux")]
pub(crate) fn thread_cpu_ms() -> Option<f64> {
    crate::profile::thread_cpu_time().map(|time| time.as_secs_f64() * 1000.0)
}

/// CPU time in milliseconds of the single thread of this process named `name`; `None`
/// when no thread or several threads have that name.
#[cfg(target_os = "linux")]
pub(crate) fn named_thread_cpu_ms(name: &str) -> Option<f64> {
    let tasks = std::fs::read_dir("/proc/self/task").ok()?;
    let mut matches = tasks.filter_map(Result::ok).filter(|task| {
        std::fs::read_to_string(task.path().join("comm")).is_ok_and(|comm| comm.trim_end() == name)
    });
    let task = matches.next()?;
    if matches.next().is_some() {
        return None;
    }
    let stat = std::fs::read_to_string(task.path().join("schedstat")).ok()?;
    let nanoseconds: u64 = stat.split_whitespace().next()?.parse().ok()?;
    Some(nanoseconds as f64 / 1e6)
}

#[cfg(windows)]
pub(crate) fn thread_cpu_ms() -> Option<f64> {
    // SAFETY: the pseudo handle names the calling thread and needs no close.
    windows::thread_times_ms(unsafe { windows::GetCurrentThread() })
}

#[cfg(windows)]
pub(crate) fn named_thread_cpu_ms(name: &str) -> Option<f64> {
    windows::named_thread_cpu_ms(name)
}

#[cfg(not(any(target_os = "linux", windows)))]
pub(crate) fn thread_cpu_ms() -> Option<f64> {
    None
}

#[cfg(not(any(target_os = "linux", windows)))]
pub(crate) fn named_thread_cpu_ms(_name: &str) -> Option<f64> {
    None
}

#[cfg(windows)]
mod windows {
    use std::ffi::c_void;

    type Handle = *mut c_void;
    const INVALID_HANDLE_VALUE: Handle = -1isize as Handle;
    const TH32CS_SNAPTHREAD: u32 = 0x4;
    const THREAD_QUERY_LIMITED_INFORMATION: u32 = 0x0800;

    #[repr(C)]
    #[derive(Default)]
    struct FileTime {
        low: u32,
        high: u32,
    }

    #[repr(C)]
    #[derive(Default)]
    struct ThreadEntry32 {
        size: u32,
        usage: u32,
        thread_id: u32,
        owner_process_id: u32,
        base_priority: i32,
        delta_priority: i32,
        flags: u32,
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub(super) fn GetCurrentThread() -> Handle;
        fn GetCurrentProcessId() -> u32;
        fn GetThreadTimes(
            thread: Handle,
            creation: *mut FileTime,
            exit: *mut FileTime,
            kernel: *mut FileTime,
            user: *mut FileTime,
        ) -> i32;
        fn CreateToolhelp32Snapshot(flags: u32, process_id: u32) -> Handle;
        fn Thread32First(snapshot: Handle, entry: *mut ThreadEntry32) -> i32;
        fn Thread32Next(snapshot: Handle, entry: *mut ThreadEntry32) -> i32;
        fn OpenThread(access: u32, inherit: i32, thread_id: u32) -> Handle;
        fn GetThreadDescription(thread: Handle, description: *mut *mut u16) -> i32;
        fn LocalFree(memory: *mut c_void) -> *mut c_void;
        fn CloseHandle(handle: Handle) -> i32;
    }

    /// Kernel plus user time of `thread`.
    pub(super) fn thread_times_ms(thread: Handle) -> Option<f64> {
        let mut creation = FileTime::default();
        let mut exit = FileTime::default();
        let mut kernel = FileTime::default();
        let mut user = FileTime::default();
        // SAFETY: `thread` is a valid handle with query access and all four outputs
        // are valid writable FILETIMEs.
        let ok =
            unsafe { GetThreadTimes(thread, &mut creation, &mut exit, &mut kernel, &mut user) };
        if ok == 0 {
            return None;
        }
        let ticks = |time: &FileTime| (u64::from(time.high) << 32) | u64::from(time.low);
        // FILETIME counts 100 ns intervals.
        Some((ticks(&kernel) + ticks(&user)) as f64 / 10_000.0)
    }

    /// Opened threads close when dropped.
    struct Owned(Handle);

    impl Drop for Owned {
        fn drop(&mut self) {
            // SAFETY: the handle was opened by this module and is closed once.
            unsafe { CloseHandle(self.0) };
        }
    }

    pub(super) fn named_thread_cpu_ms(name: &str) -> Option<f64> {
        let wanted: Vec<u16> = name.encode_utf16().collect();
        // SAFETY: a thread snapshot of all processes; closed by `Owned`.
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) };
        if snapshot == INVALID_HANDLE_VALUE {
            return None;
        }
        let snapshot = Owned(snapshot);
        // SAFETY: no preconditions.
        let process = unsafe { GetCurrentProcessId() };
        let mut entry = ThreadEntry32 {
            size: size_of::<ThreadEntry32>() as u32,
            ..Default::default()
        };
        let mut found = None;
        // SAFETY: `entry` is a writable THREADENTRY32 with its size set.
        let mut more = unsafe { Thread32First(snapshot.0, &mut entry) } != 0;
        while more {
            if entry.owner_process_id == process
                && let Some(thread) = open_thread(entry.thread_id)
                && thread_description(&thread).is_some_and(|description| description == wanted)
            {
                if found.is_some() {
                    return None;
                }
                found = Some(thread_times_ms(thread.0)?);
            }
            // SAFETY: as above.
            more = unsafe { Thread32Next(snapshot.0, &mut entry) } != 0;
        }
        found
    }

    fn open_thread(thread_id: u32) -> Option<Owned> {
        // SAFETY: opens a query-only handle; closed by `Owned`.
        let handle = unsafe { OpenThread(THREAD_QUERY_LIMITED_INFORMATION, 0, thread_id) };
        (!handle.is_null()).then_some(Owned(handle))
    }

    /// The UTF-16 name set with `SetThreadDescription`.
    fn thread_description(thread: &Owned) -> Option<Vec<u16>> {
        let mut description: *mut u16 = std::ptr::null_mut();
        // SAFETY: `thread` has query access; on success `description` is a
        // NUL-terminated string freed with LocalFree below.
        let result = unsafe { GetThreadDescription(thread.0, &mut description) };
        if result < 0 || description.is_null() {
            return None;
        }
        let mut length = 0;
        // SAFETY: the string is NUL-terminated.
        while unsafe { *description.add(length) } != 0 {
            length += 1;
        }
        // SAFETY: `length` UTF-16 units precede the terminator.
        let name = unsafe { std::slice::from_raw_parts(description, length) }.to_vec();
        // SAFETY: allocated by GetThreadDescription.
        unsafe { LocalFree(description.cast()) };
        Some(name)
    }
}
