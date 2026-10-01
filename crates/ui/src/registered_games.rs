//! Discord's "Registered Games" settings page: the game detection currently sees, games the
//! user added from running processes, inline renaming and removal of wrong detections.
use crate::{MessagingUi, design, icons};
use egui::RichText;
use model::registered_games::{self as games, RegisteredGame};

#[derive(Default)]
pub(super) struct Page {
	/// The "Add it!" picker is open.
	adding: bool,
	query: String,
	selected: Option<String>,
	/// Executable being renamed and its draft name.
	editing: Option<(String, String)>,
}

/// Rows past this stay reachable through the filter instead of one very long list.
const MAX_PICKER_ROWS: usize = 200;

impl MessagingUi {
	pub(super) fn registered_games_settings(&mut self, ui: &mut egui::Ui, demo: bool) {
		let p = design::palette(ui);
		ui.add_space(8.0);
		heading(ui, "Current Game");
		ui.add_space(4.0);
		let running = self.running_game.clone();
		design::card(ui, |ui| {
			ui.spacing_mut().item_spacing.y = 4.0;
			if self.settings.games.adding {
				self.process_picker(ui, demo);
				return;
			}
			if let Some(game) = &running {
				ui.horizontal(|ui| {
					ui.vertical(|ui| {
						ui.spacing_mut().item_spacing.y = 2.0;
						self.editable_name(ui, &game.executable, &game.name, game.application);
						ui.label(
							RichText::new(crate::i18n::translate_if_key("Now Playing!"))
								.size(13.0)
								.color(p.positive),
						);
					});
					ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
						if remove_button(ui, "Not this game? Stop detecting it.").clicked() {
							hide(&mut self.registered_games, game);
							self.running_game = None;
						}
					});
				});
			} else if let Some(own) = self
				.own_game
				.as_deref()
				.filter(|_| self.share_game_activity)
			{
				// A game speaking Rich Presence names itself; there is nothing to rename.
				ui.label(design::semibold(ui, own, 16.0).color(p.text_strong));
				ui.label(
					RichText::new(crate::i18n::translate_if_key(
						"Reported by the game through Rich Presence.",
					))
					.size(13.0)
					.color(p.muted),
				);
			} else {
				ui.label(
					design::semibold(ui, crate::i18n::translate_if_key("No game detected"), 16.0)
						.color(p.text_strong),
				);
				if !self.share_game_activity {
					ui.label(
						RichText::new(crate::i18n::translate_if_key(
							"Turn on Share game activity on the Game Activity page to detect games.",
						))
						.size(13.0)
						.color(p.muted),
					);
				}
			}
			if !self.settings.games.adding {
				ui.horizontal(|ui| {
					ui.spacing_mut().item_spacing.x = 4.0;
					ui.label(
						RichText::new(crate::i18n::translate_if_key("Not seeing your game?"))
							.size(14.0)
							.color(p.muted),
					);
					let add = ui.add(
						egui::Label::new(
							design::medium(ui, crate::i18n::translate_if_key("Add it!"), 14.0)
								.color(p.link),
						)
						.sense(egui::Sense::click()),
					);
					if add
						.on_hover_cursor(egui::CursorIcon::PointingHand)
						.clicked()
					{
						let games = &mut self.settings.games;
						games.adding = true;
						games.query.clear();
						games.selected = None;
						self.running_processes = None;
						self.running_processes_request = !demo;
						if demo {
							self.running_processes = Some(
								[
									"/opt/synthetic/My Game/game.x86_64",
									"/usr/bin/synthetic-tool",
								]
								.map(str::to_owned)
								.to_vec(),
							);
						}
					}
				});
			}
		});
		design::divider(ui);
		heading(ui, "Added Games");
		if self.registered_games.is_empty() {
			ui.label(
				RichText::new(crate::i18n::translate_if_key("No games added"))
					.size(14.0)
					.color(p.muted),
			);
			return;
		}
		ui.add_space(4.0);
		let mut remove = None;
		let mut restore = None;
		for (index, game) in self.registered_games.clone().iter().enumerate() {
			design::card(ui, |ui| {
				ui.horizontal(|ui| {
					ui.vertical(|ui| {
						ui.spacing_mut().item_spacing.y = 2.0;
						ui.set_max_width((ui.available_width() - 120.0).max(120.0));
						if game.hidden {
							ui.label(design::semibold(ui, &game.name, 16.0).color(p.muted));
						} else {
							self.editable_name(ui, &game.executable, &game.name, game.application);
						}
						let detail = if game.hidden {
							crate::i18n::translate_if_key(
								"Removed. Serein will not detect this game.",
							)
						} else if running
							.as_ref()
							.is_some_and(|r| r.executable == game.executable)
						{
							crate::i18n::translate_if_key("Now Playing!")
						} else {
							game.executable.clone()
						};
						ui.add(
							egui::Label::new(RichText::new(detail).size(12.0).color(p.muted))
								.truncate(),
						);
					});
					ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
						if game.hidden {
							if design::button(ui, "Restore", design::ButtonKind::Outline)
								.on_hover_text(crate::i18n::translate_if_key(
									"Detect this game again.",
								))
								.clicked()
							{
								restore = Some(index);
							}
						} else if remove_button(ui, "Remove this game").clicked() {
							remove = Some(index);
						}
					});
				});
			});
		}
		if let Some(index) = restore {
			self.registered_games.remove(index);
		}
		if let Some(index) = remove {
			// A game Discord knows would be detected right back, so it stays as removed.
			let game = &mut self.registered_games[index];
			if game.application.is_some() {
				game.hidden = true;
			} else {
				self.registered_games.remove(index);
			}
		}
	}

	/// Click the name to rename it; Enter or focus loss saves, Escape cancels.
	fn editable_name(
		&mut self,
		ui: &mut egui::Ui,
		executable: &str,
		name: &str,
		application: Option<model::Id>,
	) {
		let p = design::palette(ui);
		let editing = &mut self.settings.games.editing;
		if let Some((target, draft)) = editing.as_mut().filter(|(target, _)| target == executable) {
			let response = design::input(
				ui,
				egui::TextEdit::singleline(draft)
					.char_limit(games::MAX_NAME)
					.font(egui::FontId::new(15.0, design::medium_family(ui.ctx()))),
			);
			if !response.has_focus() && !response.lost_focus() {
				response.request_focus();
			}
			let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
			if response.lost_focus() || cancel {
				let draft = clamp(draft);
				let target = target.clone();
				*editing = None;
				if !cancel && games::valid_name(&draft) && draft != name {
					rename(&mut self.registered_games, &target, draft, application);
				}
			}
			return;
		}
		let label = ui
			.add(
				egui::Label::new(design::semibold(ui, name, 16.0).color(p.text_strong))
					.truncate()
					.sense(egui::Sense::click()),
			)
			.on_hover_cursor(egui::CursorIcon::Text)
			.on_hover_text(crate::i18n::translate_if_key("Click to rename"));
		if label.hovered() {
			let rect = egui::Rect::from_min_size(
				egui::pos2(label.rect.right() + 6.0, label.rect.center().y - 7.0),
				egui::Vec2::splat(14.0),
			);
			icons::paint(ui.painter(), icons::Icon::Pencil, rect, p.muted);
		}
		if label.clicked() {
			*editing = Some((executable.to_owned(), name.to_owned()));
		}
	}

	fn process_picker(&mut self, ui: &mut egui::Ui, demo: bool) {
		let p = design::palette(ui);
		ui.label(
			design::semibold(ui, crate::i18n::translate_if_key("Add a game"), 16.0)
				.color(p.text_strong),
		);
		ui.label(
			RichText::new(crate::i18n::translate_if_key(
				"Choose a running program. Serein shows it as your game whenever it runs.",
			))
			.size(13.0)
			.color(p.muted),
		);
		ui.add_space(6.0);
		let picker = &mut self.settings.games;
		design::input(
			ui,
			egui::TextEdit::singleline(&mut picker.query)
				.hint_text(crate::i18n::translate_if_key("Search running programs"))
				.char_limit(64),
		);
		ui.add_space(4.0);
		match &self.running_processes {
			None => {
				ui.horizontal(|ui| {
					ui.spinner();
					ui.label(
						RichText::new(crate::i18n::translate_if_key("Reading running programs…"))
							.color(p.muted),
					);
				});
			}
			Some(paths) => {
				let query = picker.query.trim().to_lowercase();
				let registered = &self.registered_games;
				let candidates: Vec<&String> = paths
					.iter()
					.filter(|path| query.is_empty() || path.to_lowercase().contains(&query))
					.filter(|path| {
						let key = games::normalize(path);
						!registered
							.iter()
							.any(|g| !g.hidden && Some(&g.executable) == key.as_ref())
					})
					.take(MAX_PICKER_ROWS)
					.collect();
				if candidates.is_empty() {
					design::hint(ui, "No matching programs are running.");
				}
				egui::ScrollArea::vertical()
					.id_salt("registered-games-picker")
					.max_height(240.0)
					.auto_shrink([false, true])
					.show(ui, |ui| {
						ui.spacing_mut().item_spacing.y = 2.0;
						for path in candidates {
							let selected = picker.selected.as_ref() == Some(path);
							let response = process_row(ui, path, selected);
							if response.clicked() {
								picker.selected = Some(path.clone());
							}
							if response.double_clicked() {
								picker.selected = Some(path.clone());
							}
						}
					});
			}
		}
		ui.add_space(8.0);
		let mut add = false;
		let mut cancel = false;
		ui.horizontal(|ui| {
			ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
				ui.add_enabled_ui(picker.selected.is_some(), |ui| {
					add = design::button(ui, "Add Game", design::ButtonKind::Primary).clicked();
				});
				cancel = design::button(ui, "Cancel", design::ButtonKind::Neutral).clicked();
			});
		});
		if add
			&& let Some(path) = picker.selected.take()
			&& let Some(executable) = games::normalize(&path)
		{
			let name = games::default_name(&path);
			self.registered_games.retain(|g| g.executable != executable);
			if self.registered_games.len() < games::MAX_GAMES {
				self.registered_games.push(RegisteredGame {
					executable,
					name,
					application: None,
					hidden: false,
				});
			}
			cancel = true;
		}
		if cancel {
			picker.adding = false;
			picker.query.clear();
			picker.selected = None;
			if !demo {
				self.running_processes = None;
			}
		}
	}
}

