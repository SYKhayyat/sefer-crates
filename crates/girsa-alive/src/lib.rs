//! Is the process that wrote this file still running?
//!
//! # What this can and cannot tell you
//!
//! It can tell you a process is **gone**. It cannot tell you the one you meant
//! is *there*.
//!
//! Process ids are reused. Every operating system here hands out a bounded
//! range of them and wraps around, so a pid recorded an hour ago and answering
//! [`Liveness::Alive`] today may belong to a text editor that started
//! afterwards. There is no fix for this at this layer — the fix is to ask the
//! process who it is, which is exactly what `/health` does.
//!
//! So the contract is one-directional and everything here depends on it:
//!
//! * [`Liveness::Gone`] is **trustworthy**. Nothing is running under that pid,
//!   so whatever wrote the file is not there, and a caller may stop early.
//! * [`Liveness::Alive`] means only *some* process holds that pid. It is a
//!   reason to go on and ask, never an answer on its own.
//! * [`Liveness::Unknown`] is the operating system declining to say. Treated
//!   exactly like `Alive`: go on and ask.
//!
//! A check that is only ever allowed to say *no* is worth having anyway,
//! because *no* is the case that otherwise costs a timeout.
//!
//! # Why this is its own crate
//!
//! The workspace forbids `unsafe_code`. That is right for crates that parse
//! refs and print citations, and it is why `girsa-post` carried a `pid` field
//! for a release with a comment admitting the check it implied could not be
//! written there. Neither implementing it nor deleting the field was available:
//! the field is genuinely useful when you are looking at two endpoint files
//! wondering which window wrote which.
//!
//! The exception is therefore a crate rather than a lint escape in the middle
//! of a large one. It is three functions wide, it does not take the workspace
//! lint table (see `Cargo.toml`), and every `unsafe` block in the tree is in
//! this file where a reviewer can hold all of them in view at once.
//!
//! # Why the declarations are written out here
//!
//! `libc` and `windows-sys` both exist and both are respectable. This needs
//! three symbols on Windows and one on Unix, all of them stable ABI for
//! decades, and `girsa-post` already declined to compile an HTTP client into
//! both applications for two request kinds. The same argument applies with more
//! force to a dependency whose Windows edition is measured in megabytes of
//! generated bindings.

/// What the operating system says about a process id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// Some process holds this pid. **Not necessarily the one you meant** —
    /// see the module note on reuse.
    Alive,
    /// Nothing holds this pid. Trustworthy, and the only answer that is.
    Gone,
    /// The operating system would not say. Indistinguishable from `Alive` for
    /// every purpose here, and separate from it so that a log can tell the two
    /// apart when something is misbehaving.
    Unknown,
}

impl Liveness {
    /// Whether this answer rules the process out.
    ///
    /// The only question callers should be asking, and phrased so the safe
    /// reading is the easy one: `Unknown` is not `gone`.
    #[must_use]
    pub fn gone(self) -> bool {
        self == Self::Gone
    }
}

/// Ask whether `pid` is a live process.
///
/// Reads the answer and nothing else — it does not signal, attach to, or wait
/// on the process.
///
/// Pid `0` is [`Liveness::Unknown`] on both platforms rather than passed
/// through: on Unix `kill(0, 0)` addresses the caller's whole process group,
/// which is a different question with a dangerous shape, and on Windows it is
/// the System Idle Process. An endpoint file recording `0` is a bug in whoever
/// wrote it, and the honest answer to a malformed question is *I do not know*.
#[must_use]
pub fn alive(pid: u32) -> Liveness {
    if pid == 0 {
        return Liveness::Unknown;
    }
    platform::alive(pid)
}

#[cfg(unix)]
mod platform {
    use super::Liveness;

    extern "C" {
        /// Signal `0` is the documented existence probe: all the permission
        /// checking, none of the signal. POSIX.1-2008, `kill(2)`.
        fn kill(pid: i32, sig: i32) -> i32;
    }

    /// `ESRCH` — no such process. The one errno that means *gone*.
    const ESRCH: i32 = 3;
    /// `EPERM` — it exists, and it is not ours to signal. Which is an answer:
    /// a process you may not touch is still a process.
    const EPERM: i32 = 1;

    pub(super) fn alive(pid: u32) -> Liveness {
        // A pid above `i32::MAX` cannot name a process on any Unix here, so it
        // is `Gone` by construction rather than by asking — and casting it
        // would wrap into a negative, which `kill` reads as a process *group*.
        let Ok(pid) = i32::try_from(pid) else {
            return Liveness::Gone;
        };
        // SAFETY: `kill` with signal 0 sends nothing and only reports whether
        // the pid could be signalled. No pointers cross the boundary, the
        // return value is a plain `int`, and `pid` is positive (0 and negatives
        // address groups and are excluded above and in `alive`).
        if unsafe { kill(pid, 0) } == 0 {
            return Liveness::Alive;
        }
        match std::io::Error::last_os_error().raw_os_error() {
            Some(ESRCH) => Liveness::Gone,
            Some(EPERM) => Liveness::Alive,
            _ => Liveness::Unknown,
        }
    }
}

