#version 430
layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

layout(r32i, binding = 0) readonly uniform iimage2D current_state;
layout(r32i, binding = 1) writeonly uniform iimage2D next_state;

void main() {
    uvec2 invoc_pos = gl_WorkGroupID.xy * gl_WorkGroupSize.xy + gl_LocalInvocationID.xy;

    ivec4 val = imageLoad(current_state, ivec2(invoc_pos));
    imageStore(next_state, ivec2(invoc_pos), val);
}