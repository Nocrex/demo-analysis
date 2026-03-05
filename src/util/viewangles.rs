use core::f32;
use std::ops::{Div, Sub};

use serde::{Deserialize, Serialize};

use crate::util::Vec3;


#[derive(Debug, Default, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct Viewangles {
    pub pitch: f32,
    pub yaw: f32,
}

impl Viewangles {
    pub const NAN: Self = Self::new(f32::NAN, f32::NAN);

    pub const fn new(pitch: f32, yaw: f32) -> Self {
        Self { pitch, yaw }
    }

    // Compute the difference in viewangles. We have to account for the fact viewangles are in a circle.
    // E.g. If viewangle goes from 350 to 10 degrees, we want to return 20 degrees.
    pub fn component_delta(&self, prev: &Self, tick_delta: u32) -> Self {
        let tick_delta = if tick_delta < 1 { 1 } else { tick_delta };
        (self - prev) / tick_delta as f32
    }

    // Returns the euclidean 2D magnitude
    pub fn mag(&self) -> f32 {
        (self.pitch.powi(2) + self.yaw.powi(2)).sqrt()
    }

    // Returns the angle between self and other on a sphere
    pub fn angle(&self, other: &Self) -> f32 {
        self.to_unit_vec().angle_between(&other.to_unit_vec())
    }

    pub fn to_unit_vec(&self) -> Vec3 {
        let yaw = self.yaw.to_radians();
        let pitch = self.pitch.to_radians();
        Vec3::new(
            pitch.cos() * yaw.sin(),
            pitch.sin(),
            pitch.cos() * yaw.cos(),
        )
    }
}

impl Sub for &Viewangles {
    type Output = Viewangles;

    fn sub(self, rhs: Self) -> Self::Output {
        let yaw_delta = {
            let diff = (self.yaw - rhs.yaw).rem_euclid(360.0);
            if diff > 180.0 {
                diff - 360.0
            } else {
                diff
            }
        };
        Viewangles::new(self.pitch - rhs.pitch, yaw_delta)
    }
}

impl Div<f32> for Viewangles {
    type Output = Viewangles;

    fn div(self, rhs: f32) -> Self::Output {
        Viewangles::new(self.pitch / rhs, self.yaw / rhs)
    }
}