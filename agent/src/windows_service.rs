#[cfg(all(target_os = "windows", feature = "windows_service"))]
use windows::Win32::System::Threading::{GetCurrentThreadId, SetThreadDesktop};
#[cfg(all(target_os = "windows", feature = "windows_service"))]
use windows::Win32::UI::WindowsAndMessaging::{OpenInputDesktop, CloseDesktop, DESKTOP_CONTROL_FLAGS};
#[cfg(all(target_os = "windows", feature = "windows_service"))]
use windows::Win32::Foundation::{BOOL, HANDLE};
#[cfg(all(target_os = "windows", feature = "windows_service"))]
use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
#[cfg(all(target_os = "windows", feature = "windows_service"))]
use tracing::{info, warn, error, debug};

#[cfg(all(target_os = "windows", feature = "windows_service"))]
pub struct AutoDesktop {
    handle: Option<HANDLE>,
}

#[cfg(all(target_os = "windows", feature = "windows_service"))]
impl AutoDesktop {
    pub fn new() -> Self {
        Self {
            handle: switch_to_secure_desktop(),
        }
    }
}

#[cfg(all(target_os = "windows", feature = "windows_service"))]
impl Drop for AutoDesktop {
    fn drop(&mut self) {
        if let Some(h) = self.handle {
            restore_desktop(h);
        }
    }
}

#[cfg(all(target_os = "windows", feature = "windows_service"))]
pub fn switch_to_secure_desktop() -> Option<HANDLE> {
    unsafe {
        // 1. Open the Winlogon desktop
        let h_desktop = OpenInputDesktop(0, false, 0x01ff); // GENERIC_ALL equivalent for desktops
        if h_desktop.is_invalid() {
            debug!("Could not open input desktop (maybe already on it or access denied)");
            return None;
        }

        // 2. Set it for the current thread
        if SetThreadDesktop(h_desktop).is_ok() {
            info!("Successfully switched thread to secure desktop");
            return Some(h_desktop);
        } else {
            warn!("Failed to set thread desktop");
            let _ = CloseDesktop(h_desktop);
            None
        }
    }
}

#[cfg(all(target_os = "windows", feature = "windows_service"))]
pub fn restore_desktop(h_desktop: HANDLE) {
    unsafe {
        let _ = CloseDesktop(h_desktop);
    }
}

#[cfg(not(all(target_os = "windows", feature = "windows_service")))]
pub fn switch_to_secure_desktop() -> Option<i32> {
    None
}
