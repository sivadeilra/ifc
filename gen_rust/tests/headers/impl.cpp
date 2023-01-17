#include "foo.h"

extern "C" void __fastfail(unsigned int);

void assert(bool condition) {
    if (!condition) {
        __fastfail(42);
    }
}

int get_foo(FooId_t id) {
    return id;
}

void set_foo(int x) {}

void add_flavor(FooFlavor ff) {
    assert(ff == FooFlavor::Mocha);
}

void scoop_flavor(IcecreamFlavor i) {
    assert(i == IcecreamFlavor::Chocolate);
}

void all_the_flavor(UseAsPointer*, UseAsReference&, UseAsReference2&&, UseAsArray[1], const UseAsQualifiedRef&) {}

