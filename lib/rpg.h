#pragma once

// The application supplies frame/cursor/font tiles at these indices; these constants do not load graphics.
#ifndef TEXT_TILE_FILL
#define TEXT_TILE_FILL ((u8)0)
#endif
#ifndef TEXT_TILE_FRAME_TL
#define TEXT_TILE_FRAME_TL ((u8)1)
#endif
#ifndef TEXT_TILE_FRAME_TR
#define TEXT_TILE_FRAME_TR ((u8)2)
#endif
#ifndef TEXT_TILE_FRAME_BL
#define TEXT_TILE_FRAME_BL ((u8)3)
#endif
#ifndef TEXT_TILE_FRAME_BR
#define TEXT_TILE_FRAME_BR ((u8)4)
#endif
#ifndef TEXT_TILE_FRAME_H
#define TEXT_TILE_FRAME_H ((u8)5)
#endif
#ifndef TEXT_TILE_FRAME_V
#define TEXT_TILE_FRAME_V ((u8)6)
#endif
#ifndef TEXT_TILE_CURSOR
#define TEXT_TILE_CURSOR ((u8)7)
#endif
#ifndef TEXT_TILE_PAGE_WAIT
#define TEXT_TILE_PAGE_WAIT ((u8)8)
#endif
#ifndef TEXT_DIGIT_BASE
#define TEXT_DIGIT_BASE ((u8)'0')
#endif

#define TEXT_CTRL_END       ((u8)0x00)
#define TEXT_CTRL_NEWLINE   ((u8)0x0A)
#define TEXT_CTRL_WAIT      ((u8)0x01)
#define TEXT_CTRL_PAGE_WAIT ((u8)0x02)

#define PAD_KEY_RIGHT  ((u8)0x01)
#define PAD_KEY_LEFT   ((u8)0x02)
#define PAD_KEY_UP     ((u8)0x04)
#define PAD_KEY_DOWN   ((u8)0x08)
#define PAD_KEY_A      ((u8)0x10)
#define PAD_KEY_B      ((u8)0x20)
#define PAD_KEY_SELECT ((u8)0x40)
#define PAD_KEY_START  ((u8)0x80)

// Default layout: four 512-byte slots, each with a six-byte header and at most 506 payload bytes.
#define SAVE_SLOT_COUNT ((u8)4)
#define SAVE_SLOT_SIZE  ((u16)512)
#define SAVE_MAGIC0     ((u8)0x4B)
#define SAVE_MAGIC1     ((u8)0x51)
#define SAVE_VERSION    ((u8)1)

#define SAVE_OK         ((u8)1)
#define SAVE_ERR        ((u8)0)

// A bank byte plus a CPU-visible address forms this data reference; it does not switch banks by itself.
typedef __packed struct {
    u8 bank;
    const u8* ptr;
} far_ptr_t;

typedef __packed struct {
    u8 x;
    u8 y;
    u8 w;
    u8 h;
    const u8* const* items;
    u8 count;
    u8 cursor;
} menu_t;

typedef __packed struct {
    u8 id;
    u8 qty;
    const u8* name;
} item_t;

// Packed nine-byte ordinary-map descriptor. Tile IDs use bank_tiles; collision bits and event bytes
// use the descriptor bank. Width/height are tile cells, with row-major data.
typedef __packed struct {
    u8 width;
    u8 height;
    u8 bank_tiles;
    const u8* tiles;
    const u8* collision_bits;
    const void* events;
} map_t;

// One 2x2 metatile: four tile IDs plus collision, palette metadata and event ID.
// The map drawing routine uses tile IDs only; it does not apply palette attributes.
typedef __packed struct {
    u8 tile_tl;
    u8 tile_tr;
    u8 tile_bl;
    u8 tile_br;
    u8 collision;
    u8 palette;
    u8 event;
} metatile_t;

// Metatile dimensions count 2x2 cells. bank_tiles=0 means use the descriptor bank;
// nonzero bank_tiles supplies both ID-array and definition-table reads.
typedef __packed struct {
    u8 width;
    u8 height;
    u8 bank_tiles;
    const u8* metatile_ids;
    const metatile_t* metatiles;
} metatile_map_t;

typedef __packed struct {
    u8 x;
    u8 y;
    u8 w;
    u8 h;
    const u8* const* items;
    u8 count;
    u8 cursor;
    u8 prev_keys;
    u8 selected;
    u8 cancelled;
} menu_state_t;

typedef __packed struct {
    u8 x;
    u8 y;
    u8 hp;
    u8 move;
    u8 atk_min;
    u8 atk_max;
    u8 team;
    u8 acted;
} unit_t;

