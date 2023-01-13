#![allow(overflowing_literals)]

extern crate foo;
extern crate bar;
use foo::*;
use bar::*;

fn main() {
    // Macros (that rely on macros)
    assert_eq!(STATUS_CREATE_COMPAT_BMP_ERROR!(), 0x803F0008 as i32);
    assert_eq!(STATUS_SOME_OTHER_ERROR!(), 0x803F0009 as i32);

    // We allowed Classy, but not Bassy or IWhatever, but they both get pulled
    // in as they are referenced by Classy.
    let _c = Classy {
        _base: Bassy { count: 123 },
        _base1: IWhatever {},
        klass: 42.0,
        uaf: UsedAsField { v: -1 },
    };

    // Neither FooFlavor nor IcecreamFlavor were explicitly allow listed, but
    // since they are used as parameter, they get pulled in.
    unsafe {
        // C-style enum
        add_flavor(Mocha);
        // enum class
        scoop_flavor(IcecreamFlavor::Chocolate);
    }

    // Namespace support is currently broken.
    // assert_eq!(N1::N2::d1, Directions::Up);
    // assert_eq!(N1::N2::N3::d2, 3);
}
