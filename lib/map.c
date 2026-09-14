#include "rpg.h"

// One shared active map, either ordinary tiles or metatiles. Descriptor copies
// are retained in RAM while referenced assets are read through their ROM banks.
static map_t map_current;
static metatile_map_t map_meta_current;
static u8 map_bank;
static u8 map_meta_bank;
static u8 map_loaded;
static u8 map_meta_loaded;
static u8 map_camera_x;
static u8 map_camera_y;
static u8 map_row[32];
static u8 map_dirty;

// Return the active map width in its own cell units, or zero when no map is loaded.
u8 map_current_width()
{
    if (map_meta_loaded != 0) return map_meta_current.width;
    if (map_loaded == 0) return 0;
    return map_current.width;
}

// Return the active map height in its own cell units, or zero when no map is loaded.
u8 map_current_height()
{
    if (map_meta_loaded != 0) return map_meta_current.height;
    if (map_loaded == 0) return 0;
    return map_current.height;
}

// Load the nine-byte ordinary-map descriptor and draw up to 20 by 18 tiles
// from its top-left corner. Drawing does not upload tile graphics or clear
// background cells beyond smaller map dimensions.
void map_load(u8 bank, const map_t* map)
{
    u8 row;
    u8 h;
    u8 w;

    // The literal copy size depends on the packed map_t layout; changing that layout requires updating this copy.
    __farmemcpy(&map_current, bank, map, 9);
    map_bank = bank;
    map_loaded = 1;
    map_meta_loaded = 0;

    w = map_current.width;
    h = map_current.height;
    if (w > 20) w = 20;
    if (h > 18) h = 18;

    row = 0;
    while (row < h) {
        u8 x = 0;
        u16 base_index = __map_index(0, row, map_current.width);
        while (x < w) {
            map_row[x] = __farpeek8(map_current.bank_tiles, map_current.tiles + base_index + x);
            x++;
        }
        __settile_row(0, row, map_row, w);
        row++;
    }
}

// Use metatile collision when that mode is active; otherwise read one packed
// collision bit. Missing maps/out-of-range cells block movement, but a valid
// ordinary map without a collision array is entirely passable.
u8 map_is_blocked(u8 x, u8 y)
{
    u16 index;
    u8 value;

    if (map_meta_loaded != 0) return map_is_solid(x, y);
    if (map_loaded == 0) return 1;
    if (x >= map_current.width || y >= map_current.height) return 1;
    if (map_current.collision_bits == 0) return 0;

    index = __map_index(x, y, map_current.width);
    // Collision uses least-significant bit first within each row-major group of eight cells.
    value = __farpeek8(map_bank, map_current.collision_bits + (index >> 3));
    if ((value & (u8)(1 << (index & 7))) != 0) return 1;
    return 0;
}

// Report whether an event exists at the cell. Ordinary maps expose a Boolean
// byte-presence test; metatile maps reduce their event ID to a Boolean here.
u8 map_trigger_at(u8 x, u8 y)
{
    u16 index;
    u8 value;

    if (map_meta_loaded != 0) return (u8)(map_get_event(x, y) != 0);
    if (map_loaded == 0) return 0;
    if (x >= map_current.width || y >= map_current.height) return 0;
    if (map_current.events == 0) return 0;

    index = __map_index(x, y, map_current.width);
    value = __farpeek8(map_bank, ((const u8*)map_current.events) + index);
    return (u8)((value != 0) ? 1 : 0);
}

// Center an ordinary-map tile coordinate within a 20-by-18 viewport and clamp
// the scroll origin to its edges. This adjusts scroll registers only; it does
// not stream newly exposed map rows into VRAM.
void camera_center(u8 x, u8 y)
{
    u8 tile_x = 0;
    u8 tile_y = 0;
    u8 max_x = 0;
    u8 max_y = 0;

    map_camera_x = x;
    map_camera_y = y;

    if (map_loaded == 0) return;

    if (map_current.width > 20) max_x = (u8)(map_current.width - 20);
    if (map_current.height > 18) max_y = (u8)(map_current.height - 18);

    if (x > 10) tile_x = (u8)(x - 10);
    if (y > 9) tile_y = (u8)(y - 9);

    if (tile_x > max_x) tile_x = max_x;
    if (tile_y > max_y) tile_y = max_y;

    __scroll_bg_set((u8)(tile_x << 3), (u8)(tile_y << 3));
}

// Use the metatile data-bank override when nonzero; zero means the descriptor bank.
static u8 map_meta_data_bank()
{
    if (map_meta_current.bank_tiles != 0) return map_meta_current.bank_tiles;
    return map_meta_bank;
}

