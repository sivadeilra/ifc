
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

#define SEVERITY_SUCCESS    0
#define SEVERITY_ERROR      1

#define FACILITY_WIN32K_NTGDI 0x3F

#define MAKE_HRESULT(sev,fac,code) \
    ((HRESULT) (((unsigned long)(sev)<<31) | ((unsigned long)(fac)<<16) | ((unsigned long)(code))) )

#define STATUS_CREATE_COMPAT_BMP_ERROR     MAKE_HRESULT(SEVERITY_ERROR, FACILITY_WIN32K_NTGDI, 0x8)

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

enum class IcecreamFlavor {
    Chocolate,
    Vanilla,
};

class Bassy {
    public:
    int count;
};

struct UsedAsField {
    int v;
};

class Classy : public Bassy, public IWhatever {
    public:
    float klass;
    UsedAsField uaf;
};

extern "C" int get_foo(FooId_t id);
extern "C" void set_foo(int x);

extern "C" void add_flavor(FooFlavor ff);
extern "C" void scoop_flavor(IcecreamFlavor i);

struct UseAsPointer{};
struct UseAsReference{};
struct UseAsReference2{};
struct UseAsArray{};
struct UseAsQualifiedRef{};

extern "C" void all_the_flavor(UseAsPointer*, UseAsReference&, UseAsReference2&&, const UseAsArray(&)[1], const UseAsQualifiedRef&);

enum class Directions {
    Up,
    Down,
};

namespace N1 {
    namespace N2 {
        inline constexpr Directions d1 = Directions::Up;
    }
}

namespace N1::N2::N3 {
    inline constexpr int d2 = 3;
    inline constexpr int ignored = 4;
}

struct IsBlocked {};
struct UsesBlocked {
    IsBlocked ib;
};

struct HasOrphan {
    struct IsOrphan * o;
};
