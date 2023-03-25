use fixed::{FixedI32, types::extra::U20};

pub const MAX_MAP_SIZE: (u32, u32) = (16384, 16384);

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Biome {
    Sea,
    Shoal,
    Beach,
    Plains,
    Swamp,
    Forest,
    Desert,
    Scree,
    Mountain,
    SnowyMountain,
}
impl Biome {
    pub fn all() -> &'static [Self] {
        use Biome::*;
        &[
            Sea, Shoal, Beach, Plains,
            Swamp, Forest, Desert, Scree,
            Mountain, SnowyMountain,
        ]
    }

    pub fn localized_name(&self) -> &'static str {
        match self {
            Biome::Sea => "Sea",
            Biome::Shoal => "Shoal",
            Biome::Beach => "Beach",
            Biome::Plains => "Plains",
            Biome::Swamp => "Swamp",
            Biome::Forest => "Forest",
            Biome::Desert => "Desert",
            Biome::Scree => "Scree",
            Biome::Mountain => "Mountain",
            Biome::SnowyMountain => "Snowy mountain",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Resource {
    Compound { count: u8 },
    Rock,
    Fruit,
    PoisonedFruit,
    FireCrystal,
    EnigmaticCrystal,
}

const MAX_RESOURCES_IN_CELL: usize = 4;
type MapCell = (Biome, [Option<Resource>; MAX_RESOURCES_IN_CELL]);

#[derive(Debug, Clone)]
pub struct Map {
    size: (u32, u32),
    biomes: Box<[MapCell]>,
}
impl Map {
    pub fn new(size: (u32, u32), biomes: Box<[MapCell]>) -> Self {
        assert!(biomes.len() == (size.0 as usize) * (size.1 as usize));
        Self { size, biomes }
    }

    pub fn disassemble(self) -> ((u32, u32), Box<[MapCell]>) {
        (self.size, self.biomes)
    }
}

#[derive(Debug, Clone)]
pub struct Landscape {
    size: (u32, u32),
    height: Box<[FixedI32<U20>]>,
}
impl Landscape {
    pub fn new(size: (u32, u32), height: Box<[FixedI32<U20>]>) -> Self {
        assert!(height.len() == (size.0 as usize) * (size.1 as usize));
        Self { size, height }
    }

    pub fn disassemble(self) -> ((u32, u32), Box<[FixedI32<U20>]>) {
        (self.size, self.height)
    }
}