fn heading(ui: &mut egui::Ui, text: &str) {
	let p = design::palette(ui);
	ui.label(design::medium(ui, crate::i18n::translate_if_key(text), 22.0).color(p.text_strong));
}

fn process_row(ui: &mut egui::Ui, path: &str, selected: bool) -> egui::Response {
	let p = design::palette(ui);
	let (rect, response) =
		ui.allocate_exact_size(egui::vec2(ui.available_width(), 40.0), egui::Sense::click());
	if ui.is_rect_visible(rect) {
		let fill = if selected {
			p.selected
		} else if response.hovered() {
			p.hover
		} else {
			egui::Color32::TRANSPARENT
		};
		ui.painter().rect_filled(rect, 6, fill);
		let icon = egui::Rect::from_center_size(
			egui::pos2(rect.left() + 18.0, rect.center().y),
			egui::Vec2::splat(18.0),
		);
		icons::paint(ui.painter(), icons::Icon::GameController, icon, p.muted);
		let text = rect
			.shrink2(egui::vec2(0.0, 3.0))
			.with_min_x(rect.left() + 36.0);
		let mut child = ui.new_child(
			egui::UiBuilder::new()
				.max_rect(text)
				.layout(egui::Layout::top_down(egui::Align::Min)),
		);
		child.spacing_mut().item_spacing.y = 0.0;
		child.add(
			egui::Label::new(
				design::medium(&child, games::default_name(path), 14.0).color(p.text_strong),
			)
			.truncate()
			.selectable(false),
		);
		child.add(
			egui::Label::new(RichText::new(path).size(11.0).color(p.muted))
				.truncate()
				.selectable(false),
		);
	}
	response.widget_info(|| {
		egui::WidgetInfo::selected(egui::Role::ListBoxOption, true, selected, path)
	});
	response
}

