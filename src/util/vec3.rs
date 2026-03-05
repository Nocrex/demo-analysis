use serde::{Deserialize, Serialize};
use tf_demo_parser::demo::vector::Vector;

#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Vec3 {
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0);

    pub const fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn dot(&self, rhs: &Self) -> f32 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn angle_between(&self, rhs: &Self) -> f32 {
        self.dot(rhs).acos().to_degrees()
    }
}

impl From<Vector> for Vec3 {
    fn from(value: Vector) -> Self {
        Self::new(value.x, value.y, value.z)
    }
}
