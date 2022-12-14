#version 430
layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(r32i, binding = 0) readonly uniform iimage2D current_state;
layout(r32i, binding = 1) coherent uniform iimage2D next_state;
uniform ivec2 u_map_size;
uniform int u_random_seed;

float get_pixel(ivec2 pos);
float atomic_add_pixel(ivec2 pos, float diff);

// -- Includes --
#define BRUSH_GLSL__MAP_WIDTH   u_map_size.x
#define BRUSH_GLSL__MAP_HEIGHT  u_map_size.y
#include<brush.glsl>

#define GRADIENT_GLSL__GET_PIXEL(pos) get_pixel(pos)
#define GRADIENT_GLSL__MAP_WIDTH u_map_size.x
#define GRADIENT_GLSL__MAP_HEIGHT u_map_size.y
#include<gradient.glsl>

#include<random.glsl>

// -- Defines and functions signatures --
#define EPS 0.000001
#define MAX_DROPLET_LIFETIME 30
#define INITIAL_WATER_VOLUME 1.0
#define INERTIA 0.2

#define SOIL_CAPACITY_PER_WATER 4.0
#define MIN_SOIL_CAPACITY 0.02
#define DEPOSIT_SPEED 0.3
#define ERODE_SPEED 0.3
#define EVAPORATE_SPEED 0.01

void simulate_droplet(vec2 pos);

// -- Code --
void main() {
    uvec2 invoc_seed = gl_WorkGroupID.xy * gl_WorkGroupSize.xy + gl_LocalInvocationID.xy;
    uvec2 unbound_pos = u2hash2( invoc_seed + uvec2(u_map_size) * uint(u_random_seed) * 17 );
    uvec2 pos = uvec2(unbound_pos.x % u_map_size.x, unbound_pos.y % u_map_size.y);

    //atomic_add_pixel(ivec2(pos), -get_pixel(ivec2(pos)));
    simulate_droplet(vec2(pos));
}

void simulate_droplet(vec2 pos) {
    vec2 vel = vec2(0.0);//CalculateHeightAndGradient(pos).gradient;
    float water_volume = INITIAL_WATER_VOLUME;
    float soil_amount = 0.0;
    //atomic_add_pixel(ivec2(pos), -get_pixel(ivec2(pos)));

    for (int lifetime = 0; lifetime < MAX_DROPLET_LIFETIME; lifetime++) {
        ivec2 current_texel = ivec2(pos);
        vec2 cell_offset = fract(pos);

        HeightAndGradient height_and_grad = CalculateHeightAndGradient(pos);
        vel = (vel * INERTIA - height_and_grad.gradient * (1.0 - INERTIA));

        float vel_length = length(vel);
        vec2 direction = vel / vel_length;
        pos += direction;

        if (vel_length < EPS || pos.x < 0.0 || pos.y < 0.0 || pos.x >= float(u_map_size.x) || pos.y >= float(u_map_size.y))
            break;

        // Find the droplet's new height and calculate the deltaHeight
        float new_height = CalculateHeightAndGradient(pos).height;
        float delta_height = new_height - height_and_grad.height;

        float cur_soil_capacity = max(-delta_height * vel_length * water_volume * SOIL_CAPACITY_PER_WATER, MIN_SOIL_CAPACITY);
        //atomic_add_pixel(ivec2(pos), -0.1 * get_pixel(ivec2(pos)));

        if (soil_amount > cur_soil_capacity || delta_height > 0.0) {
            float amount_to_deposit = (delta_height > 0.0) ?
                min(delta_height, soil_amount) : (soil_amount - cur_soil_capacity) * DEPOSIT_SPEED;

            soil_amount -= amount_to_deposit;

            atomic_add_pixel(current_texel + ivec2(0, 0), amount_to_deposit * (1.0 - cell_offset.x) * (1.0 - cell_offset.y));
            atomic_add_pixel(current_texel + ivec2(1, 0), amount_to_deposit * cell_offset.x * (1.0 - cell_offset.y));
            atomic_add_pixel(current_texel + ivec2(0, 1), amount_to_deposit * (1.0 - cell_offset.x) * cell_offset.y);
            atomic_add_pixel(current_texel + ivec2(1, 1), amount_to_deposit * cell_offset.x * cell_offset.y);
        } else {
            // Erode a fraction of the droplet's current carry capacity.
            // Clamp the erosion to the change in height so that it doesn't dig a hole in the terrain behind the droplet
            float amountToErode = min((cur_soil_capacity - soil_amount) * ERODE_SPEED, -delta_height);

            // Use erosion brush to erode from all nodes inside the droplet's erosion radius
            for (int brushPointIndex = 0; brushPointIndex < get_brush_indices_count(current_texel); brushPointIndex++) {
                BrushIndex index = get_brush_index(current_texel, brushPointIndex);
                float weighedErodeAmount = amountToErode * index.weight;

                ivec2 node = ivec2(index.cell_x, index.cell_y);
                float pixel_value = get_pixel(node);
                float delta_soil = (pixel_value < weighedErodeAmount) ? max(pixel_value, 0.0) : weighedErodeAmount;
                atomic_add_pixel(node, -delta_soil);
                soil_amount += delta_soil;
            }
        }
        water_volume *= (1.0 - EVAPORATE_SPEED);
    }

    /*ivec2 current_texel = ivec2(pos);
    vec2 cell_offset = fract(pos);
    atomic_add_pixel(current_texel + ivec2(0, 0), soil_amount * (1.0 - cell_offset.x) * (1.0 - cell_offset.y));
    atomic_add_pixel(current_texel + ivec2(0, 1), soil_amount * cell_offset.x * (1.0 - cell_offset.y));
    atomic_add_pixel(current_texel + ivec2(1, 0), soil_amount * (1.0 - cell_offset.x) * cell_offset.y);
    atomic_add_pixel(current_texel + ivec2(1, 1), soil_amount * cell_offset.x * cell_offset.y);*/
}

// Value of 1.0 converted to int value
#define INT_VAL_RANGE 1000000

float get_pixel(ivec2 pos) {
    int int_val = imageLoad(current_state, pos).x;
    return float(int_val) / float(INT_VAL_RANGE);
}

float atomic_add_pixel(ivec2 pos, float diff) {
    int add_val = int(diff * float(INT_VAL_RANGE));
    int old_val = imageAtomicAdd(next_state, pos, add_val);
    return float(old_val) / float(INT_VAL_RANGE);
}