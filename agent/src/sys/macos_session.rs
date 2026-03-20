#[cfg(target_os = "macos")]
use core_foundation::dictionary::CFDictionary;
#[cfg(target_os = "macos")]
use core_foundation::string::CFString;
#[cfg(target_os = "macos")]
use core_foundation::base::{TCFType, CFTypeRef};

#[cfg(target_os = "macos")]
#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGSessionCopyCurrentDictionary() -> CFTypeRef;
}

#[cfg(target_os = "macos")]
pub fn is_login_window() -> bool {
    unsafe {
        let dict_ptr = CGSessionCopyCurrentDictionary();
        if dict_ptr.is_null() {
            return false;
        }
        
        // Wrap the pointer into a CFDictionary
        let dict: CFDictionary<CFString, CFTypeRef> = TCFType::wrap_under_create_rule(dict_ptr as *const _);
        
        // Key for checking the session username
        let key = CFString::from_static_string("kCGSessionUserNameKey");
        
        match dict.find(&key) {
            Some(name_ptr) => {
                use core_foundation::base::CFGetTypeID;
                use core_foundation::string::CFStringGetTypeID;
                
                if CFGetTypeID(*name_ptr as *const _) == CFStringGetTypeID() {
                    let name = CFString::wrap_under_get_rule(*name_ptr as *const _);
                    let name_str = name.to_string();
                    name_str == "loginwindow"
                } else {
                    false
                }
            }
            None => false,
        }
    }
}

#[cfg(not(target_os = "macos"))]
pub fn is_login_window() -> bool {
    false
}
