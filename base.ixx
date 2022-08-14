module;

#include "real_stuff.h"

export module base_mod;

#if 0

export namespace base_ns {
    export int get_x() { return 42; }
}

export namespace cookie::wookie {
    namespace bookie {

        class POINT {
            public:
            int x;
            int y;
            float zap;

            void eat_sharks();
        };

        int get_y() { return 42; }
    }
}



#endif

typedef unsigned long long FAVORITE_INT;

export enum class ZorbaEnumClass : FAVORITE_INT {
    A = 10,
    B = 20,
};

constexpr int make_int_bigger(int x) {
    return x + 1;
}

export enum PLAIN_OLD_ENUM {
    PLAIN_FOO = 10,
    PLAIN_BAR = make_int_bigger(20),
    PLAIN_NEXT,
    PLAIN_MORE,
};

#if 0

typedef struct {
    int f1;
    int f2;
} struct_defined_with_typedef, also_defined_with_typedef;

export 
[[rust(is_safe)]]
[[deprecated]]
void do_something_int(int x);

export void foo(int x, int y);


#endif