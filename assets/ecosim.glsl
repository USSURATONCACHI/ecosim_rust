#version 430
layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

#include <terrain.glsl>

struct Entity {
    int x;
    int y;

    int health;
    int energy;
    int action_pts;
};

uniform ivec2 world_size;



void main() {

}
