module;
export module vars_mod;

extern int EXTERN_VAR_DEF;

int init_var(int x);

int VAR_DEF;
int VAR_DEF_WITH_CONST_INIT = 42;
int VAR_DEF_WITH_DYN_INIT = init_var(300);



constexpr wchar_t CONSTEXPR_VAR_WCHAR_T = L'A';

constexpr char CONSTEXPR_VAR_CHAR = 123;
constexpr signed char CONSTEXPR_VAR_SCHAR = 123;
constexpr unsigned char CONSTEXPR_VAR_UCHAR = 123;

constexpr short CONSTEXPR_VAR_SHORT = 123;
constexpr int CONSTEXPR_VAR_INT = 123;
constexpr long CONSTEXPR_VAR_LONG = 123;
constexpr long long CONSTEXPR_VAR_LONGLONG = 123;

constexpr __int8 CONSTEXPR_VAR_INT8 = 123;
constexpr __int16 CONSTEXPR_VAR_INT16 = 123;
constexpr __int32 CONSTEXPR_VAR_INT32 = 123;
constexpr __int64 CONSTEXPR_VAR_INT64 = 123;

constexpr unsigned short CONSTEXPR_VAR_USHORT = 123;
constexpr unsigned int CONSTEXPR_VAR_UINT = 123;
constexpr unsigned long CONSTEXPR_VAR_ULONG = 123;
constexpr unsigned long long CONSTEXPR_VAR_ULONGLONG = 123;

constexpr unsigned __int8 CONSTEXPR_VAR_UINT8 = 123;
constexpr unsigned __int16 CONSTEXPR_VAR_UINT16 = 123;
constexpr unsigned __int32 CONSTEXPR_VAR_UINT32 = 123;
constexpr unsigned __int64 CONSTEXPR_VAR_UINT64 = 123;


static const int STATIC_CONST_INT = 123;

// nyi: dyad ops
// constexpr const char* CSTRING = "hello, world";

extern int INT_ARRAY_UNSIZED[];

extern int INT_ARRAY_SIZED[100];

extern const int CONST_INT_ARRAY_UNSIZED[];

extern const int CONST_INT_ARRAY_SIZED[100];
