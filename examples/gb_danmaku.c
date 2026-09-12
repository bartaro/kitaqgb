// 弾プールの生成と個数
// Expected: 001; numeric pool example, no bullet BG compositor
#include "gb_common.h"
#include "danmaku.h"

void main() {
    m_init();
    m_text(2,3,"DANMAKU");
    danmaku_reset(); danmaku_spawn(40,40,16,0);
    m_number(dm_count);
    while (1) { m_wait();  }
}
