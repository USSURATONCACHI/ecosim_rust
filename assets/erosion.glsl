/*
Erosion code is copied (and modified) from here: https://github.com/SebLague/Hydraulic-Erosion/blob/master/Assets/Scripts/Erosion.cs
Original code licence:

MIT License

Copyright (c) 2019 Sebastian Lague

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
*/

#version 430
layout(local_size_x = 32, local_size_y = 32, local_size_z = 1) in;

#define erosionRadius 3

// At zero, water will instantly change direction to flow downhill. At 1, water will never change direction.
#define inertia .1f

// Multiplier for how much sediment a droplet can carry
#define sedimentCapacityFactor 4.0

// Used to prevent carry capacity getting too close to zero on flatter terrain
#define minSedimentCapacity .01f

#define erodeSpeed .3f
#define depositSpeed .3f
#define evaporateSpeed .01f
#define gravity 9.0
#define maxDropletLifetime 30

#define initialWaterVolume 2.0
#define initialSpeed 1.0

#define EPS 0.000001

#define INT_VAL_RANGE 1000000

layout(r32i, binding = 0) uniform iimage2D map;
uniform ivec2 u_map_size;
uniform uvec2 u_tile_offset;
uniform int u_random_seed;

// 2D array
layout(std430, binding = 1) buffer _erosion_brush_indices_data { int erosion_brush_indices_data[]; };
layout(std430, binding = 2) buffer _erosion_brush_indices_slices { int erosion_brush_indices_slices[]; };    // [start_index_0, length_0, start_index_1, length_1, ...]
uniform int erosion_brush_indices_slices_count;

// 2D array
layout(std430, binding = 3) buffer _erosion_brush_weights_data { float erosion_brush_weights_data[]; };
layout(std430, binding = 4) buffer _erosion_brush_weights_slices { int erosion_brush_weights_slices[]; };    // [start_index_0, length_0, start_index_1, length_1, ...]
uniform int erosion_brush_weights_slices_count;

struct HeightAndGradient {
    float height;
    vec2 gradient;
};
HeightAndGradient CalculateHeightAndGradient(vec2 pos);

void erode(ivec2 pos);
float rand_float(ivec2 pos, float prev_rand);
uint uhash2(uvec2 s);

float get_pixel(ivec2 pos);
float atomic_add_pixel(ivec2 pos, float diff);

int get_erosion_brush_indices_arr_length(int index);
int get_erosion_brush_indices_elem(int i, int j);

int get_erosion_brush_weights_arr_length(int index);
float get_erosion_brush_weights_elem(int i, int j);

void main() {
    uvec2 in_pos = gl_WorkGroupID.xy * gl_WorkGroupSize.xy + gl_LocalInvocationID.xy + u_tile_offset;
    /*if (pos.y >= u_map_size.y || pos.x >= u_map_size.x) {
        return;
    }*/
    vec2 pos = ivec2(vec2(rand_float(ivec2(in_pos), 17.0), rand_float(ivec2(in_pos), 42.0)) * vec2(u_map_size - 1));
    // atomic_add_pixel(ivec2(pos), -0.01);
    erode(ivec2(pos));
}

