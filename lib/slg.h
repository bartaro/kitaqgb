#pragma once

// A row-major byte board. Storage is borrowed; cell values and coordinates have game-defined meaning.
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

// Undo records are retained cell values only; popping one does not apply it to a board.
typedef __packed struct {
    SLGMove* items;
    u8 count;
    u8 capacity;
} SLGUndoStack;

// Attach width * height bytes of caller-owned cell storage without clearing it.
void slg_board_init(SLGBoard* board, u8 width, u8 height, u8* cells);
// Fill every cell with value; a null board is ignored. A valid board must own
// enough accessible cell storage for its declared dimensions.
void slg_board_clear(SLGBoard* board, u8 value);
// Return zero for a null board or out-of-range coordinate; otherwise read the
// cell. Zero therefore cannot distinguish an invalid lookup from an empty cell.
u8 slg_board_get(const SLGBoard* board, u8 x, u8 y);
// Write a cell only when the board and coordinate are valid.
void slg_board_set(SLGBoard* board, u8 x, u8 y, u8 value);
// Attach caller-owned move storage and reset the list length.
void slg_move_list_init(SLGMoveList* list, SLGMove* items, u8 capacity);
// Forget the moves without erasing their storage; a null list is ignored.
void slg_move_list_clear(SLGMoveList* list);
// Append coordinates and a value to the bounded move list. Return zero on a
// null/full list without replacing any existing move.
u8 slg_move_list_push(SLGMoveList* list, u8 x, u8 y, u8 value);
// Attach caller-owned undo records and initialize an empty stack.
void slg_undo_init(SLGUndoStack* stack, SLGMove* items, u8 capacity);
// Record a cell's previous value for later undo. Return zero if the stack is
// null or full; this function does not change the board itself.
u8 slg_undo_push(SLGUndoStack* stack, u8 x, u8 y, u8 old_value);
// Remove the newest undo record and optionally copy it out. A null output
// still discards the record; a null or empty stack returns zero.
u8 slg_undo_pop(SLGUndoStack* stack, SLGMove* out_move);

