//! Native macOS title-bar appearance without recreating the window or its webviews.
#![allow(unsafe_code)]

use objc2::rc::Retained;
use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior};

fn native_window(window: &winit::window::Window) -> Result<Retained<NSWindow>, &'static str> {
	use objc2::MainThreadMarker;
	use objc2_app_kit::NSView;
	use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
	let _main = MainThreadMarker::new().ok_or("Window appearance requires the main thread.")?;
	let RawWindowHandle::AppKit(handle) = window
		.window_handle()
		.map_err(|_| "Native window unavailable.")?
		.as_raw()
	else {
		return Err("Expected a macOS window.");
	};
	// SAFETY: winit owns the borrowed view; AppKit access stays on the main thread.
	let view = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
	view.window().ok_or("Native window unavailable.")
}

fn managed_spaces(mut behavior: NSWindowCollectionBehavior) -> NSWindowCollectionBehavior {
	behavior.remove(
		NSWindowCollectionBehavior::CanJoinAllSpaces
			| NSWindowCollectionBehavior::MoveToActiveSpace
			| NSWindowCollectionBehavior::Stationary
			| NSWindowCollectionBehavior::Transient,
	);
	behavior.insert(NSWindowCollectionBehavior::Managed);
	behavior
}

/// The main chat window belongs to a desktop and participates in Mission Control.
/// Apply once at creation, preserving fullscreen and window-cycling capabilities.
pub fn configure_spaces(window: &winit::window::Window) -> Result<(), &'static str> {
	let window = native_window(window)?;
	window.setCollectionBehavior(managed_spaces(window.collectionBehavior()));
	window.setMovable(true);
	Ok(())
}

pub fn set_native_title_bar(
	window: &winit::window::Window,
	native: bool,
) -> Result<(), &'static str> {
	use objc2_app_kit::{NSWindowStyleMask, NSWindowTitleVisibility};
	let window = native_window(window)?;
	let mut style = window.styleMask();
	if style.contains(NSWindowStyleMask::FullSizeContentView) == native {
		style.set(NSWindowStyleMask::FullSizeContentView, !native);
		window.setStyleMask(style);
		window.setTitlebarAppearsTransparent(!native);
		window.setTitleVisibility(if native {
			NSWindowTitleVisibility::Visible
		} else {
			NSWindowTitleVisibility::Hidden
		});
	}
	Ok(())
}

#[cfg(test)]
mod tests {
	use super::*;
	#[test]
	fn main_window_joins_one_space_without_losing_fullscreen_or_cycling() {
		let preserved = NSWindowCollectionBehavior::FullScreenPrimary
			| NSWindowCollectionBehavior::ParticipatesInCycle;
		let sticky = NSWindowCollectionBehavior::CanJoinAllSpaces
			| NSWindowCollectionBehavior::MoveToActiveSpace
			| NSWindowCollectionBehavior::Stationary
			| NSWindowCollectionBehavior::Transient;
		let expected = preserved | NSWindowCollectionBehavior::Managed;
		assert_eq!(managed_spaces(preserved | sticky), expected);
		assert_eq!(managed_spaces(expected), expected);
	}
}
