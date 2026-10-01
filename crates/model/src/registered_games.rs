//! Games the user registered on this device, matching Discord's "Registered Games" page:
//! manually added executables, renamed detections and detections the user removed.
use crate::Id;
use serde::{Deserialize, Serialize};

pub const MAX_GAMES: usize = 128;
pub const MAX_NAME: usize = 128;
pub const MAX_EXECUTABLE: usize = 256;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisteredGame {
	/// Normalized executable path as it was running when registered; see [`normalize`].
	pub executable: String,
	pub name: String,
	/// The detectable application this executable matched, when Discord knows the game.
	#[serde(default)]
	pub application: Option<Id>,
	/// Removed by the user: never detect this executable or application again.
	#[serde(default)]
	pub hidden: bool,
}

impl RegisteredGame {
	pub fn valid(&self) -> bool {
		normalize(&self.executable).as_deref() == Some(self.executable.as_str())
			&& valid_name(&self.name)
	}
}

/// The game a local process scan currently reports, shown as "Current Game".
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningGame {
	pub executable: String,
	pub name: String,
	pub application: Option<Id>,
	/// Shown by its registered name rather than Discord's.
	pub renamed: bool,
}

pub fn valid_name(name: &str) -> bool {
	let trimmed = name.trim();
	!trimmed.is_empty()
		&& trimmed.len() == name.len()
		&& name.len() <= MAX_NAME
		&& !name.chars().any(char::is_control)
}

/// Executables are compared as path suffixes, so both sides use one normal form.
pub fn normalize(name: &str) -> Option<String> {
	let name = name.trim().trim_start_matches('>');
	if name.is_empty() || name.len() > MAX_EXECUTABLE || name.chars().any(char::is_control) {
		return None;
	}
	let name = name
		.to_lowercase()
		.replace('\\', "/")
		.trim_matches('/')
		.to_owned();
	(!name.is_empty()).then_some(name)
}

/// A readable default title for a newly added executable: its file name without extension.
pub fn default_name(executable: &str) -> String {
	let file = executable.rsplit(['/', '\\']).next().unwrap_or(executable);
	let file = file.strip_suffix(".app").unwrap_or(file);
	let stem = file
		.rsplit_once('.')
		.filter(|(stem, extension)| !stem.is_empty() && extension.len() <= 4)
		.map_or(file, |(stem, _)| stem);
	let mut name: String = stem.trim().chars().take(MAX_NAME).collect();
	while name.len() > MAX_NAME {
		name.pop();
	}
	if name.is_empty() {
		"Unknown game".into()
	} else {
		name
	}
}

/// Drop invalid and duplicate entries so a hand-edited file cannot grow detection work.
pub fn sanitize(games: &mut Vec<RegisteredGame>) {
	let mut seen = std::collections::HashSet::new();
	games.retain(|game| game.valid() && seen.insert(game.executable.clone()));
	games.truncate(MAX_GAMES);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn names_and_entries_are_bounded() {
		assert_eq!(
			default_name("/opt/my game/Game-Bin.x86_64"),
			"Game-Bin.x86_64"
		);
		assert_eq!(default_name("c:/games/lms.exe"), "lms");
		assert_eq!(default_name("/applications/foo.app"), "foo");
		assert_eq!(default_name("/usr/bin/.hidden"), ".hidden");
		let game = |executable: &str, name: &str| RegisteredGame {
			executable: executable.into(),
			name: name.into(),
			application: None,
			hidden: false,
		};
		let mut games = vec![
			game("a.exe", "A"),
			game("a.exe", "Duplicate"),
			game("B.EXE", "Not normalized"),
			game("c.exe", " padded "),
			game("d.exe", "D"),
		];
		sanitize(&mut games);
		assert_eq!(
			games.iter().map(|g| g.name.as_str()).collect::<Vec<_>>(),
			["A", "D"]
		);
	}
}
