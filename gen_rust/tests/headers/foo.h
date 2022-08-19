
#define FOO_A 10
#define FOO_B 20

#define FOO_HEX_VALUE 0x1c00
#define FOO_HEX_VALUE_2    0x4'0000'0000ULL

#define FOO_HEX_MAX       0xffff'ffff
#define FOO_HEX_MAX_U     0xffff'ffffU
#define FOO_HEX_MAX_UL    0xffff'ffffUL
#define FOO_HEX_MAX_LL    0xffff'ffff'ffff'ffffLL
#define FOO_HEX_MAX_ULL   0xffff'ffff'ffff'ffffULL
#define FOO_HEX_MIN       0x8000'0000
#define FOO_HEX_MIN_L     0x8000'0000L
#define FOO_HEX_MIN_LL    0x8000'0000'0000'0000LL

#define FOO_HEX_8000_0001 0x8000'0001L

#define FOO_HEX_8000_0000_0000_0001 0x8000'0000'0000'0001LL

#define FOO_PAREN     (100)

struct FooStuff {
    int a;
    int b;
    float c;
};

#define FOO_DECREMENT(f) ((f)->a--)

int get_foo();
void set_foo(int x);
