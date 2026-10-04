#pragma once

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

#define SAVE_SLOT_COUNT ((u8)4)
#define SAVE_SLOT_SIZE  ((u16)512)
#define SAVE_MAGIC0     ((u8)0x4B)
#define SAVE_MAGIC1     ((u8)0x51)
#define SAVE_VERSION    ((u8)1)

#define SAVE_OK         ((u8)1)
#define SAVE_ERR        ((u8)0)

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

typedef __packed struct {
    u8 width;
    u8 height;
    u8 bank_tiles;
    const u8* tiles;
    const u8* collision_bits;
    const void* events;
} map_t;

typedef __packed struct {
    u8 tile_tl;
    u8 tile_tr;
    u8 tile_bl;
    u8 tile_br;
    u8 collision;
    u8 palette;
    u8 event;
} metatile_t;

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

u16 __rle_decode_vram(u16 dst, u8 bank, const void* src);

u8 rng8();
u16 rng16();
u8 rand_range(u8 max);
u8 weighted_choice(const u8* weights, u8 count);
void rng_seed(u16 seed);
u8 rng_next8();
u16 rng_next16();
u8 rng_range(u8 max);
u8 rng_chance(u8 percent);

void text_open(u8 x, u8 y, u8 w, u8 h);
void text_close();
void text_print(const u8* str);
void text_print_far(u8 bank, const u8* str);
u8 text_choice(const u8* const* choices, u8 count);
void text_set_speed(u8 speed);
void text_window(u8 x, u8 y, u8 w, u8 h);
void text_clear_rect(u8 x, u8 y, u8 w, u8 h);
void text_print_xy(u8 x, u8 y, const u8* str);
void text_print_u8(u8 x, u8 y, u8 value);
void text_print_u16(u8 x, u8 y, u16 value);
void text_print_s16(u8 x, u8 y, s16 value);

void script_run(u8 bank, const u8* script);
u8 script_step();
void script_wait_frames(u8 n);

u8 menu_run(const menu_t* menu);
u8 menu_yesno();
u8 menu_inventory(const item_t* items, u8 count);
void menu_init_state(menu_state_t* state, u8 x, u8 y, u8 w, u8 h, const u8* const* items, u8 count);
void menu_draw(menu_state_t* state);
void menu_update(menu_state_t* state);
u8 menu_get_selected(menu_state_t* state);
u8 menu_was_cancelled(menu_state_t* state);

void map_load(u8 bank, const map_t* map);
u8 map_is_blocked(u8 x, u8 y);
u8 map_trigger_at(u8 x, u8 y);
void camera_center(u8 x, u8 y);
void map_load_metatile(u8 bank, const metatile_map_t* map);
void map_draw_visible(u8 camera_tx, u8 camera_ty);
u8 map_get_metatile(u8 tx, u8 ty);
u8 map_get_attr(u8 tx, u8 ty);
u8 map_is_solid(u8 tx, u8 ty);
u8 map_get_event(u8 tx, u8 ty);
void map_mark_dirty(u8 tx, u8 ty);
void map_flush_dirty();

u8 save_write(u8 slot, const void* data, u16 len);
u8 save_read(u8 slot, void* data, u16 len);
u8 save_exists(u8 slot);
void save_init();
u8 save_load(u8 slot, void* data, u16 len);
u8 save_check(u8 slot);
void save_clear(u8 slot);

u8 unit_move_range(unit_t* u, u8* out_mask);
u8 unit_attack_range(unit_t* u, u8* out_mask);
u8 unit_can_act(const unit_t* u);

u8 path_find_bfs(u8 sx, u8 sy, u8 gx, u8 gy, u8* out_path, u8 max_len);
void range_fill_move(u8 sx, u8 sy, u8 move, u8* out_costmap);

u8 flag_get(u16 id);
void flag_set(u16 id);
void flag_clear(u16 id);
u8 quest_state(u8 quest_id);
void quest_set_state(u8 quest_id, u8 state);

u16 rle_decode(void* dst, const void* src);
u16 rle_decode_far(void* dst, u8 bank, const void* src);

u8 map_current_width();
u8 map_current_height();
