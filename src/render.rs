use crate::config::{Color, Crosshair, Outline, Shape};

const SAMPLES: usize = 4;

type Rgba = [f32; 4];

pub struct Image {
	pub size: i32,
	pub pixels: Vec<u32>,
}

impl Image {
	pub fn visible_size(&self) -> (i32, i32) {
		let visible =
			|x: i32, y: i32| self.pixels[(y * self.size + x) as usize] >> 24 != 0;
		let span = |hit: &dyn Fn(i32) -> bool| {
			let hits: Vec<i32> = (0..self.size).filter(|&i| hit(i)).collect();
			hits.last().map_or(0, |last| last - hits[0] + 1)
		};
		let width = span(&|x| (0..self.size).any(|y| visible(x, y)));
		let height = span(&|y| (0..self.size).any(|x| visible(x, y)));
		(width, height)
	}
}

pub fn render(crosshair: &Crosshair, scale: f32) -> Image {
	let layers: Vec<Layer> =
		[bars(crosshair, scale), dot(crosshair, scale)].into_iter().flatten().collect();
	let width = if crosshair.bars.any() {
		crosshair.bars.thickness
	} else {
		crosshair.dot.size
	};
	let odd = scaled(width, scale) as i32 % 2;
	let reach = layers.iter().map(Layer::reach).fold(0.0, f32::max);
	let size = 2 * (reach.ceil() as i32 + 1) + odd;
	let center = size as f32 / 2.0;
	let pixels = (0..size * size)
		.map(|i| pixel(&layers, (i % size) as f32 - center, (i / size) as f32 - center))
		.collect();
	Image { size, pixels }
}

fn bars(crosshair: &Crosshair, scale: f32) -> Option<Layer> {
	let bars = &crosshair.bars;
	let half_length = scaled(bars.length, scale) / 2.0;
	let half_thickness = scaled(bars.thickness, scale) / 2.0;
	let middle = scaled(bars.gap, scale) + half_length;
	let radius = match bars.shape {
		Shape::Rounded => half_thickness.min(half_length),
		Shape::Sharp => 0.0,
	};
	let vertical = |y| RoundedRect {
		x: 0.0,
		y,
		half_width: half_thickness,
		half_height: half_length,
		radius,
	};
	let horizontal = |x| RoundedRect {
		x,
		y: 0.0,
		half_width: half_length,
		half_height: half_thickness,
		radius,
	};
	let shapes: Vec<RoundedRect> = [
		(bars.top, vertical(-middle)),
		(bars.bottom, vertical(middle)),
		(bars.left, horizontal(-middle)),
		(bars.right, horizontal(middle)),
	]
	.into_iter()
	.filter_map(|(on, shape)| on.then_some(shape))
	.collect();
	(!shapes.is_empty())
		.then(|| Layer::new(shapes, bars.color, bars.opacity, &bars.outline, scale))
}

fn dot(crosshair: &Crosshair, scale: f32) -> Option<Layer> {
	let dot = &crosshair.dot;
	let radius = scaled(dot.size, scale) / 2.0;
	let corner = match dot.shape {
		Shape::Rounded => radius,
		Shape::Sharp => 0.0,
	};
	let shape = RoundedRect {
		x: 0.0,
		y: 0.0,
		half_width: radius,
		half_height: radius,
		radius: corner,
	};
	dot.enabled
		.then(|| Layer::new(vec![shape], dot.color, dot.opacity, &dot.outline, scale))
}

fn scaled(pixels: i32, scale: f32) -> f32 {
	(pixels as f32 * scale).round()
}

fn pixel(layers: &[Layer], x: f32, y: f32) -> u32 {
	let mut sum = [0.0; 4];
	for i in 0..SAMPLES * SAMPLES {
		let sample_x = x + ((i % SAMPLES) as f32 + 0.5) / SAMPLES as f32;
		let sample_y = y + ((i / SAMPLES) as f32 + 0.5) / SAMPLES as f32;
		let color = layers.iter().fold([0.0; 4], |below, layer| {
			over(layer.sample(sample_x, sample_y), below)
		});
		for (total, channel) in sum.iter_mut().zip(color) {
			*total += channel;
		}
	}
	let [r, g, b, a] =
		sum.map(|total| (total / (SAMPLES * SAMPLES) as f32 * 255.0).round() as u32);
	a << 24 | r << 16 | g << 8 | b
}

