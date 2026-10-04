#include "slg.h"

static u16 slg_index(const SLGBoard* board, u8 x, u8 y)
{
    return (u16)((u16)y * (u16)board->width + (u16)x);
}

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

void slg_board_init(SLGBoard* board, u8 width, u8 height, u8* cells)
{
    board->width = width;
    board->height = height;
    board->cells = cells;
}

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

u8 slg_board_get(const SLGBoard* board, u8 x, u8 y)
{
    u8* p;
    if (board == 0) return 0;
    if (x >= board->width || y >= board->height) return 0;
    p = slg_cell_ptr((SLGBoard*)board, x, y);
    return *p;
}

void slg_board_set(SLGBoard* board, u8 x, u8 y, u8 value)
{
    u8* p;
    if (board == 0) return;
    if (x >= board->width || y >= board->height) return;
    p = slg_cell_ptr(board, x, y);
    *p = value;
}

void slg_move_list_init(SLGMoveList* list, SLGMove* items, u8 capacity)
{
    list->items = items;
    list->capacity = capacity;
    list->count = 0;
}

void slg_move_list_clear(SLGMoveList* list)
{
    if (list == 0) return;
    list->count = 0;
}

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

void slg_undo_init(SLGUndoStack* stack, SLGMove* items, u8 capacity)
{
    stack->items = items;
    stack->capacity = capacity;
    stack->count = 0;
}

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
