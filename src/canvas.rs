//! A supersampled drawing surface that shapes paint into with per-pixel shaders.

use crate::color::Rgb;
use crate::geometry::{Bounds, Point};

/// Share of a pixel's samples that must be painted for the pixel to show. Anything less
/// is left to the terminal's own background, which we can't see and so can't blend with.
const COVERAGE_THRESHOLD: f32 = 0.5;

/// Samples each output pixel on a `samples` × `samples` grid, so the edges between
/// shapes come out anti-aliased once the canvas is resolved.
pub struct Canvas {
    width: usize,
    height: usize,
    samples: usize,
    subpixels: Vec<Option<Rgb>>,
}

impl Canvas {
    pub fn new(width: usize, height: usize, samples: usize) -> Self {
        Self {
            width,
            height,
            samples,
            subpixels: vec![None; width * samples * height * samples],
        }
    }

    /// The whole drawing surface, in output pixels.
    pub fn bounds(&self) -> Bounds {
        Bounds {
            min: Point::new(0.0, 0.0),
            max: Point::new(self.width as f32, self.height as f32),
        }
    }

    /// Paints over every sample inside `bounds` that `shader` returns a colour for.
    /// Coordinates are in output pixels with y pointing down.
    pub fn paint(&mut self, bounds: Bounds, shader: impl Fn(Point) -> Option<Rgb>) {
        self.visit(bounds, |point, sample| {
            if let Some(color) = shader(point) {
                *sample = Some(color);
            }
        });
    }

    /// Like [`paint`](Self::paint), but `shader` also returns an opacity (0–1) and the
    /// colour is mixed into whatever is already there.
    pub fn blend(&mut self, bounds: Bounds, shader: impl Fn(Point) -> Option<(Rgb, f32)>) {
        self.visit(bounds, |point, sample| {
            let Some((color, opacity)) = shader(point) else {
                return;
            };
            match sample {
                Some(beneath) => *beneath = beneath.mix(color, opacity.clamp(0.0, 1.0)),
                // The terminal's background can't be blended with, so a sample over bare
                // canvas is either solid or absent, by the same rule that resolves pixels.
                None if opacity >= COVERAGE_THRESHOLD => *sample = Some(color),
                None => {}
            }
        });
    }

    fn visit(&mut self, bounds: Bounds, mut each: impl FnMut(Point, &mut Option<Rgb>)) {
        let scale = self.samples as f32;
        let columns = self.width * self.samples;
        let rows = self.height * self.samples;
        let (x_start, x_end) = sample_span(bounds.min.x, bounds.max.x, scale, columns);
        let (y_start, y_end) = sample_span(bounds.min.y, bounds.max.y, scale, rows);
        for sy in y_start..y_end {
            for sx in x_start..x_end {
                let point = Point::new((sx as f32 + 0.5) / scale, (sy as f32 + 0.5) / scale);
                each(point, &mut self.subpixels[sy * columns + sx]);
            }
        }
    }

    /// Averages the samples down to output resolution.
    pub fn resolve(&self) -> Image {
        let columns = self.width * self.samples;
        let samples_per_pixel = self.samples * self.samples;
        let mut pixels = Vec::with_capacity(self.width * self.height);
        for y in 0..self.height {
            for x in 0..self.width {
                let mut sum = Rgb::BLACK;
                let mut covered = 0;
                for sy in y * self.samples..(y + 1) * self.samples {
                    for sx in x * self.samples..(x + 1) * self.samples {
                        if let Some(color) = self.subpixels[sy * columns + sx] {
                            sum = sum + color;
                            covered += 1;
                        }
                    }
                }
                let coverage = covered as f32 / samples_per_pixel as f32;
                pixels.push(
                    (covered > 0 && coverage >= COVERAGE_THRESHOLD)
                        .then(|| sum.scale(1.0 / covered as f32)),
                );
            }
        }
        Image {
            width: self.width,
            height: self.height,
            pixels,
        }
    }
}

/// Sample indices covering `[min, max)` in pixel units, clipped to the canvas.
fn sample_span(min: f32, max: f32, scale: f32, limit: usize) -> (usize, usize) {
    let start = (min * scale).floor().clamp(0.0, limit as f32) as usize;
    let end = (max * scale).ceil().clamp(0.0, limit as f32) as usize;
    (start, end.max(start))
}

/// A finished picture; `None` pixels are transparent.
pub struct Image {
    width: usize,
    height: usize,
    pixels: Vec<Option<Rgb>>,
}

impl Image {
    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn pixel(&self, x: usize, y: usize) -> Option<Rgb> {
        self.pixels[y * self.width + x]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const RED: Rgb = Rgb::new(1.0, 0.0, 0.0);
    const BLUE: Rgb = Rgb::new(0.0, 0.0, 1.0);

    #[test]
    fn pixels_need_majority_coverage_to_show() {
        let mut canvas = Canvas::new(2, 1, 4);
        // Fully covers pixel 0 and a quarter of pixel 1.
        canvas.paint(
            Bounds {
                min: Point::new(0.0, 0.0),
                max: Point::new(1.25, 1.0),
            },
            |_| Some(RED),
        );
        let image = canvas.resolve();
        assert_eq!(image.pixel(0, 0), Some(RED));
        assert_eq!(image.pixel(1, 0), None);
    }

    #[test]
    fn edges_between_shapes_are_blended() {
        let mut canvas = Canvas::new(1, 1, 2);
        let everywhere = Bounds::around(Point::new(0.5, 0.5), 1.0, 1.0);
        canvas.paint(everywhere, |_| Some(RED));
        canvas.paint(everywhere, |point| (point.x > 0.5).then_some(BLUE));
        assert_eq!(canvas.resolve().pixel(0, 0), Some(Rgb::new(0.5, 0.0, 0.5)));
    }

    #[test]
    fn blending_mixes_into_what_is_already_painted() {
        let mut canvas = Canvas::new(1, 1, 1);
        canvas.paint(canvas.bounds(), |_| Some(RED));
        canvas.blend(canvas.bounds(), |_| Some((BLUE, 0.25)));
        assert_eq!(
            canvas.resolve().pixel(0, 0),
            Some(Rgb::new(0.75, 0.0, 0.25))
        );
    }

    #[test]
    fn blending_over_bare_canvas_is_solid_or_absent() {
        let mut canvas = Canvas::new(2, 1, 1);
        canvas.blend(canvas.bounds(), |point| {
            Some((BLUE, if point.x < 1.0 { 0.8 } else { 0.3 }))
        });
        let image = canvas.resolve();
        assert_eq!(image.pixel(0, 0), Some(BLUE));
        assert_eq!(image.pixel(1, 0), None);
    }

    #[test]
    fn painting_off_canvas_is_clipped() {
        let mut canvas = Canvas::new(2, 2, 2);
        canvas.paint(Bounds::around(Point::new(-5.0, -5.0), 1.0, 1.0), |_| {
            Some(RED)
        });
        canvas.paint(Bounds::around(Point::new(50.0, 50.0), 1.0, 1.0), |_| {
            Some(RED)
        });
        let image = canvas.resolve();
        assert!((0..2).all(|y| (0..2).all(|x| image.pixel(x, y).is_none())));
    }
}