fn over(top: Rgba, below: Rgba) -> Rgba {
	let rest = 1.0 - top[3];
	std::array::from_fn(|i| top[i] + below[i] * rest)
}

fn premultiply(color: Color, opacity: f32) -> Rgba {
	let channel = |value: u8| value as f32 / 255.0 * opacity;
	[channel(color.r), channel(color.g), channel(color.b), opacity]
}

struct Layer {
	shapes: Vec<RoundedRect>,
	fill: Rgba,
	outline: Option<(Rgba, f32)>,
}

impl Layer {
	fn new(
		shapes: Vec<RoundedRect>,
		color: Color,
		opacity: f32,
		outline: &Outline,
		scale: f32,
	) -> Self {
		Self {
			shapes,
			fill: premultiply(color, opacity),
			outline: outline.enabled.then(|| {
				(
					premultiply(outline.color, outline.opacity),
					scaled(outline.thickness, scale),
				)
			}),
		}
	}

	fn sample(&self, x: f32, y: f32) -> Rgba {
		let distance = self
			.shapes
			.iter()
			.map(|shape| shape.distance(x, y))
			.fold(f32::INFINITY, f32::min);
		match self.outline {
			_ if distance <= 0.0 => self.fill,
			Some((color, thickness)) if distance <= thickness => color,
			_ => [0.0; 4],
		}
	}

	fn reach(&self) -> f32 {
		let outline = self.outline.map_or(0.0, |(_, thickness)| thickness);
		let shapes = self
			.shapes
			.iter()
			.map(|s| (s.x.abs() + s.half_width).max(s.y.abs() + s.half_height))
			.fold(0.0, f32::max);
		shapes + outline
	}
}

struct RoundedRect {
	x: f32,
	y: f32,
	half_width: f32,
	half_height: f32,
	radius: f32,
}

impl RoundedRect {
	fn distance(&self, x: f32, y: f32) -> f32 {
		let dx = (x - self.x).abs() - (self.half_width - self.radius);
		let dy = (y - self.y).abs() - (self.half_height - self.radius);
		if self.radius == 0.0 {
			return dx.max(dy);
		}
		dx.max(0.0).hypot(dy.max(0.0)) + dx.max(dy).min(0.0) - self.radius
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::config::Bars;

	fn without_bars() -> Crosshair {
		let mut crosshair = Crosshair::default();
		crosshair.bars = Bars {
			top: false,
			bottom: false,
			left: false,
			right: false,
			..crosshair.bars
		};
		crosshair
	}

	#[test]
	fn bars_reach_past_gap_length_and_outline_on_each_side() {
		let crosshair = Crosshair::default();
		let Bars { length, thickness, gap, outline, .. } = crosshair.bars;
		let reach = gap + length + outline.thickness;
		assert_eq!(thickness % 2, 1, "an odd thickness centers on a pixel");
		assert_eq!(
			render(&crosshair, 1.0).visible_size(),
			(2 * reach + 1, 2 * reach + 1)
		);
		assert_eq!(render(&crosshair, 2.0).visible_size(), (4 * reach, 4 * reach));
	}

	#[test]
	fn a_lone_dot_spans_its_size_and_outline() {
		let crosshair = without_bars();
		let expected = crosshair.dot.size + 2 * crosshair.dot.outline.thickness;
		assert_eq!(render(&crosshair, 1.0).visible_size(), (expected, expected));
	}

	#[test]
	fn hidden_bars_leave_their_axis() {
		let mut crosshair = Crosshair::default();
		crosshair.bars.left = false;
		crosshair.bars.right = false;
		let (width, height) = render(&crosshair, 1.0).visible_size();
		assert!(width < height);
	}

	#[test]
	fn a_hidden_crosshair_draws_nothing() {
		let mut crosshair = without_bars();
		crosshair.dot.enabled = false;
		let image = render(&crosshair, 1.5);
		assert!(image.pixels.iter().all(|&pixel| pixel == 0));
		assert_eq!(image.visible_size(), (0, 0));
	}
}
