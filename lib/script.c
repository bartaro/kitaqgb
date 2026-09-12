#include "rpg.h"

#define SCRIPT_OP_END        ((u8)0x00)
#define SCRIPT_OP_TEXT       ((u8)0x01)
#define SCRIPT_OP_SET_FLAG   ((u8)0x02)
#define SCRIPT_OP_CLEAR_FLAG ((u8)0x03)
#define SCRIPT_OP_JUMP       ((u8)0x04)
#define SCRIPT_OP_IF_FLAG    ((u8)0x05)
#define SCRIPT_OP_CHOICE     ((u8)0x06)
#define SCRIPT_OP_WAIT       ((u8)0x07)
#define SCRIPT_OP_CALL_EVENT ((u8)0x08)

static u8 script_active;
static u8 script_bank;
static const u8* script_pc;
static u8 script_wait;
static u8 script_last_choice;
static u8 script_return_active;
static u8 script_return_bank;
static const u8* script_return_pc;

static u8 script_read8()
{
    u8 v = __farpeek8(script_bank, script_pc);
    script_pc++;
    return v;
}

static u16 script_read16()
{
    u16 v = __farpeek16(script_bank, script_pc);
    script_pc = (const u8*)(script_pc + 2);
    return v;
}

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

void script_run(u8 bank, const u8* script)
{
    script_active = 1;
    script_bank = bank;
    script_pc = script;
    script_wait = 0;
    script_last_choice = 0xFF;
    script_return_active = 0;
}

void script_wait_frames(u8 n)
{
    script_wait = n;
}

u8 script_step()
{
    if (script_active == 0) return 2;
    if (script_wait != 0) {
        script_wait--;
        return 1;
    }

    while (script_active != 0) {
        u8 op = script_read8();

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

        if (op == SCRIPT_OP_JUMP) {
            script_bank = script_read8();
            script_pc = (const u8*)script_read16();
            continue;
        }

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

        script_active = 0;
        return 2;
    }

    return 2;
}