void erode(ivec2 i_pos) {
    //int height = imageLoad(map, i_pos).x;
    //imageStore(map, i_pos, ivec4(height * 3 / 4, 0, 0, 0));
    // Create droplet at random point
    vec2 pos = vec2(i_pos) + vec2(rand_float(i_pos, 0.0), rand_float(i_pos, 1.0));
    vec2 dir = vec2(0.0);
    float speed = initialSpeed;
    float water = initialWaterVolume;
    float sediment = 0.0;

    for (int lifetime = 0; lifetime < maxDropletLifetime; lifetime++) {
        ivec2 node = ivec2(pos);
        int dropletIndex = node.y * u_map_size.x + node.x;
        // Calculate droplet's offset inside the cell (0,0) = at NW node, (1,1) = at SE node
        vec2 cellOffset = fract(pos);

        // Calculate droplet's height and direction of flow with bilinear interpolation of surrounding heights
        HeightAndGradient heightAndGradient = CalculateHeightAndGradient(pos);

        // Update the droplet's direction and position (move position 1 unit regardless of speed)
        dir = (dir * inertia - heightAndGradient.gradient * (1.0 - inertia));
        // Normalize direction
        float len = length(dir);
        if (len >= EPS) {
            dir /= len;
        }
        pos += dir;

        // Stop simulating droplet if it's not moving or has flowed over edge of map
        if ((dir.x < EPS && dir.y < EPS) || pos.x < 0.0 || pos.x >= float(u_map_size.x - 1.0) || pos.y < 0.0 || pos.y >= float(u_map_size.y - 1.0)) {
            break;
        }

        // Find the droplet's new height and calculate the deltaHeight
        float newHeight = CalculateHeightAndGradient(pos).height;
        float deltaHeight = newHeight - heightAndGradient.height;

        // Calculate the droplet's sediment capacity (higher when moving fast down a slope and contains lots of water)
        float sedimentCapacity = max(-deltaHeight * speed * water * sedimentCapacityFactor, minSedimentCapacity);

        // If carrying more sediment than capacity, or if flowing uphill:
        if (sediment > sedimentCapacity || deltaHeight > 0.0) {
            // If moving uphill (deltaHeight > 0) try fill up to the current height, otherwise deposit a fraction of the excess sediment
            float amountToDeposit = (deltaHeight > 0.0) ? min(deltaHeight, sediment) : (sediment - sedimentCapacity) * depositSpeed;
            sediment -= amountToDeposit;

            // Add the sediment to the four nodes of the current cell using bilinear interpolation
            // Deposition is not distributed over a radius (like erosion) so that it can fill small pits
            atomic_add_pixel(node, amountToDeposit * (1 - cellOffset.x) * (1 - cellOffset.y));
            atomic_add_pixel(node + ivec2(0, 1), amountToDeposit * cellOffset.x * (1 - cellOffset.y));
            atomic_add_pixel(node + ivec2(1, 0), amountToDeposit * (1 - cellOffset.x) * cellOffset.y);
            atomic_add_pixel(node + ivec2(1, 1), amountToDeposit * cellOffset.x * cellOffset.y);
        } else {
            // Erode a fraction of the droplet's current carry capacity.
            // Clamp the erosion to the change in height so that it doesn't dig a hole in the terrain behind the droplet
            float amountToErode = min((sedimentCapacity - sediment) * erodeSpeed, -deltaHeight);

            // Use erosion brush to erode from all nodes inside the droplet's erosion radius
            for (int brushPointIndex = 0; brushPointIndex < get_erosion_brush_indices_arr_length(dropletIndex); brushPointIndex++) {
                int nodeIndex = get_erosion_brush_indices_elem(dropletIndex, brushPointIndex);
                ivec2 local_node = ivec2(nodeIndex % u_map_size.x, nodeIndex / u_map_size.x);

                float weighedErodeAmount = amountToErode * get_erosion_brush_weights_elem(dropletIndex, brushPointIndex);

                float pixel_value = get_pixel(local_node);
                float deltaSediment = (pixel_value < weighedErodeAmount) ? pixel_value : weighedErodeAmount;
                atomic_add_pixel(local_node, -deltaSediment);
                sediment += deltaSediment;
            }
        }

        // Update droplet's speed and water content
        speed = sqrt(speed * speed + deltaHeight * gravity);
        water *= (1.0 - evaporateSpeed);
    }

}

HeightAndGradient CalculateHeightAndGradient(vec2 pos) {
    ivec2 coord = ivec2(pos);

    // Calculate droplet's offset inside the cell (0,0) = at NW node, (1,1) = at SE node
    float x = fract(pos.x);
    float y = fract(pos.y);

    // Calculate heights of the four nodes of the droplet's cell
    float heightNW = get_pixel(coord);
    float heightNE = get_pixel(coord + ivec2(0, 1));
    float heightSW = get_pixel(coord + ivec2(1, 0));
    float heightSE = get_pixel(coord + ivec2(1, 1));

    // Calculate droplet's direction of flow with bilinear interpolation of height difference along the edges
    float gradientX = (heightNE - heightNW) * (1.0 - y) + (heightSE - heightSW) * y;
    float gradientY = (heightSW - heightNW) * (1.0 - x) + (heightSE - heightNE) * x;

    // Calculate height with bilinear interpolation of the heights of the nodes of the cell
    float height = heightNW * (1.0 - x) * (1.0 - y) + heightNE * x * (1.0 - y) + heightSW * (1.0 - x) * y + heightSE * x * y;

    return HeightAndGradient(height, vec2(gradientX, gradientY));
}

