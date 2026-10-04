#include "physics3d.c"
#pragma bank 0
__wram KQBody3D bodies[2];
__wram KQWorld3D world;
__wram u16 results[32];
__wram u8 done;
void main() {
    u8 i;
    for(i=0;i<32;i++) results[i]=0;
    kq3d_body_init(&bodies[0],0,0,0,4,4,4,256);
    kq3d_body_init(&bodies[1],0,0,0,8,8,8,0);
    bodies[0].flags=KQ3D_BODY_SPHERE;
    bodies[0].x=12; results[0]=kq3d_overlap_sphere_aabb(&bodies[0],&bodies[1]);
    bodies[0].x=13; results[1]=kq3d_overlap_sphere_aabb(&bodies[0],&bodies[1]);
    bodies[0].x=11; bodies[0].y=11; results[2]=kq3d_overlap_sphere_aabb(&bodies[0],&bodies[1]);
    bodies[0].x=10; bodies[0].y=10; results[3]=kq3d_overlap_sphere_aabb(&bodies[0],&bodies[1]);
    results[4]=kq3d_overlap_sphere_aabb(0,&bodies[1]);
    results[5]=kq3d_overlap_sphere_aabb(&bodies[0],0);
    bodies[0].half_x=0; results[6]=kq3d_overlap_sphere_aabb(&bodies[0],&bodies[1]);
    kq3d_body_init(&bodies[0],20,30,40,4,4,4,0);
    kq3d_world_init(&world,bodies,1);
    world.gravity_x=0;world.gravity_y=0;world.gravity_z=0;
    bodies[0].flags=KQ3D_BODY_KINEMATIC;
    bodies[0].vx=3;bodies[0].vy=-2;bodies[0].vz=9;
    kq3d_step(&world);
    results[7]=bodies[0].x;results[8]=bodies[0].y;results[9]=bodies[0].z;
    results[10]=bodies[0].inv_mass_q8;
    kq3d_integrate_body(&world,&bodies[0]);
    results[11]=bodies[0].x;results[12]=bodies[0].y;results[13]=bodies[0].z;
    kq3d_body_init(&bodies[0],0,0,0,4,4,4,256);
    bodies[0].flags=KQ3D_BODY_BROKEN;bodies[0].break_speed=1;
    bodies[0].ax=1;bodies[0].last_impact_speed=99;
    kq3d_integrate_body(&world,&bodies[0]);
    results[14]=bodies[0].flags;results[15]=bodies[0].active;results[16]=bodies[0].last_impact_speed;
    done=165;
    while(1) {}
}
