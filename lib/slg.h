#pragma once

typedef __packed struct {
    u8 width;
    u8 height;
    u8* cells;
} SLGBoard;

typedef __packed struct {
    u8 x;
    u8 y;
    u8 value;
} SLGMove;

typedef __packed struct {
    SLGMove* items;
    u8 count;
    u8 capacity;
} SLGMoveList;

typedef __packed struct {
    SLGMove* items;
    u8 count;
    u8 capacity;
} SLGUndoStack;

void slg_board_init(SLGBoard* board, u8 width, u8 height, u8* cells);
void slg_board_clear(SLGBoard* board, u8 value);
u8 slg_board_get(const SLGBoard* board, u8 x, u8 y);
void slg_board_set(SLGBoard* board, u8 x, u8 y, u8 value);
void slg_move_list_init(SLGMoveList* list, SLGMove* items, u8 capacity);
void slg_move_list_clear(SLGMoveList* list);
u8 slg_move_list_push(SLGMoveList* list, u8 x, u8 y, u8 value);
void slg_undo_init(SLGUndoStack* stack, SLGMove* items, u8 capacity);
u8 slg_undo_push(SLGUndoStack* stack, u8 x, u8 y, u8 old_value);
u8 slg_undo_pop(SLGUndoStack* stack, SLGMove* out_move);

