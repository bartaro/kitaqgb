// 固定長オブジェクトプール
// Expected: 001
#include "gb_common.h"
#include "entity.h"

void main() {
    m_init();
    m_text(2,3,"ENTITY");
    u8 id; entity_init(); id=entity_create(1,20,30);
    if(id!=0xFF) entity_get(id)->vx=2; m_number(entity_count_active());
    while (1) { m_wait();  }
}
