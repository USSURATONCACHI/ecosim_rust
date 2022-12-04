#version 430
layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

layout(r8ui, binding = 0) uniform uimage2D current_state;
layout(r8ui, binding = 1) uniform uimage2D next_state;

uniform ivec2 world_size;
uniform uvec2 tile_offset;

void calc_cell(ivec2 pos);

void main() {
    uvec2 pos = gl_WorkGroupID.xy * gl_WorkGroupSize.xy + gl_LocalInvocationID.xy + tile_offset;
    if (pos.y >= world_size.y || pos.x >= world_size.x) {
        return;
    }
    calc_cell(ivec2(pos));
}

void calc_cell(ivec2 pos) {
    uint neighbours = 0;

    neighbours += imageLoad(current_state, pos + ivec2(-1, -1)).x;
    neighbours += imageLoad(current_state, pos + ivec2(-1, 0)).x;
    neighbours += imageLoad(current_state, pos + ivec2(-1, 1)).x;
    neighbours += imageLoad(current_state, pos + ivec2(0, -1)).x;
    neighbours += imageLoad(current_state, pos + ivec2(0, 1)).x;
    neighbours += imageLoad(current_state, pos + ivec2(1, -1)).x;
    neighbours += imageLoad(current_state, pos + ivec2(1, 0)).x;
    neighbours += imageLoad(current_state, pos + ivec2(1, 1)).x;

    bool cell_updated = imageLoad(current_state, pos).x > 0 ? (neighbours >= 4) : (neighbours >= 6);
    imageStore(next_state, pos, cell_updated ? uvec4(1) : uvec4(0));
}
