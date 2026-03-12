#[cfg(target_os = "windows")]
use windows::Win32::System::StationsAndDesktops::{OpenInputDesktop, CloseDesktop, SetThreadDesktop, DESKTOP_ACCESS_FLAGS, DESKTOP_CONTROL_FLAGS, HDESK};
#[cfg(target_os = "windows")]
use tracing::{info, warn, debug};

#[cfg(target_os = "windows")]
pub struct AutoDesktop {
    handle: Option<HDESK>,
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
pub fn switch_to_secure_desktop() -> Option<HDESK> {
    unsafe {
        // 1. Open the Winlogon desktop
        let h_desktop = match OpenInputDesktop(DESKTOP_CONTROL_FLAGS(0), false, DESKTOP_ACCESS_FLAGS(0x01ff)) {
            Ok(h) => h,
            Err(e) => {
                debug!("Could not open input desktop: {:?}", e);
                return None;
            }
        };

        // 2. Set it for the current thread
        if SetThreadDesktop(h_desktop).is_ok() {
            info!("Successfully switched thread to secure desktop");
            Some(h_desktop)
        } else {
            warn!("Failed to set thread desktop");
            let _ = CloseDesktop(h_desktop);
            None
        }
    }
}

#[cfg(target_os = "windows")]
pub fn restore_desktop(h_desktop: HDESK) {
    unsafe {
        let _ = CloseDesktop(h_desktop);
    }
}

#[cfg(not(target_os = "windows"))]
pub struct AutoDesktop;
#[cfg(not(target_os = "windows"))]
impl AutoDesktop { pub fn new() -> Self { Self } }
