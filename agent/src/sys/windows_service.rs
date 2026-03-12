#[cfg(target_os = "windows")]
use windows::Win32::System::Threading::{GetCurrentThreadId, SetThreadDesktop};
#[cfg(target_os = "windows")]
use windows::Win32::UI::WindowsAndMessaging::{OpenInputDesktop, CloseDesktop, DESKTOP_CONTROL_FLAGS};
#[cfg(target_os = "windows")]
use windows::Win32::Foundation::{BOOL, HANDLE};
#[cfg(target_os = "windows")]
use windows::Win32::System::RemoteDesktop::WTSGetActiveConsoleSessionId;
#[cfg(target_os = "windows")]
use tracing::{info, warn, error, debug};

#[cfg(target_os = "windows")]
pub struct AutoDesktop {
    handle: Option<HANDLE>,
}

#[cfg(target_os = "windows")]
impl AutoDesktop {
    pub fn new() -> Self {
        Self {
            handle: switch_to_secure_desktop(),
        }
    }
}

#[cfg(target_os = "windows")]
impl Drop for AutoDesktop {
    fn drop(&mut self) {
        if let Some(h) = self.handle {
            restore_desktop(h);
        }
    }
}

#[cfg(target_os = "windows")]
pub fn switch_to_secure_desktop() -> Option<HANDLE> {
    #[cfg(feature = "windows_service")]
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
    #[cfg(not(feature = "windows_service"))]
    None
}

#[cfg(target_os = "windows")]
pub fn restore_desktop(h_desktop: HANDLE) {
    #[cfg(feature = "windows_service")]
    unsafe {
        let _ = CloseDesktop(h_desktop);
    }
    #[cfg(not(feature = "windows_service"))]
    let _ = h_desktop;
}
