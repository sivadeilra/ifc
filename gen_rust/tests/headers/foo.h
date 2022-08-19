
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

#define FOO_DECREMENT(f) ((f)->a--)

typedef unsigned int FooId_t;

__interface IWhatever {
    virtual void whatever() = 0;
};

struct FooStuff {
    FooId_t id;
    int a;
    int b;
    float c;
};

enum FooFlavor {
    Reversi,
    Mocha,
    HighGround,
};

class Bassy {
    public:
    int count;
};

class Classy : public Bassy, public IWhatever {
    public:
    float klass;
};

int get_foo(FooId_t id);
void set_foo(int x);
