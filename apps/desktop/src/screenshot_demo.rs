//! One native-renderer capture, available only with explicit offline demo arguments.
use eframe::egui;
use std::path::PathBuf;

#[derive(Default)]
pub(crate) struct Capture {
	path: Option<PathBuf>,
	requested: bool,
}

impl Capture {
	pub(crate) fn new(demo: bool) -> Self {
		Self {
			path: demo
				.then(|| {
					std::env::args()
						.find_map(|arg| arg.strip_prefix("--demo-screenshot=").map(PathBuf::from))
				})
				.flatten(),
			..Self::default()
		}
	}

	pub(crate) fn tick(&mut self, ctx: &egui::Context) {
		let Some(path) = self.path.as_ref() else {
			return;
		};
		let frame = ctx.cumulative_frame_nr();
		if frame >= 15 && !self.requested {
			self.requested = true;
			let path = path.clone();
			ctx.request_screenshot(move |pixels| {
				match image::save_buffer(
					&path,
					pixels.as_raw(),
					pixels.width() as u32,
					pixels.height() as u32,
					image::ColorType::Rgba8,
				) {
					Ok(()) => eprintln!("Synthetic native screenshot saved to {}", path.display()),
					Err(error) => eprintln!("Synthetic screenshot failed: {error}"),
				}
			});
		}
		// A finite repaint window settles layout and drives asynchronous WGPU readback.
		if frame < 45 {
			ctx.request_repaint_after(std::time::Duration::from_millis(50));
		} else {
			self.path = None;
		}
	}
}
