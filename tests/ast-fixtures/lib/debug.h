#pragma once

#ifndef DEBUG_TRACE_MAX
#define DEBUG_TRACE_MAX 32
#endif

typedef __packed struct {
    const u8* name;
    u16 value;
    u16 frame;
} DebugTraceEntry;

extern DebugTraceEntry kq_debug_trace_log[DEBUG_TRACE_MAX];
extern u8 kq_debug_trace_count;
extern u16 kq_debug_frame;
extern u16 kq_debug_last_assert;

void debug_init();
void debug_set_frame(u16 frame);
void debug_mark_frame(const u8* label);
void debug_trace_u8(const u8* name, u8 value);
void debug_trace_u16(const u8* name, u16 value);
void debug_assert_fail(u16 code);
u8 debug_get_trace_count();
const DebugTraceEntry* debug_get_trace_log();
u16 debug_get_last_assert();

#define KITAQGB_TRACE_U8(name, value) debug_trace_u8((const u8*)(name), (u8)(value))
#define KITAQGB_TRACE_U16(name, value) debug_trace_u16((const u8*)(name), (u16)(value))
#define KITAQGB_TRACE(name, value) debug_trace_u16((const u8*)(name), (u16)(value))
#define KITAQGB_MARK_FRAME(label) debug_mark_frame((const u8*)(label))
#define KITAQGB_ASSERT(cond) do { if (!(cond)) debug_assert_fail((u16)1); } while (0)
#define KITAQGB_ASSERT_CODE(cond, code) do { if (!(cond)) debug_assert_fail((u16)(code)); } while (0)