void __wait_vblank();
u16 __readpadex(u8 prev_keys);
u16 __getbgmapbase();
void __memset(void* dst, u8 value, u16 count);
void __memcpy(void* dst, const void* src, u16 count);
void __settile(u8 x, u8 y, u8 tile);
void __settilebg(u8 x, u8 y, u8 tile);
void __settileat(u16 base, u8 x, u8 y, u8 tile);
void __vram_memcpy(u16 dst, const void* src, u16 len);
void __vram_memset(u16 dst, u8 value, u16 len);
void __far_memcpy(void* dst, u8 bank, const void* src, u16 len);
u8 __rng8();
void __rng_seed(u16 seed);
void __scroll_bg_set(u8 scx, u8 scy);

void __settile_xy(u8 x, u8 y, u8 tile);
void __settile_rect(u8 x, u8 y, u8 w, u8 h, u8 tile);
void __settile_row(u8 x, u8 y, const u8* src, u8 len);
void __settile_col(u8 x, u8 y, const u8* src, u8 len);
void __fill_tilemap(u16 base, u8 tile, u16 count);
void __settilemap_rect(u16 base, u8 x, u8 y, u8 w, u8 h, const u8* src);

void __vram_copy(u16 dst, const void* src, u16 len);
void __vram_fill(u16 dst, u8 value, u16 len);
void __vram_copy_hblank(u16 dst, const void* src, u16 len);
void __vram_copy_dma(u16 dst, const void* src, u16 len);

u8 __farpeek8(u8 bank, const void* addr);
u16 __farpeek16(u8 bank, const void* addr);
void __farmemcpy(void* dst, u8 bank, const void* src, u16 len);
// The current GB backend evaluates/discards these arguments but emits no indirect call.
void __farcall_ptr(u8 bank, const void* func);

#define far_read8(p) __farpeek8((p).bank, (p).ptr)
#define far_read16(p) __farpeek16((p).bank, (p).ptr)
#define far_read_block(dst, p, len) __farmemcpy((dst), (p).bank, (p).ptr, (len))

void __memcpy_small(void* dst, const void* src, u8 len);
void __memset_small(void* dst, u8 value, u8 len);
void __copy16(void* dst, const void* src);
void __copy32(void* dst, const void* src);

u16 __tile_addr(u16 base, u8 x, u8 y);
u16 __map_index(u8 x, u8 y, u8 width);
u8 __xy_in_rect(u8 x, u8 y, u8 rx, u8 ry, u8 rw, u8 rh);
u8 __manhattan(u8 x1, u8 y1, u8 x2, u8 y2);

u8 __bit_test(const u8* p, u16 bit_index);
void __bit_set(u8* p, u16 bit_index);
void __bit_clear(u8* p, u16 bit_index);
void __bit_toggle(u8* p, u16 bit_index);

// Compiler intrinsic, separate from the RAM rle_decode helpers below; it is not a bounded-stream API.
u16 __rle_decode_vram(u16 dst, u8 bank, const void* src);

// Advance the compiler-provided pseudorandom generator and return one byte.
u8 rng8();
// Consume two generator bytes in high-byte/low-byte order to form a 16-bit value.
u16 rng16();
// Reduce a random byte modulo max; zero returns zero without consuming randomness.
// Modulo reduction is biased unless max divides the generator range.
u8 rand_range(u8 max);
// Select an index from cumulative byte weights using a 16-bit random draw.
// A zero total, including an empty list, returns zero; callers must distinguish
// that case from a valid index. Modulo reduction can introduce a small bias.
u8 weighted_choice(const u8* weights, u8 count);
// Replace the generator seed so subsequent calls can reproduce a sequence.
void rng_seed(u16 seed);
// Alias the byte generator; this consumes one generator step.
u8 rng_next8();
// Alias the two-step 16-bit generator.
u16 rng_next16();
// Alias rand_range and return a value in [0,max), or zero when max is zero.
u8 rng_range(u8 max);
// Return false for zero and true for values at least 100. Intermediate values
// use the modulo-based range generator, so percentages are approximate.
u8 rng_chance(u8 percent);

