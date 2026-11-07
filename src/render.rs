use crate::config::{Color, Crosshair, Outline, Shape};

const SAMPLES_PER_AXIS: usize = 4;

type PremultipliedRgba = [f32; 4];

pub struct CrosshairImage {
	pub side_length: i32,
	pub pixels: Vec<u32>,
}

impl CrosshairImage {
	pub fn visible_bounds_size(&self) -> (i32, i32) {
		let is_pixel_visible = |column: i32, row: i32| {
			self.pixels[(row * self.side_length + column) as usize] >> 24 != 0
		};
		let visible_span = |is_line_visible: &dyn Fn(i32) -> bool| {
			let visible_lines: Vec<i32> =
				(0..self.side_length).filter(|&line| is_line_visible(line)).collect();
			visible_lines
				.last()
				.map_or(0, |last_visible_line| last_visible_line - visible_lines[0] + 1)
		};
		let width = visible_span(&|column| {
			(0..self.side_length).any(|row| is_pixel_visible(column, row))
		});
		let height = visible_span(&|row| {
			(0..self.side_length).any(|column| is_pixel_visible(column, row))
		});
		(width, height)
	}
}

pub fn render_crosshair(crosshair: &Crosshair, scale_factor: f32) -> CrosshairImage {
	let layers: Vec<Layer> =
		[bars_layer(crosshair, scale_factor), dot_layer(crosshair, scale_factor)]
			.into_iter()
			.flatten()
			.collect();
	let centered_stroke_width = if crosshair.bars.has_any_side_enabled() {
		crosshair.bars.thickness
	} else {
		crosshair.dot.size
	};
	let odd_stroke_offset =
		scaled_pixels(centered_stroke_width, scale_factor) as i32 % 2;
	let farthest_reach =
		layers.iter().map(Layer::reach_from_center).fold(0.0, f32::max);
	let side_length = 2 * (farthest_reach.ceil() as i32 + 1) + odd_stroke_offset;
	let center = side_length as f32 / 2.0;
	let pixels = (0..side_length * side_length)
		.map(|pixel_index| {
			supersampled_pixel(
				&layers,
				(pixel_index % side_length) as f32 - center,
				(pixel_index / side_length) as f32 - center,
			)
		})
		.collect();
	CrosshairImage { side_length, pixels }
}

fn bars_layer(crosshair: &Crosshair, scale_factor: f32) -> Option<Layer> {
	let bars = &crosshair.bars;
	let half_length = scaled_pixels(bars.length, scale_factor) / 2.0;
	let half_thickness = scaled_pixels(bars.thickness, scale_factor) / 2.0;
	let bar_center_distance = scaled_pixels(bars.gap, scale_factor) + half_length;
	let corner_radius = match bars.shape {
		Shape::Rounded => half_thickness.min(half_length),
		Shape::Sharp => 0.0,
	};
	let vertical_bar_at = |center_y| RoundedRect {
		center_x: 0.0,
		center_y,
		half_width: half_thickness,
		half_height: half_length,
		corner_radius,
	};
	let horizontal_bar_at = |center_x| RoundedRect {
		center_x,
		center_y: 0.0,
		half_width: half_length,
		half_height: half_thickness,
		corner_radius,
	};
	let enabled_bars: Vec<RoundedRect> = [
		(bars.top, vertical_bar_at(-bar_center_distance)),
		(bars.bottom, vertical_bar_at(bar_center_distance)),
		(bars.left, horizontal_bar_at(-bar_center_distance)),
		(bars.right, horizontal_bar_at(bar_center_distance)),
	]
	.into_iter()
	.filter_map(|(is_enabled, bar)| is_enabled.then_some(bar))
	.collect();
	(!enabled_bars.is_empty()).then(|| {
		Layer::new(enabled_bars, bars.color, bars.opacity, &bars.outline, scale_factor)
	})
}

fn dot_layer(crosshair: &Crosshair, scale_factor: f32) -> Option<Layer> {
	let dot = &crosshair.dot;
	let half_size = scaled_pixels(dot.size, scale_factor) / 2.0;
	let corner_radius = match dot.shape {
		Shape::Rounded => half_size,
		Shape::Sharp => 0.0,
	};
	let dot_shape = RoundedRect {
		center_x: 0.0,
		center_y: 0.0,
		half_width: half_size,
		half_height: half_size,
		corner_radius,
	};
	dot.enabled.then(|| {
		Layer::new(vec![dot_shape], dot.color, dot.opacity, &dot.outline, scale_factor)
	})
}

fn scaled_pixels(pixels: i32, scale_factor: f32) -> f32 {
	(pixels as f32 * scale_factor).round()
}

