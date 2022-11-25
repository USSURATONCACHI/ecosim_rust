#version 430
layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout(rgba32f, binding = 0) uniform image2D current_state;
//layout(rgba32f, binding = 1) uniform image2D next_state;

void main() {
    ivec2 pos = ivec2(gl_WorkGroupID.xy);

    imageStore(current_state, pos, vec4(vec2(pos) / 100.0, 0.0, 1.0));
    // imageStore(next_state, ivec2(0, 0), vec4(1.0, 0.0, 0.0, 1.0));
}

/*  int neighbours = 0;
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
    bool cell_updated = cell_state ? (neighbours == 2 || neighbours == 3) : (neighbours == 3);*/