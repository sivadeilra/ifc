// #![no_std]

#![feature(register_tool)]
#![register_tool(cpp)]

pub fn get_foo() -> i32 {
    42
}

pub fn set_foo(_x: i32) {}

pub mod super_cool {
    use super::*;

    #[repr(C)]
    pub struct CoolStuff {
        pub x: i32,
        pub y: f32,
        pub z: String,
        pub ntstatus: NTSTATUS,
        pub hresult: HRESULT,
    }

    struct SneakyStuff {
        x: i32,
    }

    #[no_mangle]
    pub unsafe extern "C" fn extra_cool_stuff(stuff: &mut CoolStuff) {
        let sneaky = SneakyStuff { x: 100 };
        stuff.x += sneaky.x;
        stuff.x += 1;
        stuff.y -= 1.0;
    }
}

#[repr(transparent)]
pub struct HRESULT(pub i32);

pub const E_FAIL: HRESULT = HRESULT(-42);

pub const NUM_FOO: usize = 42;
pub const NUM_BAR: usize = 10 + 20;

pub const NUM_ZAP: u32 = 100;

pub const THE_NAME: &str = "Blah blah blah";

pub type NTSTATUS = i32;

pub const NTSTATUS_ACCESS_VIOLATION: NTSTATUS = 0xc0000005u32 as i32;