// Set the tile-space window, reset its cursor and redraw it immediately.
// Opening does not reset the existing character speed or load font tiles.
void text_open(u8 x, u8 y, u8 w, u8 h);
// Erase the open window with the fill tile and disable windowed character output.
void text_close();
// Decode an END-terminated stream of tile bytes and newline/wait/page controls.
// WAIT consumes a following frame-count byte. The source must remain readable
// through blocking waits; the routine does not validate stream length.
void text_print(const u8* str);
// Decode the same text controls while fetching each byte from the specified
// ROM bank. The pointer advances within that bank; it does not roll into the next bank.
void text_print_far(u8 bank, const u8* str);
// Redraw choices and block for edge-triggered navigation: UP/DOWN wrap, A
// accepts, and B returns 0xFF. Zero choices also returns 0xFF. Choice strings
// use raw tile bytes, and the caller must fit them inside the available rows.
u8 text_choice(const u8* const* choices, u8 count);
// Set VBlank waits per windowed character; zero prints without character delays.
void text_set_speed(u8 speed);
// Open and redraw a window using the same tile-space arguments as text_open.
void text_window(u8 x, u8 y, u8 w, u8 h);
// Fill the supplied tile rectangle without changing the active text window or cursor.
void text_clear_rect(u8 x, u8 y, u8 w, u8 h);
// Draw a zero-terminated tile string at absolute tile coordinates, without
// window clipping, control codes or the windowed printing delay.
void text_print_xy(u8 x, u8 y, const u8* str);
// Print an unsigned byte in decimal using only the required number of cells.
void text_print_u8(u8 x, u8 y, u8 value);
// Print an unsigned word in decimal using only the required number of cells.
void text_print_u16(u8 x, u8 y, u16 value);
// Print a minus tile for negative values, followed by the unsigned magnitude.
// Positive values use the unsigned decimal path; no plus sign is emitted.
void text_print_s16(u8 x, u8 y, s16 value);

// Replace the active script and clear wait, choice and saved-return state.
void script_run(u8 bank, const u8* script);
// Execute commands until an output/choice yields (0), a wait yields (1), or
// execution ends (2). Control/flag commands continue within the same call, so
// a script loop without a yielding command can block the game loop indefinitely.
u8 script_step();
// Set the number of subsequent script_step calls to spend waiting. Frame-based
// timing requires the application to call script_step once per frame.
void script_wait_frames(u8 n);

// Run a blocking menu loop with wraparound up/down selection, A to accept
// and B to cancel. Return 0xFF for an empty menu or cancellation. Item strings
// and menu dimensions must fit the visible tile area.
u8 menu_run(const menu_t* menu);
// Open a two-choice dialog and return the text-choice result (YES first, NO second).
u8 menu_yesno();
// Run a blocking item menu, drawing names and two-digit quantities. Return
// the selected index or 0xFF on empty input/B; this does not consume an item.
u8 menu_inventory(const item_t* items, u8 count);
// Initialize a nonblocking menu state that borrows the item pointer table.
// Selection starts unset (0xFF), and the cancellation flag starts clear.
void menu_init_state(menu_state_t* state, u8 x, u8 y, u8 w, u8 h, const u8* const* items, u8 count);
// Draw the current menu and cursor without waiting or sampling input. The
// caller schedules suitable display timing and keeps item text within bounds.
void menu_draw(menu_state_t* state);
// Sample new button presses and update cursor/selection state without drawing.
// Selected and cancelled results remain latched; a B press in the same update
// as A clears selection and leaves cancellation set.
void menu_update(menu_state_t* state);
// Read the latched selection without clearing it; null state returns 0xFF.
u8 menu_get_selected(menu_state_t* state);
// Read the latched cancellation flag; null state returns false.
u8 menu_was_cancelled(menu_state_t* state);

// Load the nine-byte ordinary-map descriptor and draw up to 20 by 18 tiles
// from its top-left corner. Drawing does not upload tile graphics or clear
// background cells beyond smaller map dimensions.
void map_load(u8 bank, const map_t* map);
// Use metatile collision when that mode is active; otherwise read one packed
// collision bit. Missing maps/out-of-range cells block movement, but a valid
// ordinary map without a collision array is entirely passable.
u8 map_is_blocked(u8 x, u8 y);
// Report whether an event exists at the cell. Ordinary maps expose a Boolean
// byte-presence test; metatile maps reduce their event ID to a Boolean here.
u8 map_trigger_at(u8 x, u8 y);
// Center an ordinary-map tile coordinate within a 20-by-18 viewport and clamp
// the scroll origin to its edges. This adjusts scroll registers only; it does
// not stream newly exposed map rows into VRAM.
void camera_center(u8 x, u8 y);
// Select metatile mode, copy its descriptor and draw the initial viewport
// with the camera at cell (0,0).
void map_load_metatile(u8 bank, const metatile_map_t* map);
// Draw a 10-by-9 metatile viewport as 20-by-18 individual background tiles.
// Out-of-range cells use definition zero; palette attributes are not written.
// Remember the camera and clear the dirty flag after the full redraw.
void map_draw_visible(u8 camera_tx, u8 camera_ty);
// Read a row-major metatile ID, returning ID zero for inactive or out-of-range
// lookups. Callers must provide a usable definition for metatile zero.
u8 map_get_metatile(u8 tx, u8 ty);
// Return the selected definition's palette byte. An out-of-range coordinate
// uses the metatile-zero fallback; this function does not write CGB attributes.
u8 map_get_attr(u8 tx, u8 ty);
// Treat out-of-range metatile cells as solid; otherwise inspect the definition
// collision flag. In ordinary-map mode, delegate to its packed collision bits.
u8 map_is_solid(u8 tx, u8 ty);
// Return a metatile event ID, or the ordinary-map Boolean trigger result.
// Missing/out-of-range metatile cells have no event.
u8 map_get_event(u8 tx, u8 ty);
// Mark the viewport for a full redraw when a valid metatile cell changes.
// The coordinate is validated but no per-cell dirty list is retained.
void map_mark_dirty(u8 tx, u8 ty);
// Redraw the remembered metatile viewport only when the shared dirty flag is set.
void map_flush_dirty();

