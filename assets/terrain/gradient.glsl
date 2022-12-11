#if defined(GRADIENT_GLSL__GET_PIXEL) && defined(GRADIENT_GLSL__MAP_WIDTH) && defined(GRADIENT_GLSL__MAP_HEIGHT)

struct HeightAndGradient {
    float height;
    vec2 gradient;
};

HeightAndGradient CalculateHeightAndGradient(vec2 pos) {
    ivec2 coord = ivec2(pos);

    // Calculate droplet's offset inside the cell (0,0) = at NW node, (1,1) = at SE node
    float x = fract(pos.x);
    float y = fract(pos.y);

    ivec2 bounds = ivec2(GRADIENT_GLSL__MAP_WIDTH, GRADIENT_GLSL__MAP_HEIGHT) - 1;
    // Calculate heights of the four nodes of the droplet's cell
    float height_nx_ny = GRADIENT_GLSL__GET_PIXEL( min(coord + ivec2(0, 0), bounds) );
    float height_px_ny = GRADIENT_GLSL__GET_PIXEL( min(coord + ivec2(1, 0), bounds) );
    float height_nx_py = GRADIENT_GLSL__GET_PIXEL( min(coord + ivec2(0, 1), bounds) );
    float height_px_py = GRADIENT_GLSL__GET_PIXEL( min(coord + ivec2(1, 1), bounds) );

    // Calculate droplet's direction of flow with bilinear interpolation of height difference along the edges
    float dx = (height_px_ny - height_nx_ny) * (1.0 - y) + (height_px_py - height_nx_py) * y;
    float dy = (height_nx_py - height_nx_ny) * (1.0 - x) + (height_px_py - height_px_ny) * x;

    // Calculate height with bilinear interpolation of the heights of the nodes of the cell
    float height = height_nx_ny * (1.0 - x) * (1.0 - y) + height_px_ny * x * (1.0 - y) + height_nx_py * (1.0 - x) * y + height_px_py * x * y;

    return HeightAndGradient(height, vec2(dx, dy));
}

#endif
