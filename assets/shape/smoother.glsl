#version 430
layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

layout(r8ui, binding = 0) readonly uniform uimage2D current_state;
layout(r8ui, binding = 1) coherent uniform uimage2D next_state;

layout(std430, binding = 2) buffer buf_cells_taken {
    uint b_cells_taken;
};

uniform ivec2 u_size;

void main() {
    ivec2 pos = ivec2(gl_WorkGroupID.xy * gl_WorkGroupSize.xy + gl_LocalInvocationID.xy);

    if (pos.x <= 0 || pos.y <= 0 || pos.x >= u_size.x - 1 || pos.y >= u_size.y - 1)
        return;

    uint state = imageLoad(current_state, pos).x;
    if (state != 0) {
        imageStore(next_state, pos, uvec4(state));
        return;
    }

    // if any neighbour is true, make true
    uint neighbours = 0;

    neighbours += imageLoad(current_state, pos + ivec2(-1, 0)).x == 2 ? uint(1) : uint(0);
    neighbours += imageLoad(current_state, pos + ivec2(0, -1)).x == 2 ? uint(1) : uint(0);
    neighbours += imageLoad(current_state, pos + ivec2(0, 1)).x == 2 ? uint(1) : uint(0);
    neighbours += imageLoad(current_state, pos + ivec2(1, 0)).x == 2 ? uint(1) : uint(0);

    if (neighbours > 0) {
        imageStore(next_state, pos, uvec4(2));
        atomicAdd(b_cells_taken, 1);
    }
}
