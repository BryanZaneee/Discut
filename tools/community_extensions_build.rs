// Shared by build scripts through `include!`. Points `COMMUNITY_EXTENSIONS` at the
// `community-extensions` submodule, or at empty placeholders when it is not checked out, so
// builds and Clippy never need it. Code that reads those packages at runtime still does.

/// Every submodule file the workspace embeds with `include_bytes!`.
const COMMUNITY_FILES: [&str; 15] = [
	"catalog.json",
	"plugins/emoji-sticker-images/manifest.json",
	"plugins/message-delete-protector/manifest.json",
	"plugins/packages/emoji-sticker-images.serein-extension",
	"plugins/packages/message-delete-protector.serein-extension",
	"previews/ocean.png",
	"themes/forest.serein-extension",
	"themes/golden.serein-extension",
	"themes/katana.serein-extension",
	"themes/latte.serein-extension",
	"themes/midnight.serein-extension",
	"themes/obsidian.serein-extension",
	"themes/ocean.serein-extension",
	"themes/rose.serein-extension",
	"themes/teal.serein-extension",
];

fn community_extensions() {
	let manifest = std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
	let real = manifest.join("../../community-extensions");
	let complete = COMMUNITY_FILES.iter().all(|file| real.join(file).is_file());
	let root = if complete {
		for file in COMMUNITY_FILES {
			println!("cargo:rerun-if-changed={}", real.join(file).display());
		}
		real
	} else {
		// Watching the (possibly empty) submodule directory notices a later checkout.
		if real.is_dir() {
			println!("cargo:rerun-if-changed={}", real.display());
		}
		let placeholder =
			std::path::PathBuf::from(std::env::var_os("OUT_DIR").unwrap()).join("community-extensions");
		for file in COMMUNITY_FILES {
			let path = placeholder.join(file);
			std::fs::create_dir_all(path.parent().unwrap()).unwrap();
			std::fs::write(path, []).unwrap();
		}
		placeholder
	};
	println!("cargo:rustc-env=COMMUNITY_EXTENSIONS={}", root.display());
}
