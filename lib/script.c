#include "rpg.h"

// Compact banked script opcodes. Multi-byte operands use the compiler far-read
// helpers; the stream must be valid and terminated because no length is stored.
// 00: no operands; finish the script or resume the one saved caller.
#define SCRIPT_OP_END        ((u8)0x00)
// 01: bank:u8, address:u16 little-endian; print the banked text stream, then yield.
#define SCRIPT_OP_TEXT       ((u8)0x01)
// 02: flag ID:u16 little-endian; IDs must be below 2048.
#define SCRIPT_OP_SET_FLAG   ((u8)0x02)
// 03: flag ID:u16 little-endian; clear the selected flag and continue.
#define SCRIPT_OP_CLEAR_FLAG ((u8)0x03)
// 04: bank:u8, address:u16; see the handler caveat: the address is read after switching the script bank.
#define SCRIPT_OP_JUMP       ((u8)0x04)
// 05: flag ID:u16, bank:u8, address:u16; read all operands before an optional branch.
#define SCRIPT_OP_IF_FLAG    ((u8)0x05)
// 06: count:u8, table address:u16; pointers and strings use the current script bank. Count must be nonzero.
#define SCRIPT_OP_CHOICE     ((u8)0x06)
// 07: count:u8; yield now, then spend count subsequent script_step calls waiting.
#define SCRIPT_OP_WAIT       ((u8)0x07)
// 08: bank:u8, address:u16; transfer with one shared return slot, not a nesting stack.
#define SCRIPT_OP_CALL_EVENT ((u8)0x08)

// Single shared interpreter state with one saved return location, not a call
// stack. Script execution and blocking choice dialogs are not reentrant.
static u8 script_active;
static u8 script_bank;
static const u8* script_pc;
static u8 script_wait;
static u8 script_last_choice;
static u8 script_return_active;
static u8 script_return_bank;
static const u8* script_return_pc;

// Read one byte from the current script bank/address and advance the address.
// Address increments do not select the next ROM bank automatically; keep operands within the mapped source.
static u8 script_read8()
{
    u8 v = __farpeek8(script_bank, script_pc);
    script_pc++;
    return v;
}

// Read a 16-bit operand from the current bank and advance past both bytes.
static u16 script_read16()
{
    u16 v = __farpeek16(script_bank, script_pc);
    script_pc = (const u8*)(script_pc + 2);
    return v;
}

// Render a zero-terminated banked string along one tile row. The caller must
// provide a terminator and keep text within the intended visible row.
static void script_draw_far_string(u8 bank, const u8* str, u8 x, u8 y)
{
    const u8* p = str;
    u8 px = x;
    while (1) {
        u8 c = __farpeek8(bank, p);
        p++;
        if (c == 0) return;
        __settile_xy(px, y, c);
        px++;
    }
}

// Run a blocking choice loop over a banked table of 16-bit string pointers.
// A accepts the cursor and B returns 0xFF; callers must provide at least one item.
static u8 script_choice(u8 bank, const u8* table_ptr, u8 count)
{
    u8 cursor = 0;
    u8 prev = 0;

    while (1) {
        u8 i;
        for (i = 0; i < count; i++) {
            u16 ptr = __farpeek16(bank, table_ptr + ((u16)i << 1));
            u8 py = (u8)(2 + i);
            __settile_xy(2, py, (i == cursor) ? TEXT_TILE_CURSOR : TEXT_TILE_FILL);
            script_draw_far_string(bank, (const u8*)ptr, 3, py);
        }

        __wait_vblank();
        {
            u16 kt = __readpadex(prev);
            u8 trigger;
            prev = (u8)kt;
            trigger = (u8)(kt >> 8);

            if ((trigger & PAD_KEY_UP) != 0) {
                if (cursor == 0) cursor = (u8)(count - 1);
                else cursor--;
            }
            if ((trigger & PAD_KEY_DOWN) != 0) {
                cursor++;
                if (cursor >= count) cursor = 0;
            }
            if ((trigger & PAD_KEY_A) != 0) return cursor;
            if ((trigger & PAD_KEY_B) != 0) return 0xFF;
        }
    }
}

// Replace the active script and clear wait, choice and saved-return state.
void script_run(u8 bank, const u8* script)
{
    script_active = 1;
    script_bank = bank;
    script_pc = script;
    script_wait = 0;
    script_last_choice = 0xFF;
    script_return_active = 0;
}

// Set the number of subsequent script_step calls to spend waiting. Frame-based
// timing requires the application to call script_step once per frame.
void script_wait_frames(u8 n)
{
    script_wait = n;
}

// Execute commands until an output/choice yields (0), a wait yields (1), or
// execution ends (2). Control/flag commands continue within the same call, so
// a script loop without a yielding command can block the game loop indefinitely.
u8 script_step()
{
    if (script_active == 0) return 2;
    if (script_wait != 0) {
        script_wait--;
        return 1;
    }

    while (script_active != 0) {
        u8 op = script_read8();

        // An END returns to the one saved caller when present; otherwise it ends the script.
        if (op == SCRIPT_OP_END) {
            if (script_return_active != 0) {
                script_bank = script_return_bank;
                script_pc = script_return_pc;
                script_return_active = 0;
                continue;
            }
            script_active = 0;
            return 2;
        }

        if (op == SCRIPT_OP_TEXT) {
            u8 bank = script_read8();
            u16 ptr = script_read16();
            text_print_far(bank, (const u8*)ptr);
            return 0;
        }

        if (op == SCRIPT_OP_SET_FLAG) {
            u16 id = script_read16();
            flag_set(id);
            continue;
        }

        if (op == SCRIPT_OP_CLEAR_FLAG) {
            u16 id = script_read16();
            flag_clear(id);
            continue;
        }

        // This implementation installs the destination bank before reading the address
        // operand. That read therefore uses the new bank at the current operand address;
        // cross-bank script authors must account for this ordering.
        if (op == SCRIPT_OP_JUMP) {
            script_bank = script_read8();
            script_pc = (const u8*)script_read16();
            continue;
        }

        // Read all branch operands before switching bank/address so an untaken branch
        // still consumes the full instruction.
        if (op == SCRIPT_OP_IF_FLAG) {
            u16 id = script_read16();
            u8 bank = script_read8();
            u16 ptr = script_read16();
            if (flag_get(id) != 0) {
                script_bank = bank;
                script_pc = (const u8*)ptr;
            }
            continue;
        }

        if (op == SCRIPT_OP_CHOICE) {
            u8 count = script_read8();
            u16 table = script_read16();
            script_last_choice = script_choice(script_bank, (const u8*)table, count);
            return 0;
        }

        if (op == SCRIPT_OP_WAIT) {
            script_wait = script_read8();
            return 1;
        }

        // Save the continuation only when the single return slot is empty. Nested
        // calls replace the current script but do not push another return address.
        if (op == SCRIPT_OP_CALL_EVENT) {
            u8 bank = script_read8();
            u16 ptr = script_read16();
            if (script_return_active == 0) {
                script_return_active = 1;
                script_return_bank = script_bank;
                script_return_pc = script_pc;
            }
            script_bank = bank;
            script_pc = (const u8*)ptr;
            continue;
        }

        // Unknown opcodes terminate with the same return code as END; there is no distinct malformed-stream error.
        script_active = 0;
        return 2;
    }

    return 2;
}
