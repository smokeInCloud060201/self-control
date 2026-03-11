use super::ffi::*;
use std::mem;

#[derive(PartialEq, Eq, Debug, Clone, Copy)]
#[repr(C)]
pub struct Display(u32);

impl Display {
    pub fn primary() -> Display {
        Display(unsafe { CGMainDisplayID() })
    }

    pub fn online() -> Result<Vec<Display>, CGError> {
        unsafe {
            let mut arr = mem::MaybeUninit::<[u32; 16]>::uninit();
            let mut len: u32 = 0;

            match CGGetOnlineDisplayList(16, arr.as_mut_ptr() as *mut u32, &mut len) {
                CGError::Success => (),
                x => return Err(x)
            }

            let arr = arr.assume_init();
            let mut res = Vec::with_capacity(len as usize);
            for i in 0..len as usize {
                res.push(Display(*arr.get_unchecked(i)));
            }
            Ok(res)
        }
    }

    pub fn id(self) -> u32 {
        self.0
    }

    pub fn width(self) -> usize {
        unsafe { CGDisplayPixelsWide(self.0) }
    }

    pub fn height(self) -> usize {
        unsafe { CGDisplayPixelsHigh(self.0) }
    }

    pub fn logical_width(self) -> usize {
        unsafe { CGDisplayBounds(self.0).size.width as usize }
    }

    pub fn logical_height(self) -> usize {
        unsafe { CGDisplayBounds(self.0).size.height as usize }
    }

    pub fn origin_x(self) -> i32 {
        unsafe { CGDisplayBounds(self.0).origin.x as i32 }
    }

    pub fn origin_y(self) -> i32 {
        unsafe { CGDisplayBounds(self.0).origin.y as i32 }
    }

    pub fn is_builtin(self) -> bool {
        unsafe { CGDisplayIsBuiltin(self.0) != 0 }
    }

    pub fn is_primary(self) -> bool {
        unsafe { CGDisplayIsMain(self.0) != 0 }
    }

    pub fn is_active(self) -> bool {
        unsafe { CGDisplayIsActive(self.0) != 0 }
    }

    pub fn is_online(self) -> bool {
        unsafe { CGDisplayIsOnline(self.0) != 0 }
    }
}