// Validate slot and payload length, write payload bytes, then write the header
// and checksum. Return one after completing the writes; interruption can leave
// a partially replaced slot, so this is not transactional save storage.
u8 save_write(u8 slot, const void* data, u16 len);
// Require matching magic/version, exact requested length and a valid checksum
// before copying data into the destination. Validation failure leaves the
// destination unchanged. An invalid slot returns before touching RAM control;
// validation failures after enabling RAM disable it before returning zero.
u8 save_read(u8 slot, void* data, u16 len);
// Validate a slot's header, payload bounds and checksum without copying the
// payload out. A nonzero result means the stored record passes these checks.
u8 save_exists(u8 slot);
// Initialize the expected RAM access state and leave cartridge RAM disabled.
// Existing slot contents are preserved.
void save_init();
// Alias the checked save_read operation.
u8 save_load(u8 slot, void* data, u16 len);
// Alias save_exists, including its header and checksum validation.
u8 save_check(u8 slot);
// Zero the complete selected slot, including header and payload; invalid slot
// IDs are ignored. Cartridge RAM is disabled when clearing finishes.
void save_clear(u8 slot);

// Build movement costs, then convert reachable cells to one and unreachable
// 0xFF cells to zero. The caller provides one output byte per current-map cell
// and a valid unit origin; path-search workspace limits also apply.
u8 unit_move_range(unit_t* u, u8* out_mask);
// Mark cells whose Manhattan distance falls within the inclusive attack range.
// This geometric mask does not test obstacles, line of sight or other units.
u8 unit_attack_range(unit_t* u, u8* out_mask);
// A unit can act only while HP is nonzero and its acted flag is clear.
// The unit pointer must refer to a valid record.
u8 unit_can_act(const unit_t* u);

// Find a shortest cardinal path with BFS and write direction codes 0=up,
// 1=right, 2=down, 3=left. Return its byte length, or zero for failure or an
// already-reached goal. The caller supplies max_len output bytes and respects
// the shared workspace/distance limits; excessive paths are not truncated.
u8 path_find_bfs(u8 sx, u8 sy, u8 gx, u8 gy, u8* out_path, u8 max_len);
// Flood cardinal neighbors at unit cost, stopping expansion at the movement
// limit and rejecting blocked destinations. Initialize output to 0xFF only
// after validating the map and origin; invalid input leaves output unchanged.
void range_fill_move(u8 sx, u8 sy, u8 move, u8* out_costmap);

// Read a bit from the flag array. Callers must keep id below 2048.
u8 flag_get(u16 id);
// Set a flag bit without disturbing adjacent flags; id must be below 2048.
void flag_set(u16 id);
// Clear a flag bit; id must be below 2048.
void flag_clear(u16 id);
// Read a quest-state byte; quest_id must be below 64.
u8 quest_state(u8 quest_id);
// Replace a quest-state byte without validating its game-specific meaning.
// The caller must keep quest_id below 64.
void quest_set_state(u8 quest_id, u8 state);

// Expand count/value byte pairs until a zero count terminates the stream.
// The caller must provide a valid terminator and sufficient destination storage;
// there are no source or destination bounds parameters. Return the 16-bit byte count.
u16 rle_decode(void* dst, const void* src);
// Decode the same count/value format using bank-qualified reads for every source
// byte. Keep the encoded stream within the supplied bank/address mapping and
// provide enough destination space; a zero count ends the stream.
u16 rle_decode_far(void* dst, u8 bank, const void* src);

// Return the active map width in its own cell units, or zero when no map is loaded.
u8 map_current_width();
// Return the active map height in its own cell units, or zero when no map is loaded.
u8 map_current_height();
