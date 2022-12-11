#ifdef BRUSH_GLSL__MAP_WIDTH
#ifdef BRUSH_GLSL__MAP_HEIGHT

#ifndef BRUSH_GLSL__DATA_BINDING
    #define BRUSH_GLSL__DATA_BINDING 1
#endif
#ifndef BRUSH_GLSL__PTRS_BINDING
    #define BRUSH_GLSL__PTRS_BINDING 2
#endif

struct BrushIndex {
    int cell_x;
    int cell_y;
    float weight;
};

struct BrushPointer {
    int start_index;
    int array_length;
};

// 2D array
layout(std430, binding = BRUSH_GLSL__DATA_BINDING) buffer buf_erosion_brush_data {
    BrushIndex brush_indices[];
};
layout(std430, binding = BRUSH_GLSL__PTRS_BINDING) buffer buf_erosion_brush_pointers {
    BrushPointer brush_pointers[];
};

int get_brush_indices_count(ivec2 point) {
    if (point.x < 0 || point.y < 0 ||
        point.x >= int(BRUSH_GLSL__MAP_WIDTH) ||
        point.y >= int(BRUSH_GLSL__MAP_HEIGHT)) {
        return 0;
    } else {
        int id = point.y * int(BRUSH_GLSL__MAP_WIDTH) + point.x;
        return brush_pointers[id].array_length;
    }
}

BrushIndex get_brush_index(ivec2 point, int id) {
    if (get_brush_indices_count(point) <= id) {
        return BrushIndex(-1, -1, 0.0);
    } else {
        int ptr_id = point.y * (BRUSH_GLSL__MAP_WIDTH) + point.x;
        int start_index = brush_pointers[ptr_id].start_index;
        return brush_indices[start_index + id];
    }
}

#endif
#endif
