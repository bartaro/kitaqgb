#include "rpg.h"

// Shared non-reentrant BFS workspace. The current map must fit 1024 cells;
// byte distances must remain below 255 because 0xFF marks an unvisited cell.
static u8 path_dist[1024];
static u16 path_queue[1024];
static u8 path_rev[1024];

// Flood cardinal neighbors at unit cost, stopping expansion at the movement
// limit and rejecting blocked destinations. Initialize output to 0xFF only
// after validating the map and origin; invalid input leaves output unchanged.
// Require width*height <=1024 and move <=254; neither limit is checked here.
// The origin is seeded even if its own cell is blocked, allowing paths out of that cell.
void range_fill_move(u8 sx, u8 sy, u8 move, u8* out_costmap)
{
    u8 w = map_current_width();
    u8 h = map_current_height();
    u16 total;
    u16 head = 0;
    u16 tail = 0;
    u16 start;

    if (w == 0 || h == 0) return;
    if (sx >= w || sy >= h) return;

    total = (u16)((u16)w * (u16)h);
    __memset(out_costmap, 0xFF, total);

    start = __map_index(sx, sy, w);
    out_costmap[start] = 0;
    path_queue[tail++] = start;

    while (head < tail) {
        u16 cur = path_queue[head++];
        u8 cost = out_costmap[cur];
        u8 y = (u8)(cur / w);
        u8 x = (u8)(cur - ((u16)y * (u16)w));

        if (cost >= move) continue;

        if (y > 0) {
            u16 next = (u16)(cur - w);
            if (out_costmap[next] == 0xFF && map_is_blocked(x, (u8)(y - 1)) == 0) {
                out_costmap[next] = (u8)(cost + 1);
                path_queue[tail++] = next;
            }
        }
        if ((u8)(x + 1) < w) {
            u16 next = (u16)(cur + 1);
            if (out_costmap[next] == 0xFF && map_is_blocked((u8)(x + 1), y) == 0) {
                out_costmap[next] = (u8)(cost + 1);
                path_queue[tail++] = next;
            }
        }
        if ((u8)(y + 1) < h) {
            u16 next = (u16)(cur + w);
            if (out_costmap[next] == 0xFF && map_is_blocked(x, (u8)(y + 1)) == 0) {
                out_costmap[next] = (u8)(cost + 1);
                path_queue[tail++] = next;
            }
        }
        if (x > 0) {
            u16 next = (u16)(cur - 1);
            if (out_costmap[next] == 0xFF && map_is_blocked((u8)(x - 1), y) == 0) {
                out_costmap[next] = (u8)(cost + 1);
                path_queue[tail++] = next;
            }
        }
    }
}

// Find a shortest cardinal path with BFS and write direction codes 0=up,
// 1=right, 2=down, 3=left. Return its byte length, or zero for failure or an
// already-reached goal. The caller supplies max_len output bytes and respects
// the shared workspace/distance limits; excessive paths are not truncated.
// Require at most 1024 cells and all explored distances below 255; max_len only limits returned output,
// not the search radius. An unvisited FF distance must never become a legitimate reachable cost.
u8 path_find_bfs(u8 sx, u8 sy, u8 gx, u8 gy, u8* out_path, u8 max_len)
{
    u8 w = map_current_width();
    u8 h = map_current_height();
    u16 total;
    u16 head = 0;
    u16 tail = 0;
    u16 start;
    u16 goal;
    u8 found = 0;

    if (w == 0 || h == 0) return 0;
    if (sx >= w || sy >= h || gx >= w || gy >= h) return 0;
    if ((sx != gx || sy != gy) && map_is_blocked(gx, gy) != 0) return 0;

    total = (u16)((u16)w * (u16)h);
    __memset(path_dist, 0xFF, total);

    start = __map_index(sx, sy, w);
    goal = __map_index(gx, gy, w);

    path_dist[start] = 0;
    path_queue[tail++] = start;

    while (head < tail) {
        u16 cur = path_queue[head++];
        u8 cost = path_dist[cur];
        u8 y = (u8)(cur / w);
        u8 x = (u8)(cur - ((u16)y * (u16)w));

        if (cur == goal) {
            found = 1;
            break;
        }

        if (y > 0) {
            u16 next = (u16)(cur - w);
            if (path_dist[next] == 0xFF && map_is_blocked(x, (u8)(y - 1)) == 0) {
                path_dist[next] = (u8)(cost + 1);
                path_queue[tail++] = next;
            }
        }
        if ((u8)(x + 1) < w) {
            u16 next = (u16)(cur + 1);
            if (path_dist[next] == 0xFF && map_is_blocked((u8)(x + 1), y) == 0) {
                path_dist[next] = (u8)(cost + 1);
                path_queue[tail++] = next;
            }
        }
        if ((u8)(y + 1) < h) {
            u16 next = (u16)(cur + w);
            if (path_dist[next] == 0xFF && map_is_blocked(x, (u8)(y + 1)) == 0) {
                path_dist[next] = (u8)(cost + 1);
                path_queue[tail++] = next;
            }
        }
        if (x > 0) {
            u16 next = (u16)(cur - 1);
            if (path_dist[next] == 0xFF && map_is_blocked((u8)(x - 1), y) == 0) {
                path_dist[next] = (u8)(cost + 1);
                path_queue[tail++] = next;
            }
        }
    }

    // Reject unreachable or oversized paths before reconstructing output.
    if (found == 0) return 0;
    if (path_dist[goal] > max_len) return 0;

    {
        // Walk backward through decreasing costs, recording forward directions in
        // reverse order. The fixed neighbor order makes tied routes deterministic.
        u8 len = 0;
        u16 cur = goal;

        while (cur != start) {
            u8 cur_cost = path_dist[cur];
            u8 y = (u8)(cur / w);
            u8 x = (u8)(cur - ((u16)y * (u16)w));

            if (y > 0 && path_dist[(u16)(cur - w)] == (u8)(cur_cost - 1)) {
                path_rev[len++] = 2;
                cur = (u16)(cur - w);
                continue;
            }
            if ((u8)(x + 1) < w && path_dist[(u16)(cur + 1)] == (u8)(cur_cost - 1)) {
                path_rev[len++] = 3;
                cur = (u16)(cur + 1);
                continue;
            }
            if ((u8)(y + 1) < h && path_dist[(u16)(cur + w)] == (u8)(cur_cost - 1)) {
                path_rev[len++] = 0;
                cur = (u16)(cur + w);
                continue;
            }
            if (x > 0 && path_dist[(u16)(cur - 1)] == (u8)(cur_cost - 1)) {
                path_rev[len++] = 1;
                cur = (u16)(cur - 1);
                continue;
            }
            return 0;
        }

        // Reverse the recovered directions so the first output step starts at the origin.
        while (len != 0) {
            len--;
            out_path[path_dist[goal] - 1 - len] = path_rev[len];
        }

        return path_dist[goal];
    }
}
