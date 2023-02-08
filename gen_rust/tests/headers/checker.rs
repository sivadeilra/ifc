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

    unsafe {
        // Neither FooFlavor nor IcecreamFlavor were explicitly allow listed, but
        // since they are used as parameter, they get pulled in.
        // C-style enum
        add_flavor(Mocha);
        // enum class
        scoop_flavor(IcecreamFlavor::Chocolate);

        // Different ways to wrap types: pointer, reference, &&, array, const ref.
        all_the_flavor(&mut UseAsPointer{}, &mut UseAsReference{}, &mut (&mut UseAsReference2{} as *mut _), &[UseAsArray{}], &UseAsQualifiedRef{});
    }

    // Read items in namespaces.
    assert_eq!(N1::N2::d1, Directions::Up);
    assert_eq!(N1::N2::N3::d2, 3);

    // Type that uses a blocked type: definition for the blocked type is hand-
    // written above.
    let _uses = UsesBlocked {
        ib: IsBlocked {}
    };
}
