#![allow(overflowing_literals)]

extern crate foo;
extern crate bar;

fn main() {
    assert_eq!(foo::STATUS_CREATE_COMPAT_BMP_ERROR!(), 0x803F0008 as i32);
}
