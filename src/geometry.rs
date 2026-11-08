//! 2D points and the small bits of maths every shape shares.

use std::ops::{Add, Div, Mul, Sub};

/// The angle between successive seeds in a sunflower head. Stepping around a circle by
/// this amount never lines up with earlier steps, which is why nature packs seeds with it.
pub const GOLDEN_ANGLE: f32 = 2.399_963_3;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Unit vector pointing along `angle` (radians).
    pub fn from_angle(angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::new(cos, sin)
    }

    pub fn length(self) -> f32 {
        self.x.hypot(self.y)
    }

    pub fn dot(self, other: Self) -> f32 {
        self.x * other.x + self.y * other.y
    }

    pub fn cross(self, other: Self) -> f32 {
        self.x * other.y - self.y * other.x
    }

    pub fn rotate(self, angle: f32) -> Self {
        let (sin, cos) = angle.sin_cos();
        Self::new(self.x * cos - self.y * sin, self.x * sin + self.y * cos)
    }

    pub fn lerp(self, other: Self, t: f32) -> Self {
        self + (other - self) * t
    }
}

impl Add for Point {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self::new(self.x + other.x, self.y + other.y)
    }
}

impl Sub for Point {
    type Output = Self;

    fn sub(self, other: Self) -> Self {
        Self::new(self.x - other.x, self.y - other.y)
    }
}

impl Mul<f32> for Point {
    type Output = Self;

    fn mul(self, factor: f32) -> Self {
        Self::new(self.x * factor, self.y * factor)
    }
}

impl Div<f32> for Point {
    type Output = Self;

    fn div(self, divisor: f32) -> Self {
        Self::new(self.x / divisor, self.y / divisor)
    }
}

/// Axis-aligned rectangle that limits how much of the canvas a shape has to visit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub min: Point,
    pub max: Point,
}

impl Bounds {
    pub fn around(center: Point, half_width: f32, half_height: f32) -> Self {
        let half = Point::new(half_width, half_height);
        Self {
            min: center - half,
            max: center + half,
        }
    }

    pub fn enclosing(points: &[Point]) -> Self {
        let empty = Self {
            min: Point::new(f32::INFINITY, f32::INFINITY),
            max: Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY),
        };
        points.iter().fold(empty, |bounds, point| Self {
            min: Point::new(bounds.min.x.min(point.x), bounds.min.y.min(point.y)),
            max: Point::new(bounds.max.x.max(point.x), bounds.max.y.max(point.y)),
        })
    }

    pub fn expand(self, margin: f32) -> Self {
        let margin = Point::new(margin, margin);
        Self {
            min: self.min - margin,
            max: self.max + margin,
        }
    }
}

/// Eases from 0 at `edge0` to 1 at `edge1`. The edges may be given in either order,
/// which makes fading *out* toward an edge read naturally.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
