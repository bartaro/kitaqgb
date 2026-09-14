#pragma once

// Use a capacity from 1 through 255; the length counter is a byte.
// Compile every unit with the same override so the array declaration and storage agree.
#ifndef DEBUG_TRACE_MAX
#define DEBUG_TRACE_MAX 32
#endif

// Each packed entry retains a name pointer and two 16-bit values; name text is not copied.
typedef __packed struct {
    const u8* name;
    u16 value;
    u16 frame;
} DebugTraceEntry;

extern DebugTraceEntry kq_debug_trace_log[DEBUG_TRACE_MAX];
extern u8 kq_debug_trace_count;
extern u16 kq_debug_frame;
extern u16 kq_debug_last_assert;

// Reset log length, frame tag and last assertion code; existing entry bytes are not erased.
void debug_init();
// Set the frame tag copied into subsequently recorded entries.
void debug_set_frame(u16 frame);
// Append a named frame marker using 0xFFFF as its value.
void debug_mark_frame(const u8* label);
// Append an unsigned byte value, widened to the log's 16-bit value field.
void debug_trace_u8(const u8* name, u8 value);
// Append a 16-bit value with the current frame tag.
void debug_trace_u16(const u8* name, u16 value);
// Record the assertion code and attempt to append an ASSERT entry. This helper
// does not halt execution, and the code is retained even if the log is full.
void debug_assert_fail(u16 code);
// Return the number of valid entries in the fixed trace buffer.
u8 debug_get_trace_count();
// Expose the internal trace array for inspection; use the count to bound reads.
const DebugTraceEntry* debug_get_trace_log();
// Read the last recorded assertion code, or the zero value installed by initialization.
u16 debug_get_last_assert();

#define KITAQGB_TRACE_U8(name, value) debug_trace_u8((const u8*)(name), (u8)(value))
#define KITAQGB_TRACE_U16(name, value) debug_trace_u16((const u8*)(name), (u16)(value))
#define KITAQGB_TRACE(name, value) debug_trace_u16((const u8*)(name), (u16)(value))
#define KITAQGB_MARK_FRAME(label) debug_mark_frame((const u8*)(label))
#define KITAQGB_ASSERT(cond) do { if (!(cond)) debug_assert_fail((u16)1); } while (0)
#define KITAQGB_ASSERT_CODE(cond, code) do { if (!(cond)) debug_assert_fail((u16)(code)); } while (0)
