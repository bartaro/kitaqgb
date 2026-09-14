#include "rpg.h"

__prg_rom u8 menu_yes[] = { 'Y', 'E', 'S', 0 };
__prg_rom u8 menu_no[] = { 'N', 'O', 0 };

// Draw a two-digit quantity by repeated subtraction. Callers must keep value
// below 100 so the tens digit remains a decimal font tile.
static void menu_draw_ascii_qty(u8 x, u8 y, u8 value)
{
    u8 tens = 0;
    u8 ones = value;
    while (ones >= 10) {
        ones -= 10;
        tens++;
    }
    __settile_xy(x, y, (u8)(TEXT_DIGIT_BASE + tens));
    __settile_xy((u8)(x + 1), y, (u8)(TEXT_DIGIT_BASE + ones));
}

// Run a blocking menu loop with wraparound up/down selection, A to accept
// and B to cancel. Return 0xFF for an empty menu or cancellation. Item strings
// and menu dimensions must fit the visible tile area.
// Pass a non-null menu and readable item strings. Initial prev=0 treats a button already held on entry
// as a new press; A wins over B when both occur together in this blocking API.
u8 menu_run(const menu_t* menu)
{
    u8 cursor;
    u8 prev;

    if (menu->count == 0) return 0xFF;

    text_open(menu->x, menu->y, menu->w, menu->h);
    cursor = menu->cursor;
    if (cursor >= menu->count) cursor = 0;
    prev = 0;

    while (1) {
        u8 i;
        for (i = 0; i < menu->count; i++) {
            u8 py = (u8)(menu->y + 1 + i);
            __settile_xy((u8)(menu->x + 1), py, (i == cursor) ? TEXT_TILE_CURSOR : TEXT_TILE_FILL);
            {
                const u8* s = menu->items[i];
                u8 px = (u8)(menu->x + 2);
                while (*s != 0) {
                    __settile_xy(px, py, *s);
                    px++;
                    s++;
                }
            }
        }

        __wait_vblank();
        {
            u16 kt = __readpadex(prev);
            u8 trigger;
            prev = (u8)kt;
            trigger = (u8)(kt >> 8);

            if ((trigger & PAD_KEY_UP) != 0) {
                if (cursor == 0) cursor = (u8)(menu->count - 1);
                else cursor--;
            }
            if ((trigger & PAD_KEY_DOWN) != 0) {
                cursor++;
                if (cursor >= menu->count) cursor = 0;
            }
            if ((trigger & PAD_KEY_A) != 0) return cursor;
            if ((trigger & PAD_KEY_B) != 0) return 0xFF;
        }
    }
}

// Initialize a nonblocking menu state that borrows the item pointer table.
// Selection starts unset (0xFF), and the cancellation flag starts clear.
// Borrow both the item table and its strings; keep them visible in the active ROM mapping.
// Use non-null state storage and initialize again to clear latched completion flags.
void menu_init_state(menu_state_t* state, u8 x, u8 y, u8 w, u8 h, const u8* const* items, u8 count)
{
    state->x = x;
    state->y = y;
    state->w = w;
    state->h = h;
    state->items = items;
    state->count = count;
    state->cursor = 0;
    state->prev_keys = 0;
    state->selected = 0xFF;
    state->cancelled = 0;
}

// Draw the current menu and cursor without waiting or sampling input. The
// caller schedules suitable display timing and keeps item text within bounds.
void menu_draw(menu_state_t* state)
{
    u8 i;
    if (state == 0) return;

    text_open(state->x, state->y, state->w, state->h);
    i = 0;
    while (i < state->count) {
        u8 py = (u8)(state->y + 1 + i);
        const u8* s = state->items[i];
        u8 px = (u8)(state->x + 2);
        __settile_xy((u8)(state->x + 1), py, (i == state->cursor) ? TEXT_TILE_CURSOR : TEXT_TILE_FILL);
        while (*s != 0) {
            __settile_xy(px, py, *s);
            px++;
            s++;
        }
        i++;
    }
}

// Sample new button presses and update cursor/selection state without drawing.
// Selected and cancelled results remain latched; a B press in the same update
// as A clears selection and leaves cancellation set.
void menu_update(menu_state_t* state)
{
    u16 kt;
    u8 trigger;

    if (state == 0) return;
    if (state->count == 0) return;

    kt = __readpadex(state->prev_keys);
    state->prev_keys = (u8)kt;
    trigger = (u8)(kt >> 8);

    if ((trigger & PAD_KEY_UP) != 0) {
        if (state->cursor == 0) state->cursor = (u8)(state->count - 1);
        else state->cursor--;
    }
    if ((trigger & PAD_KEY_DOWN) != 0) {
        state->cursor++;
        if (state->cursor >= state->count) state->cursor = 0;
    }
    if ((trigger & PAD_KEY_A) != 0) {
        state->selected = state->cursor;
    }
    if ((trigger & PAD_KEY_B) != 0) {
        state->cancelled = 1;
        state->selected = 0xFF;
    }
}

// Read the latched selection without clearing it; null state returns 0xFF.
u8 menu_get_selected(menu_state_t* state)
{
    if (state == 0) return 0xFF;
    return state->selected;
}

// Read the latched cancellation flag; null state returns false.
u8 menu_was_cancelled(menu_state_t* state)
{
    if (state == 0) return 0;
    return state->cancelled;
}

// Open a two-choice dialog and return the text-choice result (YES first, NO second).
u8 menu_yesno()
{
    const u8* items[2];

    items[0] = menu_yes;
    items[1] = menu_no;
    text_open(6, 10, 8, 4);
    return text_choice(items, 2);
}

// Run a blocking item menu, drawing names and two-digit quantities. Return
// the selected index or 0xFF on empty input/B; this does not consume an item.
// Use at most 15 visible items for this fixed dialog placement; quantities must be below 100.
// Long names are not clipped and can overwrite the quantity column.
u8 menu_inventory(const item_t* items, u8 count)
{
    u8 cursor;
    u8 prev;
    u8 i;

    if (count == 0) return 0xFF;

    text_open(1, 1, 18, (u8)(count + 2));
    cursor = 0;
    prev = 0;

    while (1) {
        for (i = 0; i < count; i++) {
            u8 py = (u8)(2 + i);
            const u8* s = items[i].name;
            u8 px = 3;
            __settile_xy(2, py, (i == cursor) ? TEXT_TILE_CURSOR : TEXT_TILE_FILL);
            while (*s != 0) {
                __settile_xy(px, py, *s);
                px++;
                s++;
            }
            menu_draw_ascii_qty(15, py, items[i].qty);
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
