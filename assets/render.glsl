#version 330

in vec2 f_tex_coords;
out vec4 out_color;

uniform vec2 u_world_size;
uniform vec2 u_screen_size;
uniform vec2 u_camera_pos;
uniform float u_camera_zoom;
uniform int u_antialiasing;

uniform uint u_render_type;

uniform usampler2D u_world_texture;
uniform isampler2D u_landscape;

float get_pixel(ivec2 pos);

#include<terrain/terrain.glsl>

#define GRADIENT_GLSL__GET_PIXEL(pos) get_pixel(pos)
#define GRADIENT_GLSL__MAP_WIDTH u_world_size.x
#define GRADIENT_GLSL__MAP_HEIGHT u_world_size.y
#include<terrain/gradient.glsl>

// #define WORLD_WRAP_X
// #define WORLD_WRAP_Y

float cube(float x);
float interp1(float x);
vec4 get_world_color(vec2 world_pos);
vec4 render(vec2 frag_pos);

void main() {
    vec2 frag_pos_px = f_tex_coords * u_screen_size;

    float bxy = int(frag_pos_px.x + frag_pos_px.y) & 1;
    float nbxy = 1. - bxy;

    int MSAA = u_antialiasing;
    // NAA x1
    ///=========
    if (MSAA == 1) {
        out_color = render(frag_pos_px + vec2(0.0));
    } else
    // MSAA x2
    ///=========
    if (MSAA == 2) {
        out_color = (render(frag_pos_px + vec2(0.33 * nbxy, 0.)) + render(frag_pos_px + vec2(0.33 * bxy, 0.33))) / 2.;
    } else
    // MSAA x3
    ///=========
    if (MSAA == 3) {
        out_color = (render(frag_pos_px + vec2(0.66 * nbxy, 0.)) + render(frag_pos_px + vec2(0.66 * bxy, 0.66)) + render(frag_pos_px + vec2(0.33, 0.33))) / 3.;
    } else
    // MSAA x4
    ///=========
    if (MSAA == 4) { // rotate grid
        out_color = (render(frag_pos_px + vec2(0.33, 0.1)) + render(frag_pos_px + vec2(0.9, 0.33)) + render(frag_pos_px + vec2(0.66, 0.9)) + render(frag_pos_px + vec2(0.1, 0.66))) / 4.;
    } else
    // MSAA x5
    ///=========
    if (MSAA == 5) { // centre rotate grid
        out_color = (render(frag_pos_px + vec2(0.33, 0.2)) + render(frag_pos_px + vec2(0.8, 0.33)) + render(frag_pos_px + vec2(0.66, 0.8)) + render(frag_pos_px + vec2(0.2, 0.66)) + render(frag_pos_px + vec2(0.5, 0.5))) / 5.;
    } else
    // SSAA x9
    ///=========
    if (MSAA == 9) {  // centre grid 3x3
        out_color = (
            render(frag_pos_px + vec2(0.17, 0.2)) + render(frag_pos_px + vec2(0.17, 0.83)) + render(frag_pos_px + vec2(0.83, 0.17)) + render(frag_pos_px + vec2(0.83, 0.83)) +
            render(frag_pos_px + vec2(0.5, 0.17)) + render(frag_pos_px + vec2(0.17, 0.5)) + render(frag_pos_px + vec2(0.5, 0.83)) + render(frag_pos_px + vec2(0.83, 0.5)) +
            render(frag_pos_px + vec2(0.5, 0.5)) * 2.) / 10.;
    } else
    // SSAA x16
    ///=========
    if (MSAA == 16) { // classic grid 4x4
        out_color = (
            render(frag_pos_px + vec2(0.00, 0.00)) + render(frag_pos_px + vec2(0.25, 0.00)) + render(frag_pos_px + vec2(0.50, 0.00)) + render(frag_pos_px + vec2(0.75, 0.00)) +
            render(frag_pos_px + vec2(0.00, 0.25)) + render(frag_pos_px + vec2(0.25, 0.25)) + render(frag_pos_px + vec2(0.50, 0.25)) + render(frag_pos_px + vec2(0.75, 0.25)) +
            render(frag_pos_px + vec2(0.00, 0.50)) + render(frag_pos_px + vec2(0.25, 0.50)) + render(frag_pos_px + vec2(0.50, 0.50)) + render(frag_pos_px + vec2(0.75, 0.50)) +
            render(frag_pos_px + vec2(0.00, 0.75)) + render(frag_pos_px + vec2(0.25, 0.75)) + render(frag_pos_px + vec2(0.50, 0.75)) + render(frag_pos_px + vec2(0.75, 0.75))) / 16.;
    }
}

