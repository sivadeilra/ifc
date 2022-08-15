module;
export module enums_mod;

enum SIMPLE_ENUM {
    SIMPLE_A = 10,
    SIMPLE_B = 20,

    SIMPLE_NEGATIVE = -50,

    REPEATED_A = 10,
};

enum class FancyEnum : long long {
    Red,
    Green,
    Blue,
};

enum class UpOrDown : bool {
    Up = false,
    Down = true,
};

enum class ENUM_WITH_CHAR_BASE : char {
    A = 10,
    B = -10,
};

constexpr unsigned int SIMPLE_CONSTANT = 123;