fn supersampled_pixel(layers: &[Layer], pixel_left: f32, pixel_top: f32) -> u32 {
	let samples_per_pixel = SAMPLES_PER_AXIS * SAMPLES_PER_AXIS;
	let mut channel_sums = [0.0; 4];
	for sample_index in 0..samples_per_pixel {
		let sample_x = pixel_left
			+ ((sample_index % SAMPLES_PER_AXIS) as f32 + 0.5)
				/ SAMPLES_PER_AXIS as f32;
		let sample_y = pixel_top
			+ ((sample_index / SAMPLES_PER_AXIS) as f32 + 0.5)
				/ SAMPLES_PER_AXIS as f32;
		let sample_color = layers.iter().fold([0.0; 4], |color_below, layer| {
			composite_over(layer.color_at(sample_x, sample_y), color_below)
		});
		for (channel_sum, sample_channel) in channel_sums.iter_mut().zip(sample_color) {
			*channel_sum += sample_channel;
		}
	}
	let [red, green, blue, alpha] = channel_sums.map(|channel_sum| {
		(channel_sum / samples_per_pixel as f32 * 255.0).round() as u32
	});
	alpha << 24 | red << 16 | green << 8 | blue
}

fn composite_over(
	color_on_top: PremultipliedRgba,
	color_below: PremultipliedRgba,
) -> PremultipliedRgba {
	let top_transparency = 1.0 - color_on_top[3];
	std::array::from_fn(|channel| {
		color_on_top[channel] + color_below[channel] * top_transparency
	})
}

fn premultiply(color: Color, opacity: f32) -> PremultipliedRgba {
	let premultiplied_channel = |value: u8| value as f32 / 255.0 * opacity;
	[
		premultiplied_channel(color.r),
		premultiplied_channel(color.g),
		premultiplied_channel(color.b),
		opacity,
	]
}

struct Layer {
	shapes: Vec<RoundedRect>,
	fill_color: PremultipliedRgba,
	outline: Option<LayerOutline>,
}

#[derive(Clone, Copy)]
struct LayerOutline {
	color: PremultipliedRgba,
	thickness: f32,
}

impl Layer {
	fn new(
		shapes: Vec<RoundedRect>,
		fill_color: Color,
		fill_opacity: f32,
		outline: &Outline,
		scale_factor: f32,
	) -> Self {
		Self {
			shapes,
			fill_color: premultiply(fill_color, fill_opacity),
			outline: outline.enabled.then(|| LayerOutline {
				color: premultiply(outline.color, outline.opacity),
				thickness: scaled_pixels(outline.thickness, scale_factor),
			}),
		}
	}

	fn color_at(&self, x: f32, y: f32) -> PremultipliedRgba {
		let distance_to_nearest_shape = self
			.shapes
			.iter()
			.map(|shape| shape.signed_distance(x, y))
			.fold(f32::INFINITY, f32::min);
		match self.outline {
			_ if distance_to_nearest_shape <= 0.0 => self.fill_color,
			Some(outline) if distance_to_nearest_shape <= outline.thickness => {
				outline.color
			},
			_ => [0.0; 4],
		}
	}

	fn reach_from_center(&self) -> f32 {
		let outline_thickness = self.outline.map_or(0.0, |outline| outline.thickness);
		let farthest_shape_edge = self
			.shapes
			.iter()
			.map(|shape| {
				(shape.center_x.abs() + shape.half_width)
					.max(shape.center_y.abs() + shape.half_height)
			})
			.fold(0.0, f32::max);
		farthest_shape_edge + outline_thickness
	}
}

struct RoundedRect {
	center_x: f32,
	center_y: f32,
	half_width: f32,
	half_height: f32,
	corner_radius: f32,
}

impl RoundedRect {
	fn signed_distance(&self, x: f32, y: f32) -> f32 {
		let outside_x =
			(x - self.center_x).abs() - (self.half_width - self.corner_radius);
		let outside_y =
			(y - self.center_y).abs() - (self.half_height - self.corner_radius);
		if self.corner_radius == 0.0 {
			return outside_x.max(outside_y);
		}
		outside_x.max(0.0).hypot(outside_y.max(0.0)) + outside_x.max(outside_y).min(0.0)
			- self.corner_radius
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
			render_crosshair(&crosshair, 1.0).visible_bounds_size(),
			(2 * reach + 1, 2 * reach + 1)
		);
		assert_eq!(
			render_crosshair(&crosshair, 2.0).visible_bounds_size(),
			(4 * reach, 4 * reach)
		);
	}

	#[test]
	fn a_lone_dot_spans_its_size_and_outline() {
		let crosshair = without_bars();
		let expected = crosshair.dot.size + 2 * crosshair.dot.outline.thickness;
		assert_eq!(
			render_crosshair(&crosshair, 1.0).visible_bounds_size(),
			(expected, expected)
		);
	}

	#[test]
	fn hidden_bars_leave_their_axis() {
		let mut crosshair = Crosshair::default();
		crosshair.bars.left = false;
		crosshair.bars.right = false;
		let (width, height) = render_crosshair(&crosshair, 1.0).visible_bounds_size();
		assert!(width < height);
	}

	#[test]
	fn a_hidden_crosshair_draws_nothing() {
		let mut crosshair = without_bars();
		crosshair.dot.enabled = false;
		let image = render_crosshair(&crosshair, 1.5);
		assert!(image.pixels.iter().all(|&pixel| pixel == 0));
		assert_eq!(image.visible_bounds_size(), (0, 0));
	}
}
