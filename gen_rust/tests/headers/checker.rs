#![allow(overflowing_literals)]

extern crate foo;
extern crate bar;
use foo::*;
use bar::*;

fn main() {
    assert_eq!(STATUS_CREATE_COMPAT_BMP_ERROR!(), 0x803F0008 as i32);
    assert_eq!(STATUS_SOME_OTHER_ERROR!(), 0x803F0009 as i32);
}
