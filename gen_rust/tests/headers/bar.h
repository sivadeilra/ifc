#include "foo.h"

#if 0
#define BAR_SOMETHING 3000
#define BAR_MORE_STUFF 3001

#define BAR_INCREMENT(x) (x + 1)

#endif

int bar(FooStuff* foo, FooId_t id, FooFlavor flavor);

struct BarState {
    int puppies;
    int kittens;
    FooStuff foo;
};
