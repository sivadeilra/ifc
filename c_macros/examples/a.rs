#![allow(warnings)] // we're actually expecting some, from vis rules

use c_macros::c_enum;

/// Blah Blah Blah
#[c_enum(old_name = "DWRITE_FOO", com, unscoped_prefix = "DWRITE_FOO_")]
pub(crate) enum Foo {
    X = 42,
    Y, // = 100,
}

#[c_enum(com, unscoped_prefix = "DWRITE_FOO_")]
enum Bar {
    Z = 100,
}

#[c_enum(flags)]
enum SomeFlags {
    A = 1,
    B = 2,
    C = 4,
    D = 8,
}

pub fn main() {}
