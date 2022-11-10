#version 330

in vec2 f_tex_coords;
out vec4 out_color;

uniform uvec2 u_world_size;
uniform vec2 u_screen_size;
uniform vec2 u_camera_pos;
uniform float u_camera_zoom;

uniform sampler2D u_world_texture;

vec4 get_world_color(vec2 world_pos);

void main() {
    vec2 frag_pos_px = f_tex_coords * u_screen_size;

    vec2 world_coords = (frag_pos_px + u_camera_pos * pow(2.0, u_camera_zoom)) / pow(2.0, u_camera_zoom);

    out_color = get_world_color(world_coords);
}

vec4 get_world_color(vec2 world_coords) {
    bool in_range = world_coords.x >= 0 && world_coords.y >= 0 && world_coords.x <= float(u_world_size.x) && world_coords.y <= float(u_world_size.y);

    vec4 texel = texelFetch(u_world_texture, ivec2(world_coords), 0);

    return in_range ? vec4(texel.xyz, 1.0) : vec4(0.0, 0.0, 0.0, 1.0);
}

