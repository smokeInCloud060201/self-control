#[cfg(target_os = "macos")]
use core_foundation::base::TCFType;
#[cfg(target_os = "macos")]
use core_foundation::string::CFString;
#[cfg(target_os = "macos")]
use std::ptr;

#[cfg(target_os = "macos")]
#[link(name = "IOKit", kind = "framework")]
extern "C" {
    fn IOPMAssertionCreateWithDescription(
        assertion_type: core_foundation::base::CFTypeRef,
        name: core_foundation::base::CFTypeRef,
        details: core_foundation::base::CFTypeRef,
        human_readable: core_foundation::base::CFTypeRef,
        localization_bundle_path: core_foundation::base::CFTypeRef,
        timeout: f64, // CFTimeInterval
        timeout_action: core_foundation::base::CFTypeRef,
        assertion_id: *mut u32,
    ) -> i32; // IOReturn

    fn IOPMAssertionRelease(assertion_id: u32) -> i32;

    fn IOPMAssertionDeclareUserActivity(
        assertion_name: core_foundation::base::CFTypeRef,
        user_activity_type: u32,
        assertion_id: *mut u32,
    ) -> i32;
}

#[cfg(target_os = "macos")]
pub struct PowerAssertion {
    id: u32,
}

#[cfg(target_os = "macos")]
impl PowerAssertion {
    pub fn prevent_display_sleep(name: &str) -> Option<Self> {
        let mut id: u32 = 0;
        
        // IOKit constants
        let assertion_type = CFString::from_static_string("NoDisplaySleepAssertion");
        let name_cf = CFString::new(name);
        
        unsafe {
            let result = IOPMAssertionCreateWithDescription(
                assertion_type.as_concrete_TypeRef() as *const _,
                name_cf.as_concrete_TypeRef() as *const _,
                ptr::null(), // details
                ptr::null(), // human_readable
                ptr::null(), // localization_bundle_path
                0.0,         // timeout (0 = never)
                ptr::null(), // timeout_action
                &mut id,
            );

            if result == 0 {
                tracing::info!(id = id, "Successfully created display sleep prevention assertion");
                Some(PowerAssertion { id })
            } else {
                tracing::warn!(error = result, "Failed to create power assertion");
                None
            }
        }
    }
}

#[cfg(target_os = "macos")]
impl Drop for PowerAssertion {
    fn drop(&mut self) {
        unsafe {
            let result = IOPMAssertionRelease(self.id);
            if result == 0 {
                tracing::info!(id = self.id, "Successfully released power assertion");
            } else {
                tracing::warn!(id = self.id, error = result, "Failed to release power assertion");
            }
        }
    }
}

#[cfg(target_os = "macos")]
pub static GLOBAL_ASSERTION: std::sync::Mutex<Option<PowerAssertion>> = std::sync::Mutex::new(None);

#[cfg(target_os = "macos")]
pub fn init_power_management() {
    if let Ok(mut assertion) = GLOBAL_ASSERTION.lock() {
        if assertion.is_none() {
            *assertion = PowerAssertion::prevent_display_sleep("SelfControl Agent - Remote Access Active");
            spawn_periodic_wake();
        }
    }
}

#[cfg(target_os = "macos")]
fn spawn_periodic_wake() {
    std::thread::spawn(|| {
        let name = CFString::from_static_string("SelfControl Agent Wake Activity");
        loop {
            let mut id: u32 = 0;
            unsafe {
                // kIOPMUserActivityTypeLive (0) is generally sufficient to wake/keep display on
                IOPMAssertionDeclareUserActivity(name.as_concrete_TypeRef() as *const _, 0, &mut id);
            }
            std::thread::sleep(std::time::Duration::from_secs(30));
        }
    });
}

#[cfg(not(target_os = "macos"))]
pub fn init_power_management() {}
