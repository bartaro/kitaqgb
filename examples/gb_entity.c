// Allocate an entity from a fixed-capacity object pool.
// Expected: 001
#include "gb_common.h"
#include "entity.h"

// Create one entity and check the 0xFF allocation-failure sentinel before
// accessing its velocity. Display the pool occupancy, not the entity ID.
void main() {
    m_init();
    m_text(2,3,"ENTITY");
    u8 id; entity_init(); id=entity_create(1,20,30);
    if(id!=0xFF) entity_get(id)->vx=2; m_number(entity_count_active());
    while (1) { m_wait();  }
}