vec4 render(vec2 frag_pos) {

    float cam_scale = pow(2.0, u_camera_zoom);
    vec2 world_coords = (frag_pos + u_camera_pos * cam_scale) / cam_scale;

    #ifdef WORLD_WRAP_X
        world_coords.x = mod(mod(world_coords.x, u_world_size.x) + u_world_size.x, u_world_size.x);
    #endif

    #ifdef WORLD_WRAP_Y
        world_coords.y = mod(mod(world_coords.y, u_world_size.y) + u_world_size.y, u_world_size.y);
    #endif

    vec4 color = get_world_color(world_coords);

    //Затенение краёв клеточек
    float klc = pow(interp1(fract(world_coords.x))*interp1(fract(world_coords.y)), 0.2);
    //Нужно ли рисовать сетку
    float is_big_enough = min(max(cam_scale - 3.0, 0.0), 5.0)/5.0;
    //Рассчет и применение затемнения
    klc = 1.0 - (1.0 - klc)*is_big_enough;
    color.rgb *= klc;

    return color;
}

#define INT_VAL_RANGE 1000000
float get_pixel(ivec2 pos) {
    int int_val = texelFetch(u_landscape, pos, 0).x;
    return float(int_val) / float(INT_VAL_RANGE);
}

vec4 get_world_color(vec2 world_coords) {
    bool in_range = world_coords.x >= 0 && world_coords.y >= 0 && world_coords.x <= u_world_size.x && world_coords.y <= u_world_size.y;

    //uint terrain_type = texelFetch(u_world_texture, ivec2(world_coords), 0).x;
    int i_height = texelFetch(u_landscape, ivec2(world_coords), 0).x;
    float height = float(i_height) / 1000000.0;
    vec3 color;

    float water_level = 0.43;
    float beach_level = water_level + 0.03;
    float mountain_level = 0.9;
    float snow_level = 0.99;

    if (u_render_type == uint(0)) {
        color = vec3(height);
    } else if (u_render_type == uint(1)) {
        if (height <= water_level) {
            color = vec3(height / 3.0, height / 2.0,  0.6);
        } else if (height <= beach_level) {
            color = getTerrainColor(Terrain_Beach) + height - 0.4;
        } else if (height <= mountain_level) {
            color = getTerrainColor(Terrain_Plains) - (height - 0.45) / 2.0;
        } else {
            color = getTerrainColor(Terrain_Mountains) - (height - mountain_level)/3.0;
        }
    } else if (u_render_type == uint(2)) {
        float steepness = length(CalculateHeightAndGradient(ivec2(world_coords)).gradient);

        /*if (height <= 0.4) {
            color = vec3(height / 3.0, height / 2.0,  0.6);
        }*/
        if (height <= water_level) {
            color = vec3(height / 3.0, height / 2.0,  0.6);
        } else if (height <= beach_level) {
            color = getTerrainColor(Terrain_Beach) + height - 0.4;
        } else {
            vec3 grass = getTerrainColor(Terrain_Plains);
            vec3 mud = vec3(51.0, 25.0, 0.0) / vec3(255.0);
            vec3 mountain = getTerrainColor(Terrain_Mountains);

            float full_mud_steepness = 0.16;
            steepness = steepness / full_mud_steepness;

            if (steepness <= 1.0) {
                color = steepness * mud + (1.0 - steepness) * grass;
            } else {
                float mountain_steepness = (steepness - 1.0) / 2.0;
                color = mountain_steepness * mountain + (1.0 - mountain_steepness) * mud;
            }
            if (height >= mountain_level) {
                color = getTerrainColor(Terrain_Mountains) + height - 0.9;
            }
        }
    } else {
        vec2 grad = CalculateHeightAndGradient(world_coords).gradient;
        color = vec3(abs(grad) * 8.0, 0.0) * vec3(height);
    }

    return in_range ? vec4(color, 1.0) : vec4(0.0, 0.0, 0.0, 1.0);
}

float cube(float x) {
    return x * x * x;
}

float interp1(float x) {
    return 1. - abs(cube(x * 2. - 1.));
}