float rand_float(ivec2 pos, float prev_rand) {
    pos = pos + ivec2(int(fract(prev_rand) * 100000.0), int(mod(prev_rand, 100000.0)));
    uint raw_val = uhash2(uvec2(pos)) % 10000000;
    return float(raw_val) / 10000000.0;
}

uint uhash2(uvec2 s) {
    uvec4 s1;
    s1 = (uvec4(s >> 16u, s & 0xFFFFu) * uvec4(0x7D202CFBu, 0xEDA6A77Du, 0x43EF69ABu, 0xE5C5A9ADu)) +
    uvec4(0x61C65DE7u, 0x7A0F89EFu, 0x8AF12C51u, 0x927E0E2Bu);

    s1 = (uvec4((s1.xz ^ s1.yw) >> 16u, (s1.xz ^ s1.yw) & 0xFFFFu) * uvec4(0x11028C59u, 0xFDA77C39u, 0x26783951u, 0x15A4DBB7u)) +
    uvec4(0xA5041D0Du, 0x27AE1933u, 0xDC1CA48Du, 0x577AE491u);

    s1 = (uvec4((s1.xy ^ s1.zw) >> 16u, (s1.xy ^ s1.zw) & 0xFFFFu) * uvec4(0x0FF1738Du, 0x6A5A87E1u, 0xED8C6B77u, 0xE97B7CC1u)) +
    uvec4(0xFFABDEAFu, 0xCFA02E1Fu, 0x401BE42Fu, 0x8E7195F1u);

    s1 = (uvec4((s1.xz ^ s1.yw) >> 16u, (s1.xz ^ s1.yw) & 0xFFFFu) * uvec4(0x486E046Du, 0xAA219B31u, 0x645CF729u, 0x384865D9u))
    + uvec4(0xA56EC0FBu, 0xBA8225C3u, 0xAC8003F3u, 0xCC7C86F7u);

    return ((((s1.x * 0xE7A2CA7Bu) ^ (s1.y * 0xB294EB91u)) * 0xEA1C1AF9u) ^
    (((s1.z * 0x6D95A9B9u) ^ (s1.w * 0x227A3011u)) * 0x9EE8315Bu)) * 0xC4830579u;
}

float get_pixel(ivec2 pos) {
    int int_val = imageLoad(map, pos).x;
    return float(int_val) / float(INT_VAL_RANGE);
}

float atomic_add_pixel(ivec2 pos, float diff) {
    int add_val = int(diff * float(INT_VAL_RANGE));
    int old_val = imageAtomicAdd(map, pos, add_val);
    return float(old_val) / float(INT_VAL_RANGE);
}

int get_erosion_brush_indices_arr_length(int index) {
    if (index >= erosion_brush_indices_slices_count) {
        return -1;
    } else {
        return erosion_brush_indices_slices[index * 2 + 1];
    }
}
int get_erosion_brush_indices_elem(int i, int j) {
    if (i >= erosion_brush_indices_slices_count) {
        return -1;
    } else {
        int start_index = erosion_brush_indices_slices[i * 2 + 0];
        int length = erosion_brush_indices_slices[i * 2 + 1];

        if (j >= length) {
            return -1;
        } else {
            return erosion_brush_indices_data[start_index + j];
        }
    }
}

int get_erosion_brush_weights_arr_length(int index) {
    if (index >= erosion_brush_weights_slices_count) {
        return -1;
    } else {
        return erosion_brush_weights_slices[index * 2 + 1];
    }
}
float get_erosion_brush_weights_elem(int i, int j) {
    if (i >= erosion_brush_weights_slices_count) {
        return -1.0;
    } else {
        int start_index = erosion_brush_weights_slices[i * 2 + 0];
        int length = erosion_brush_weights_slices[i * 2 + 1];

        if (j >= length) {
            return -1.0;
        } else {
            return erosion_brush_weights_data[start_index + j];
        }
    }
}