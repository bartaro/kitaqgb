#include "debug.h"

// Shared append-only trace state; serialize writers if main code and interrupts both log.
DebugTraceEntry kq_debug_trace_log[DEBUG_TRACE_MAX];
u8 kq_debug_trace_count;
u16 kq_debug_frame;
u16 kq_debug_last_assert;

// Reset log length, frame tag and last assertion code; existing entry bytes are not erased.
void debug_init()
{
    kq_debug_trace_count = 0;
    kq_debug_frame = 0;
    kq_debug_last_assert = 0;
}

// Set the frame tag copied into subsequently recorded entries.
void debug_set_frame(u16 frame)
{
    kq_debug_frame = frame;
}

// Append a trace entry until the fixed log is full, then drop further entries.
// The name pointer is retained; its string must remain readable while the log is used.
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

// Append a named frame marker using 0xFFFF as its value.
void debug_mark_frame(const u8* label)
{
    debug_push(label, 0xFFFF);
}

// Append an unsigned byte value, widened to the log's 16-bit value field.
void debug_trace_u8(const u8* name, u8 value)
{
    debug_push(name, (u16)value);
}

// Append a 16-bit value with the current frame tag.
void debug_trace_u16(const u8* name, u16 value)
{
    debug_push(name, value);
}

// Record the assertion code and attempt to append an ASSERT entry. This helper
// does not halt execution, and the code is retained even if the log is full.
void debug_assert_fail(u16 code)
{
    kq_debug_last_assert = code;
    debug_push((const u8*)"ASSERT", code);
}

// Return the number of valid entries in the fixed trace buffer.
u8 debug_get_trace_count()
{
    return kq_debug_trace_count;
}

// Expose the internal trace array for inspection; use the count to bound reads.
const DebugTraceEntry* debug_get_trace_log()
{
    return kq_debug_trace_log;
}

// Read the last recorded assertion code, or the zero value installed by initialization.
u16 debug_get_last_assert()
{
    return kq_debug_last_assert;
}
