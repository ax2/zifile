use zifile_core::ProgressSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Indicator {
    Hidden,
    Indeterminate,
    Normal { completed: u64, total: u64 },
    Paused { completed: u64, total: u64 },
}

fn indicator(busy: bool, cancelled: bool, snapshot: Option<ProgressSnapshot>) -> Indicator {
    if !busy {
        return Indicator::Hidden;
    }
    let Some(snapshot) = snapshot else {
        return Indicator::Indeterminate;
    };
    let value = if snapshot.total_bytes > 0 {
        Some((snapshot.processed_bytes, snapshot.total_bytes))
    } else if snapshot.total_entries > 0 {
        Some((snapshot.processed_entries, snapshot.total_entries))
    } else {
        None
    };
    match (cancelled, value) {
        (true, Some((completed, total))) => Indicator::Paused {
            completed: completed.min(total),
            total,
        },
        (true, None) => Indicator::Paused {
            completed: 1,
            total: 1,
        },
        (false, Some((completed, total))) => Indicator::Normal {
            completed: completed.min(total),
            total,
        },
        (false, None) => Indicator::Indeterminate,
    }
}

pub fn sync(busy: bool, cancelled: bool, snapshot: Option<ProgressSnapshot>) {
    platform::sync(indicator(busy, cancelled, snapshot));
}

#[cfg(windows)]
mod platform {
    use std::cell::{Cell, RefCell};
    use std::ptr;

    use windows::Win32::Foundation::{HWND, LPARAM, RPC_E_CHANGED_MODE};
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED, CoCreateInstance, CoInitializeEx,
    };
    use windows::Win32::System::Threading::GetCurrentProcessId;
    use windows::Win32::UI::Shell::{
        ITaskbarList3, TBPF_INDETERMINATE, TBPF_NOPROGRESS, TBPF_NORMAL, TBPF_PAUSED, TaskbarList,
    };
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };
    use windows::core::BOOL;

    use super::Indicator;

    thread_local! {
        static COM_INITIALIZED: Cell<bool> = const { Cell::new(false) };
        static CLIENT: RefCell<Option<TaskbarClient>> = const { RefCell::new(None) };
    }

    struct TaskbarClient {
        taskbar: ITaskbarList3,
        window: HWND,
    }

    pub(super) fn sync(indicator: Indicator) {
        CLIENT.with(|client| {
            let mut client = client.borrow_mut();
            if client.is_none() {
                *client = TaskbarClient::new().ok();
            }
            if let Some(client) = client.as_ref() {
                let _ = client.apply(indicator);
            }
        });
    }

    impl TaskbarClient {
        fn new() -> windows::core::Result<Self> {
            let window = find_process_window().ok_or_else(windows::core::Error::from_thread)?;
            initialize_com()?;
            // SAFETY: COM is initialized on this thread and the class/interface pair is fixed.
            let taskbar: ITaskbarList3 =
                unsafe { CoCreateInstance(&TaskbarList, None, CLSCTX_INPROC_SERVER)? };
            // SAFETY: `taskbar` is a newly created live ITaskbarList3 interface.
            unsafe { taskbar.HrInit()? };
            Ok(Self { taskbar, window })
        }

        fn apply(&self, indicator: Indicator) -> windows::core::Result<()> {
            // SAFETY: `self.taskbar` and the process-owned window handle remain live in this client.
            unsafe {
                match indicator {
                    Indicator::Hidden => {
                        self.taskbar.SetProgressState(self.window, TBPF_NOPROGRESS)
                    }
                    Indicator::Indeterminate => self
                        .taskbar
                        .SetProgressState(self.window, TBPF_INDETERMINATE),
                    Indicator::Normal { completed, total } => {
                        self.taskbar.SetProgressState(self.window, TBPF_NORMAL)?;
                        self.taskbar
                            .SetProgressValue(self.window, completed.min(total), total)
                    }
                    Indicator::Paused { completed, total } => {
                        self.taskbar.SetProgressState(self.window, TBPF_PAUSED)?;
                        self.taskbar
                            .SetProgressValue(self.window, completed.min(total), total)
                    }
                }
            }
        }
    }

    fn initialize_com() -> windows::core::Result<()> {
        COM_INITIALIZED.with(|initialized| {
            if initialized.get() {
                return Ok(());
            }
            // SAFETY: called once per thread before using taskbar COM interfaces.
            let result = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
            if result.is_ok() || result == RPC_E_CHANGED_MODE {
                initialized.set(true);
                Ok(())
            } else {
                Err(result.into())
            }
        })
    }

    fn find_process_window() -> Option<HWND> {
        struct Search {
            process_id: u32,
            window: HWND,
        }

        unsafe extern "system" fn visit(window: HWND, parameter: LPARAM) -> BOOL {
            // SAFETY: `EnumWindows` receives a pointer to `Search` that lives for the entire call.
            let search = unsafe { &mut *(parameter.0 as *mut Search) };
            let mut process_id = 0;
            // SAFETY: Windows supplied `window` and `process_id` is writable for this call.
            unsafe { GetWindowThreadProcessId(window, Some(&mut process_id)) };
            // SAFETY: `window` is supplied by the active `EnumWindows` callback.
            if process_id == search.process_id && unsafe { IsWindowVisible(window).as_bool() } {
                search.window = window;
                false.into()
            } else {
                true.into()
            }
        }

        let mut search = Search {
            // SAFETY: this parameterless API returns the identifier of the calling process.
            process_id: unsafe { GetCurrentProcessId() },
            window: HWND(ptr::null_mut()),
        };
        // SAFETY: `search` remains pinned on this stack until synchronous enumeration returns.
        let _ = unsafe { EnumWindows(Some(visit), LPARAM(ptr::from_mut(&mut search) as isize)) };
        (!search.window.0.is_null()).then_some(search.window)
    }
}

#[cfg(not(windows))]
mod platform {
    use super::Indicator;

    pub(super) fn sync(_indicator: Indicator) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_byte_progress_and_clamps_completed_work() {
        assert_eq!(
            indicator(
                true,
                false,
                Some(ProgressSnapshot {
                    processed_entries: 2,
                    total_entries: 4,
                    processed_bytes: 130,
                    total_bytes: 100,
                })
            ),
            Indicator::Normal {
                completed: 100,
                total: 100,
            }
        );
    }

    #[test]
    fn maps_unknown_and_cancelled_work() {
        assert_eq!(indicator(true, false, None), Indicator::Indeterminate);
        assert_eq!(
            indicator(true, true, Some(ProgressSnapshot::default())),
            Indicator::Paused {
                completed: 1,
                total: 1,
            }
        );
        assert_eq!(indicator(false, false, None), Indicator::Hidden);
    }
}
