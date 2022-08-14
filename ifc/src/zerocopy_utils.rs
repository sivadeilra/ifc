
use zerocopy::FromBytes;

pub fn new_zeroed<T: FromBytes>() -> T {
    unsafe {
        core::mem::zeroed()
    }
}