// Copy one metatile definition from banked ROM. The ID and definition-table
// length must be valid; this internal helper has no range information.
static void map_read_metatile(u8 id, metatile_t* out)
{
    __farmemcpy(out, map_meta_data_bank(), map_meta_current.metatiles + id, sizeof(metatile_t));
}

// Select metatile mode, copy its descriptor and draw the initial viewport
// with the camera at cell (0,0).
void map_load_metatile(u8 bank, const metatile_map_t* map)
{
    map_meta_bank = bank;
    __farmemcpy(&map_meta_current, bank, map, sizeof(metatile_map_t));
    map_meta_loaded = 1;
    map_loaded = 0;
    map_dirty = 0;
    map_camera_x = 0;
    map_camera_y = 0;
    map_draw_visible(0, 0);
}

// Read a row-major metatile ID, returning ID zero for inactive or out-of-range
// lookups. Callers must provide a usable definition for metatile zero.
u8 map_get_metatile(u8 tx, u8 ty)
{
    u16 index;

    if (map_meta_loaded == 0) return 0;
    if (tx >= map_meta_current.width || ty >= map_meta_current.height) return 0;

    index = __map_index(tx, ty, map_meta_current.width);
    return __farpeek8(map_meta_data_bank(), map_meta_current.metatile_ids + index);
}

// Return the selected definition's palette byte. An out-of-range coordinate
// uses the metatile-zero fallback; this function does not write CGB attributes.
u8 map_get_attr(u8 tx, u8 ty)
{
    metatile_t meta;
    u8 id;
    if (map_meta_loaded == 0) return 0;
    id = map_get_metatile(tx, ty);
    map_read_metatile(id, &meta);
    return meta.palette;
}

// Treat out-of-range metatile cells as solid; otherwise inspect the definition
// collision flag. In ordinary-map mode, delegate to its packed collision bits.
u8 map_is_solid(u8 tx, u8 ty)
{
    metatile_t meta;
    u8 id;
    if (map_meta_loaded == 0) return map_is_blocked(tx, ty);
    if (tx >= map_meta_current.width || ty >= map_meta_current.height) return 1;
    id = map_get_metatile(tx, ty);
    map_read_metatile(id, &meta);
    return (u8)(meta.collision != 0);
}

// Return a metatile event ID, or the ordinary-map Boolean trigger result.
// Missing/out-of-range metatile cells have no event.
u8 map_get_event(u8 tx, u8 ty)
{
    metatile_t meta;
    u8 id;
    if (map_meta_loaded == 0) return map_trigger_at(tx, ty);
    if (tx >= map_meta_current.width || ty >= map_meta_current.height) return 0;
    id = map_get_metatile(tx, ty);
    map_read_metatile(id, &meta);
    return meta.event;
}

// Draw a 10-by-9 metatile viewport as 20-by-18 individual background tiles.
// Out-of-range cells use definition zero; palette attributes are not written.
// Remember the camera and clear the dirty flag after the full redraw.
// Keep camera_tx <=246 and camera_ty <=247 so adding the viewport offsets cannot wrap the byte coordinates.
void map_draw_visible(u8 camera_tx, u8 camera_ty)
{
    u8 my;

    if (map_meta_loaded == 0) return;

    map_camera_x = camera_tx;
    map_camera_y = camera_ty;

    my = 0;
    while (my < 9) {
        u8 mx = 0;
        while (mx < 10) {
            u8 tx = (u8)(camera_tx + mx);
            u8 ty = (u8)(camera_ty + my);
            u8 sx = (u8)(mx << 1);
            u8 sy = (u8)(my << 1);
            metatile_t meta;
            u8 id = map_get_metatile(tx, ty);
            map_read_metatile(id, &meta);
            __settile_xy(sx, sy, meta.tile_tl);
            __settile_xy((u8)(sx + 1), sy, meta.tile_tr);
            __settile_xy(sx, (u8)(sy + 1), meta.tile_bl);
            __settile_xy((u8)(sx + 1), (u8)(sy + 1), meta.tile_br);
            mx++;
        }
        my++;
    }
    map_dirty = 0;
}

// Mark the viewport for a full redraw when a valid metatile cell changes.
// The coordinate is validated but no per-cell dirty list is retained.
// This schedules a redraw only; it does not modify ROM-backed metatile IDs or definitions.
void map_mark_dirty(u8 tx, u8 ty)
{
    if (map_meta_loaded == 0) return;
    if (tx >= map_meta_current.width || ty >= map_meta_current.height) return;
    map_dirty = 1;
}

// Redraw the remembered metatile viewport only when the shared dirty flag is set.
void map_flush_dirty()
{
    if (map_dirty == 0) return;
    map_draw_visible(map_camera_x, map_camera_y);
}
