#include "foo.h"

#define BAR_SOMETHING 3000
#define BAR_MORE_STUFF 3001

#define BAR_INCREMENT(x) (x + 1)

#define STATUS_SOME_OTHER_ERROR     MAKE_HRESULT(SEVERITY_ERROR, FACILITY_WIN32K_NTGDI, 0x9)

int bar(FooStuff* foo, FooId_t id, FooFlavor flavor);

struct BarState {
    int puppies;
    int kittens;
    FooStuff foo;
};
