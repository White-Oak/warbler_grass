use bevy::{math::Vec3Swizzles, prelude::*, render::primitives::Aabb};

use crate::grass::GrassBlade;
use crate::height_map::HeightMap;
#[derive(Default, Component, Clone)]
pub struct GrassSpawner {
    pub(crate) positions_xz: Vec<Vec2>,
    pub(crate) positions_y: Vec<f32>,
    pub(crate) heights: HeightRepresentation,
    pub(crate) height_map: Option<HeightMap>,
    _density_map: Option<Handle<Image>>,
    pub(crate) flags: GrassSpawnerFlags,
}

impl GrassSpawner {
    pub fn new() -> GrassSpawner {
        Self::default()
    }

    /// Defines the positions of all grass blades.
    pub fn with_positions(mut self, positions: Vec<Vec3>) -> GrassSpawner {
        assert!(!positions.is_empty());
        let (positions_xz, positions_y) = positions
            .into_iter()
            .map(|position| (position.xz(), position.y))
            .unzip();
        self = self.with_positions_xz(positions_xz);
        self = self.with_positions_y(positions_y);

        self.validate();
        self
    }
    /// Defines the xz positions of all grass blades.
    ///
    /// ## Note:
    /// If you just want to define all world positions of all grass blades,
    /// consider using [`Self::with_positions`] instead.
    /// Just defining the xz positions allows you to also insert a heightmap
    pub fn with_positions_xz(mut self, positions_xz: Vec<Vec2>) -> GrassSpawner {
        assert!(!positions_xz.is_empty());

        if self.flags.contains(GrassSpawnerFlags::XZ_DEFINED) {
            panic!("Can not insert positions_xz to `GrassSpawner` since the xz positions are already defined");
        }

        self.flags.insert(GrassSpawnerFlags::XZ_DEFINED);

        self.positions_xz = positions_xz;

        self.validate();
        self
    }
    /// Defines the y position of all grass blades.
    ///
    /// You can only use this function or use a heightmap.
    /// Both at the same time are not supported
    pub fn with_positions_y(mut self, positions_y: Vec<f32>) -> GrassSpawner {
        assert!(!positions_y.is_empty());

        if self.flags.contains(GrassSpawnerFlags::Y_DEFINED) {
            panic!("Can not insert positions_y to `GrassSpawner` since the y positions are already defined");
        }

        self.flags.insert(GrassSpawnerFlags::Y_DEFINED);

        self.positions_y = positions_y;

        self.validate();
        self
    }
    /// Defines the height of each grass blade.
    pub fn with_heights(mut self, heights: Vec<f32>) -> GrassSpawner {
        assert!(!heights.is_empty());
        assert!(heights.iter().all(|height| *height > 0.));
        self.flags.insert(GrassSpawnerFlags::HEIGHT_DEFINED);

        self.heights = HeightRepresentation::PerBlade(heights);

        self.validate();
        self
    }
    /// Defines the height of all grass blades.
    ///
    /// Every blade will have the same height
    pub fn with_height_uniform(mut self, uniform_height: f32) -> GrassSpawner {
        assert!(uniform_height > 0.);
        self.flags.insert(GrassSpawnerFlags::HEIGHT_DEFINED);
        self.heights = HeightRepresentation::Uniform(uniform_height);
        self
    }
    /// Defines you height map for loading the y positions of your grass
    ///
    /// Note that the heightmap texture gets stretched over the minimal [Aabb] containing all defined grass blades.
    pub fn with_height_map(mut self, height_map: HeightMap) -> GrassSpawner {
        if self.flags.contains(GrassSpawnerFlags::Y_DEFINED) {
            panic!("Can not insert height map to `GrassSpawner` since the y positions are already defined");
        }

        self.flags.insert(GrassSpawnerFlags::Y_DEFINED);
        self.flags.insert(GrassSpawnerFlags::HEIGHT_MAP);

        self.height_map = Some(height_map);
        self
    }
    /// Defines the [`GrassSpawner`] from [`GrassBlade`]s
    pub fn from_grass_blades(mut self, grass_blades: Vec<GrassBlade>) -> GrassSpawner {
        assert!(!grass_blades.is_empty());
        let (positions, heights) = grass_blades
            .into_iter()
            .map(|blade| (blade.position, blade.height))
            .unzip();
        self = self.with_positions(positions);
        self = self.with_heights(heights);

        self.validate();
        self
    }
    fn validate(&self) {
        if !self.positions_xz.is_empty() && !self.positions_y.is_empty() {
            assert_eq!(self.positions_xz.len(), self.positions_y.len());
        }
        if let HeightRepresentation::PerBlade(heights) = &self.heights {
            if !self.positions_y.is_empty() {
                assert_eq!(heights.len(), self.positions_y.len());
            }
            if !self.positions_xz.is_empty() {
                assert_eq!(heights.len(), self.positions_xz.len());
            }
        }
    }
    pub fn calculate_aabb(&self) -> Aabb {
        let mut outer = Vec3::new(f32::MIN, f32::MIN, f32::MIN);
        let mut inner = Vec3::new(f32::MAX, f32::MAX, f32::MAX);
        if self.flags.contains(GrassSpawnerFlags::HEIGHT_MAP) {
            let height = self.height_map.as_ref().unwrap().height;
            self.positions_xz.iter().for_each(|xz| {
                let blade_pos = Vec3::new(xz.x, 0., xz.y);
                inner = inner.min(blade_pos);
                outer = outer.max(blade_pos + Vec3::Y * height);
            });
        } else {
            self.positions_xz
                .iter()
                .zip(self.positions_y.iter())
                .for_each(|(xz, y)| {
                    let blade_pos = Vec3::new(xz.x, *y, xz.y);
                    let height = 1.;
                    inner = inner.min(blade_pos);
                    outer = outer.max(blade_pos + Vec3::Y * height);
                });
        }
        Aabb::from_min_max(inner, outer)
    }
}
bitflags::bitflags! {
    #[repr(transparent)]
    pub struct GrassSpawnerFlags: u32 {
        const Y_DEFINED      = (1 << 0);
        const XZ_DEFINED     = (1 << 1);
        const HEIGHT_DEFINED = (1 << 2);
        const HEIGHT_MAP     = (1 << 3);
        const DENSITY_MAP    = (1 << 4);
        const NONE           = 0;
        const UNINITIALIZED  = 0xFFFF;
    }
}
impl Default for GrassSpawnerFlags {
    fn default() -> Self {
        Self::NONE
    }
}
#[derive(Clone)]
pub enum HeightRepresentation {
    PerBlade(Vec<f32>),
    Uniform(f32),
}
impl Default for HeightRepresentation {
    fn default() -> Self {
        HeightRepresentation::Uniform(1.)
    }
}
pub(crate) fn add_aabb_box_to_grass(
    mut commands: Commands,
    grasses: Query<(Entity, &GrassSpawner), Without<Aabb>>,
) {
    for (e, spawner) in grasses.iter() {
        let aabb = spawner.calculate_aabb();
        commands.entity(e).insert(aabb);
    }
}
