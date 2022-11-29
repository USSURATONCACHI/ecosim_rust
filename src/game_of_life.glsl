#version 430
layout(local_size_x = 32, local_size_y = 1, local_size_z = 1) in;
#define LOCAL_SIZE 32

layout(rgba32f, binding = 0) uniform image2D current_state;
layout(rgba32f, binding = 1) uniform image2D next_state;

uniform ivec2 world_size;
uniform uint exec_offset;

void calc_cell(ivec2 pos);

void main() {
    uint work_group_index = gl_WorkGroupID.z * gl_NumWorkGroups.y * gl_NumWorkGroups.x +
                            gl_WorkGroupID.y * gl_NumWorkGroups.x +
                            gl_WorkGroupID.x;
    uint global_exec_id = work_group_index * LOCAL_SIZE + gl_LocalInvocationIndex + exec_offset;

    ivec2 pos = ivec2(int(global_exec_id) % world_size.x, int(global_exec_id) / world_size.x);

    if (pos.y >= world_size.y) {
        return;
    }

    //imageStore(next_state, pos, vec4(vec2(pos) / 100.0, 0.0, 1.0));
    calc_cell(pos);
}

void calc_cell(ivec2 pos) {
    int neighbours = 0;
    for (int dx = -1; dx <= 1; dx++) {
        for (int dy = -1; dy <= 1; dy++) {
            if (dx == 0 && dy == 0)
            continue;

            vec3 pixel = imageLoad(current_state, pos + ivec2(dx, dy)).xyz;
            if (pixel.x >= 0.95)
            neighbours++;
        }
    }

    bool cell_state = imageLoad(current_state, pos).xyz == vec3(1.0, 1.0, 1.0);
    bool cell_updated = cell_state ? (neighbours == 2 || neighbours == 3) : (neighbours == 3);

    // imageStore(current_state, pos, vec4(vec2(pos) / 100.0, 0.0, 1.0));
    imageStore(next_state, pos, cell_updated ? vec4(1.0, 1.0, 1.0, 1.0) : vec4(0.0, 0.0, 0.0, 1.0));
}