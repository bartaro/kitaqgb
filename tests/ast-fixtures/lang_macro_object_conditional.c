#define VALUE 7
#define STEP 3
#define FLAG 1

static_assert(VALUE + STEP == 10, "object_macro_add");

#if VALUE == 7
u8 g_if_ok[1];
#else
u8 g_if_bad[-1];
#endif

#define EXPR (2 + 3)
#if EXPR * 2 == 10
u8 g_if_expr_ok[1];
#else
u8 g_if_expr_bad[-1];
#endif

#ifdef FLAG
u8 g_ifdef_ok[1];
#else
u8 g_ifdef_bad[-1];
#endif

#ifndef MISSING_FLAG
u8 g_ifndef_ok[1];
#else
u8 g_ifndef_bad[-1];
#endif

#if defined(FLAG) && !defined(MISSING_FLAG)
u8 g_defined_ok[1];
#else
u8 g_defined_bad[-1];
#endif

#if 0
u8 g_elif_bad0[-1];
#elif VALUE == 7
u8 g_elif_ok[1];
#else
u8 g_elif_bad1[-1];
#endif

#undef FLAG
#ifdef FLAG
u8 g_undef_bad[-1];
#else
u8 g_undef_ok[1];
#endif

#define A B
#define B 5
static_assert(A == 5, "recursive_object_macro");

#undef VALUE
#define VALUE 9
static_assert(VALUE == 9, "redefine_object_macro");

u8 sink_macro;

void main() {
    sink_macro = VALUE;
    while (1) {
    }
}
