#include "debug.h"

DebugTraceEntry kq_debug_trace_log[DEBUG_TRACE_MAX];
u8 kq_debug_trace_count;
u16 kq_debug_frame;
u16 kq_debug_last_assert;

void debug_init()
{
    kq_debug_trace_count = 0;
    kq_debug_frame = 0;
    kq_debug_last_assert = 0;
}

void debug_set_frame(u16 frame)
{
    kq_debug_frame = frame;
}

static void debug_push(const u8* name, u16 value)
{
    DebugTraceEntry* entry;
    if (kq_debug_trace_count >= DEBUG_TRACE_MAX) return;
    entry = &kq_debug_trace_log[(__safe_index u8)kq_debug_trace_count];
    entry->name = name;
    entry->value = value;
    entry->frame = kq_debug_frame;
    kq_debug_trace_count++;
}

void debug_mark_frame(const u8* label)
{
    debug_push(label, 0xFFFF);
}

void debug_trace_u8(const u8* name, u8 value)
{
    debug_push(name, (u16)value);
}

void debug_trace_u16(const u8* name, u16 value)
{
    debug_push(name, value);
}

void debug_assert_fail(u16 code)
{
    kq_debug_last_assert = code;
    debug_push((const u8*)"ASSERT", code);
}

u8 debug_get_trace_count()
{
    return kq_debug_trace_count;
}

const DebugTraceEntry* debug_get_trace_log()
{
    return kq_debug_trace_log;
}

u16 debug_get_last_assert()
{
    return kq_debug_last_assert;
}
