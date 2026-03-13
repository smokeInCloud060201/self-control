#[cfg(all(target_os = "macos", feature = "macos_service"))]
use core_foundation::base::TCFType;
#[cfg(all(target_os = "macos", feature = "macos_service"))]
use core_foundation::string::CFString;
#[cfg(all(target_os = "macos", feature = "macos_service"))]
use std::ptr;

#[cfg(all(target_os = "macos", feature = "macos_service"))]
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
}

#[cfg(all(target_os = "macos", feature = "macos_service"))]
pub struct PowerAssertion {
    id: u32,
}

#[cfg(all(target_os = "macos", feature = "macos_service"))]
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

#[cfg(all(target_os = "macos", feature = "macos_service"))]
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

#[cfg(all(target_os = "macos", feature = "macos_service"))]
pub static mut GLOBAL_ASSERTION: Option<PowerAssertion> = None;

#[cfg(all(target_os = "macos", feature = "macos_service"))]
pub fn init_power_management() {
    unsafe {
        if GLOBAL_ASSERTION.is_none() {
            GLOBAL_ASSERTION = PowerAssertion::prevent_display_sleep("SelfControl Agent - Remote Access Active");
        }
    }
}

#[cfg(not(all(target_os = "macos", feature = "macos_service")))]
pub fn init_power_management() {}
