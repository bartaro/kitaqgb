#include "slg.h"

// Convert tile coordinates to a row-major cell offset using the board width.
static u16 slg_index(const SLGBoard* board, u8 x, u8 y)
{
    return (u16)((u16)y * (u16)board->width + (u16)x);
}

// Walk to a previously validated cell using a 16-bit offset and byte pointer.
// Bounds checking belongs to the public get/set routines.
static u8* slg_cell_ptr(SLGBoard* board, u8 x, u8 y)
{
    u16 index;
    u8* p;

    index = slg_index(board, x, y);
    p = board->cells;
    while (index != 0) {
        p++;
        index--;
    }
    return p;
}

// Attach width * height bytes of caller-owned cell storage without clearing it.
// Initialization requires non-null board storage. Nonempty dimensions require width*height accessible bytes.
void slg_board_init(SLGBoard* board, u8 width, u8 height, u8* cells)
{
    board->width = width;
    board->height = height;
    board->cells = cells;
}

// Fill every cell with value; a null board is ignored. A valid board must own
// enough accessible cell storage for its declared dimensions.
void slg_board_clear(SLGBoard* board, u8 value)
{
    u16 total;
    u16 i;
    u8* p;
    if (board == 0) return;
    total = (u16)((u16)board->width * (u16)board->height);
    i = 0;
    p = board->cells;
    while (i < total) {
        *p = value;
        p++;
        i++;
    }
}

// Return zero for a null board or out-of-range coordinate; otherwise read the
// cell. Zero therefore cannot distinguish an invalid lookup from an empty cell.
u8 slg_board_get(const SLGBoard* board, u8 x, u8 y)
{
    u8* p;
    if (board == 0) return 0;
    if (x >= board->width || y >= board->height) return 0;
    p = slg_cell_ptr((SLGBoard*)board, x, y);
    return *p;
}

// Write a cell only when the board and coordinate are valid.
void slg_board_set(SLGBoard* board, u8 x, u8 y, u8 value)
{
    u8* p;
    if (board == 0) return;
    if (x >= board->width || y >= board->height) return;
    p = slg_cell_ptr(board, x, y);
    *p = value;
}

// Attach caller-owned move storage and reset the list length.
// Use non-null list storage and room for capacity records; no allocation is performed.
void slg_move_list_init(SLGMoveList* list, SLGMove* items, u8 capacity)
{
    list->items = items;
    list->capacity = capacity;
    list->count = 0;
}

// Forget the moves without erasing their storage; a null list is ignored.
void slg_move_list_clear(SLGMoveList* list)
{
    if (list == 0) return;
    list->count = 0;
}

// Append coordinates and a value to the bounded move list. Return zero on a
// null/full list without replacing any existing move.
u8 slg_move_list_push(SLGMoveList* list, u8 x, u8 y, u8 value)
{
    SLGMove* move;
    if (list == 0) return 0;
    if (list->count >= list->capacity) return 0;
    move = &list->items[(__safe_index u8)list->count];
    move->x = x;
    move->y = y;
    move->value = value;
    list->count++;
    return 1;
}

// Attach caller-owned undo records and initialize an empty stack.
// Use non-null stack storage and room for capacity records; entries are not cleared here.
void slg_undo_init(SLGUndoStack* stack, SLGMove* items, u8 capacity)
{
    stack->items = items;
    stack->capacity = capacity;
    stack->count = 0;
}

// Record a cell's previous value for later undo. Return zero if the stack is
// null or full; this function does not change the board itself.
u8 slg_undo_push(SLGUndoStack* stack, u8 x, u8 y, u8 old_value)
{
    if (stack == 0) return 0;
    if (stack->count >= stack->capacity) return 0;
    stack->items[stack->count].x = x;
    stack->items[stack->count].y = y;
    stack->items[stack->count].value = old_value;
    stack->count++;
    return 1;
}

// Remove the newest undo record and optionally copy it out. A null output
// still discards the record; a null or empty stack returns zero.
u8 slg_undo_pop(SLGUndoStack* stack, SLGMove* out_move)
{
    if (stack == 0) return 0;
    if (stack->count == 0) return 0;
    stack->count--;
    if (out_move != 0) {
        out_move->x = stack->items[stack->count].x;
        out_move->y = stack->items[stack->count].y;
        out_move->value = stack->items[stack->count].value;
    }
    return 1;
}
