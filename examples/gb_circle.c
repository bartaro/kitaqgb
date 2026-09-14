// Advance a circular body through one physics step.
// Expected: 042 after one step; this example uses integer pixels
#include "gb_common.h"
#include "physics2d_circle.h"
KQCircleBody2D balls[1]; KQCircleWorld2D world;
// Activate one circle at X = 40 with horizontal velocity 2 and zero gravity.
// Advance one step and display its resulting integer-pixel X coordinate.
void main() {
    m_init();
    m_text(2,3,"CIRCLE");
    kq2dc_world_init(&world,balls,1);
    balls[0].active=1; balls[0].x=40; balls[0].y=40;
    balls[0].radius=4; balls[0].inv_mass_q8=256;
    balls[0].vx=2; balls[0].vy=0;
    world.gravity_x=0; world.gravity_y=0; world.linear_damping_q8=255;
    kq2dc_step(&world); m_number((u8)balls[0].x);
    while (1) { m_wait();  }
}
