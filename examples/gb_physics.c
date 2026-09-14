// Advance a rectangular body through one physics step.
// Expected: 042: integer pixel coordinates
#include "gb_common.h"
#include "physics2d.h"
KQWorld2D world; KQBody2D boxes[1];
// Create one box at X = 40, set horizontal velocity 2 and zero gravity,
// then display its X coordinate after a single physics step.
void main() {
    m_init();
    m_text(2,3,"PHYSICS");
    kq2d_world_init(&world,boxes,1);
    kq2d_body_init(&boxes[0],40,40,4,4);
    kq2d_body_set_velocity(&boxes[0],2,0);
    world.gravity_x=0; world.gravity_y=0;
    kq2d_step(&world); m_number((u8)boxes[0].x);
    while (1) { m_wait();  }
}
