#define Terrain_Ocean            0
#define Terrain_Shallow          1
#define Terrain_Beach            2
#define Terrain_Swamp            3
#define Terrain_Lowland          4
#define Terrain_Plains           5
#define Terrain_Grassland        6
#define Terrain_Hills            7
#define Terrain_Desert           8
#define Terrain_Foothills        9
#define Terrain_Mountains        10
#define Terrain_SnowyMountains   11

vec3 getTerrainColor(int terrain) {
    switch (terrain) {
        case Terrain_Ocean:             return vec3(0.0,   0.0,   102.0) / vec3(255.0);
        case Terrain_Shallow:           return vec3(51.0,  102.0, 255.0) / vec3(255.0);
        case Terrain_Beach:             return vec3(150.0, 110.0, 0.0  ) / vec3(255.0);
        case Terrain_Swamp:             return vec3(51.0,  51.0,  0.0  ) / vec3(255.0);
        case Terrain_Lowland:           return vec3(0.0,   153.0, 51.0 ) / vec3(255.0);
        case Terrain_Plains:            return vec3(0.0,   102.0, 0.0  ) / vec3(255.0);
        case Terrain_Grassland:         return vec3(0.0,   204.0, 0.0  ) / vec3(255.0);
        case Terrain_Hills:             return vec3(102.0, 153.0, 0.0  ) / vec3(255.0);
        case Terrain_Desert:            return vec3(204.0, 153.0, 0.0  ) / vec3(255.0);
        case Terrain_Foothills:         return vec3(153.0, 255.0, 153.0) / vec3(255.0);
        case Terrain_Mountains:         return vec3(133.0, 133.0, 133.0) / vec3(255.0);
        case Terrain_SnowyMountains:    return vec3(255.0, 255.0, 255.0) / vec3(255.0);
    }
    return vec3(0.0);
}