/// Round close control matching the saved-account rows.
fn remove_button(ui: &mut egui::Ui, hint: &str) -> egui::Response {
	let p = design::palette(ui);
	let (rect, response) = ui.allocate_exact_size(egui::Vec2::splat(28.0), egui::Sense::click());
	let over = response.hovered() || response.has_focus();
	if over {
		ui.painter()
			.circle_filled(rect.center(), 14.0, p.danger.gamma_multiply(0.16));
	}
	icons::paint(
		ui.painter(),
		icons::Icon::Close,
		rect.shrink(8.0),
		if over { p.danger } else { p.muted },
	);
	let hint = crate::i18n::translate_if_key(hint);
	response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &hint));
	response.on_hover_text(hint)
}

/// Trimmed and cut to the byte bound at a character boundary.
fn clamp(name: &str) -> String {
	let mut name = name.trim().to_owned();
	while name.len() > games::MAX_NAME {
		name.pop();
	}
	name.trim_end().to_owned()
}

fn rename(
	list: &mut Vec<RegisteredGame>,
	executable: &str,
	name: String,
	application: Option<model::Id>,
) {
	if let Some(game) = list.iter_mut().find(|g| g.executable == executable) {
		game.name = name;
	} else if list.len() < games::MAX_GAMES {
		list.push(RegisteredGame {
			executable: executable.to_owned(),
			name,
			application,
			hidden: false,
		});
	}
}

fn hide(list: &mut Vec<RegisteredGame>, game: &games::RunningGame) {
	if let Some(entry) = list.iter_mut().find(|g| g.executable == game.executable) {
		entry.hidden = true;
		entry.application = entry.application.or(game.application);
	} else if list.len() < games::MAX_GAMES {
		list.push(RegisteredGame {
			executable: game.executable.clone(),
			name: game.name.clone(),
			application: game.application,
			hidden: true,
		});
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hiding_and_renaming_update_one_entry() {
		let running = games::RunningGame {
			executable: "lms.exe".into(),
			name: "Last Man Standing".into(),
			application: Some(model::Id(7)),
			renamed: false,
		};
		let mut list = Vec::new();
		rename(&mut list, "lms.exe", "Renamed".into(), running.application);
		hide(&mut list, &running);
		assert_eq!(list.len(), 1);
		assert!(list[0].hidden);
		assert_eq!(list[0].name, "Renamed");
		assert_eq!(clamp(&format!("  {}é ", "x".repeat(127))).len(), 127);
	}
}