#[cfg(windows)]
mod platform {
    use super::Liveness;

    type Handle = *mut core::ffi::c_void;

    extern "system" {
        fn OpenProcess(access: u32, inherit_handle: i32, pid: u32) -> Handle;
        fn GetExitCodeProcess(process: Handle, code: *mut u32) -> i32;
        fn CloseHandle(object: Handle) -> i32;
    }

    /// The narrowest right that still permits `GetExitCodeProcess`, and the one
    /// that works across a privilege boundary where `PROCESS_QUERY_INFORMATION`
    /// does not. Asking for more than the question needs is how a check like
    /// this ends up reporting *gone* for a process running as another user.
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    /// What `GetExitCodeProcess` writes for a process that has not exited.
    ///
    /// It is famously also a legal exit code, which would matter if this were
    /// reading the code. It is not: it is reading *has it exited*, and a
    /// process that exited with 259 is a process nothing will answer on anyway.
    const STILL_ACTIVE: u32 = 259;
    /// `ERROR_INVALID_PARAMETER` — there is no such process.
    const ERROR_INVALID_PARAMETER: i32 = 87;
    /// `ERROR_ACCESS_DENIED` — there is, and it is not ours to open.
    const ERROR_ACCESS_DENIED: i32 = 5;

    pub(super) fn alive(pid: u32) -> Liveness {
        // SAFETY: `OpenProcess` takes three integers and returns either a
        // handle or null. Nothing is dereferenced before the null check below.
        let process = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
        if process.is_null() {
            return match std::io::Error::last_os_error().raw_os_error() {
                Some(ERROR_INVALID_PARAMETER) => Liveness::Gone,
                Some(ERROR_ACCESS_DENIED) => Liveness::Alive,
                _ => Liveness::Unknown,
            };
        }

        let mut code: u32 = 0;
        // SAFETY: `process` is a live handle from the call above — the null
        // case returned. `code` is a stack `u32` that outlives the call, which
        // is the only pointer crossing, and `GetExitCodeProcess` writes at most
        // four bytes to it. The handle is closed on both paths below.
        let read = unsafe { GetExitCodeProcess(process, &raw mut code) };
        // SAFETY: `process` came from `OpenProcess` and has not been closed. It
        // is closed exactly once — nothing above returns between the two calls.
        unsafe { CloseHandle(process) };

        if read == 0 {
            // The handle opened and the question failed anyway. Something is
            // wrong with the ask, not with the process.
            return Liveness::Unknown;
        }
        if code == STILL_ACTIVE {
            Liveness::Alive
        } else {
            Liveness::Gone
        }
    }
}

#[cfg(not(any(unix, windows)))]
mod platform {
    use super::Liveness;

    /// Somewhere that is neither. There is no check to make, and claiming
    /// `Gone` here would make a caller skip a process that is running.
    pub(super) fn alive(_pid: u32) -> Liveness {
        Liveness::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The process asking is running. If this fails, the call is not reaching
    /// the operating system at all.
    #[test]
    fn we_are_alive() {
        assert_eq!(alive(std::process::id()), Liveness::Alive);
    }

    /// A pid nothing can hold.
    ///
    /// `u32::MAX` is above every platform's `pid_max` here — Linux caps at
    /// 2^22, and Windows pids are multiples of four below 2^32 but are not
    /// handed out anywhere near the top. This is the assertion that the crate
    /// can say *no*, which is the only answer it is trusted for.
    #[test]
    fn a_pid_nothing_holds_is_gone() {
        assert_eq!(alive(u32::MAX), Liveness::Gone);
        assert!(alive(u32::MAX).gone());
    }

    /// A process that has exited is gone, and this is the case the whole crate
    /// exists for: a sibling application that crashed without withdrawing its
    /// endpoint file.
    #[test]
    fn a_process_that_has_exited_is_gone() {
        let spawned = std::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
            .args(if cfg!(windows) {
                ["/C", "exit 0"]
            } else {
                ["-c", "exit 0"]
            })
            .spawn();
        let Ok(mut child) = spawned else {
            panic!("could not spawn a process to watch exit");
        };
        let pid = child.id();
        assert_eq!(alive(pid), Liveness::Alive, "it has not been reaped yet");
        // Waited on, so the child is reaped rather than a zombie — an unreaped
        // child on Unix still holds its pid and would answer `Alive`, which is
        // correct and is not what this test is about.
        let _ = child.wait();
        assert_eq!(alive(pid), Liveness::Gone);
    }

    /// Zero is not passed through to either platform.
    #[test]
    fn zero_is_not_a_question() {
        assert_eq!(alive(0), Liveness::Unknown);
        assert!(!alive(0).gone(), "unknown must never read as gone");
    }
}
