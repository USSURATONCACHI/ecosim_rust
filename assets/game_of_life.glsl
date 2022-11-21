#version 430
layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout(rgb8ui) uniform uimage2D current_state;
layout(rgb8ui) uniform uimage2D next_state;

void main() {
    ivec2 pos = ivec2(gl_WorkGroupID.xy);

    int neighbours = 0;
    for (int dx = -1; dx <= 1; dx++) {
        for (int dy = -1; dy <= 1; dy++) {
            if (dx == 0 && dy == 0)
            continue;

            uvec3 pixel = imageLoad(current_state, pos + ivec2(dx, dy)).xyz;
            if (pixel == uvec3(255, 255, 255))
            neighbours++;
        }
    }

    bool cell_state = imageLoad(current_state, pos).xyz == uvec3(255, 255, 255);
    bool cell_updated = cell_state ? (neighbours == 2 || neighbours == 3) : (neighbours == 3);

    imageStore(next_state, pos, cell_updated ? uvec4(255) : uvec4(0));
}