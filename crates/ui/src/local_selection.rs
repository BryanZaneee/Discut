//! Account-local navigation choices. The Discord catalog and memberships are never edited.
use crate::MessagingUi;
use client_core::State;
use model::{Id, PreferenceEdit};

#[derive(Default)]
pub(super) struct Selection {
	query: String,
	capacity_reached: bool,
}

impl MessagingUi {
	pub(super) fn conversation_selection(&mut self, ui: &mut egui::Ui, state: &mut State) {
		ui.heading("Choose conversations");
		ui.label("Your Discord servers, DMs and group chats appear automatically. Everything is included by default.");
		ui.label("These choices only organize Discut on this device. They do not leave servers, delete chats or mute notifications. Notifications and direct links can still open hidden conversations.");
		ui.label("Discord still supplies your conversation list. Message history loads as you open conversations and scroll; this is not a bulk history import.");
		let available = self.channel_preferences_loaded || state.demo;
		if !available {
			ui.label("Loading this account’s saved choices…");
		}
		if !self.channel_preferences_status.is_empty() {
			ui.colored_label(
				crate::design::palette(ui).warning,
				crate::i18n::translate_if_key(self.channel_preferences_status),
			);
			if ui.button("Retry saving or loading choices").clicked() {
				if self.channel_preferences_loaded {
					self.channel_preferences_changed = true;
				} else {
					self.channel_preferences_reload = true;
				}
			}
		}
		ui.add_enabled_ui(available, |ui| {
			ui.add(
				egui::TextEdit::singleline(&mut self.local_selection.query)
					.char_limit(128)
					.hint_text("Find a server or conversation"),
			);
			if ui.button("Restore all conversations").clicked() {
				let changed = !self.channel_preferences.excluded_guilds.is_empty()
					|| !self.channel_preferences.excluded_channels.is_empty();
				self.channel_preferences.excluded_guilds.clear();
				self.channel_preferences.excluded_channels.clear();
				self.selection_changed(
					state,
					if changed {
						PreferenceEdit::Changed
					} else {
						PreferenceEdit::Unchanged
					},
				);
			}
			let query = self.local_selection.query.to_lowercase();
			let servers: Vec<_> = state
				.guilds
				.iter()
				.filter(|g| g.name.to_lowercase().contains(&query))
				.map(|g| (g.id, g.name.clone()))
				.collect();
			self.selection_rows(ui, state, "Servers", true, &servers);
			for (heading, kind) in [("Direct messages", 1), ("Group chats", 3)] {
				let rows: Vec<_> = state
					.channels
					.iter()
					.filter(|c| c.guild.is_none() && c.kind == kind)
					.filter_map(|c| {
						let name = state.conversation_name(c);
						name.to_lowercase()
							.contains(&query)
							.then(|| (c.id, name.to_owned()))
					})
					.collect();
				self.selection_rows(ui, state, heading, false, &rows);
			}
		});
		if self.local_selection.capacity_reached {
			ui.colored_label(crate::design::palette(ui).warning, "Local choices and shortcuts share a 256-item limit. Restore an excluded conversation or remove a shortcut before hiding another.");
		}
		ui.separator();
	}

	fn selection_rows(
		&mut self,
		ui: &mut egui::Ui,
		state: &mut State,
		heading: &str,
		guild: bool,
		rows: &[(Id, String)],
	) {
		egui::CollapsingHeader::new(format!("{heading} ({})", rows.len()))
			.default_open(true)
			.show(ui, |ui| {
				if rows.is_empty() {
					ui.label("No matching conversations in the loaded account list.");
				}
				egui::ScrollArea::vertical()
					.id_salt(heading)
					.max_height(160.0)
					.show_rows(ui, 24.0, rows.len(), |ui, range| {
						for (id, name) in &rows[range] {
							let mut included = if guild {
								self.channel_preferences.guild_included(*id)
							} else {
								!self.channel_preferences.excluded_channels.contains(id)
							};
							if ui
								.push_id((guild, id.0), |ui| ui.checkbox(&mut included, name))
								.inner
								.changed()
							{
								let edit = if guild {
									self.channel_preferences.set_guild_included(*id, included)
								} else {
									self.channel_preferences.set_channel_included(*id, included)
								};
								self.selection_changed(state, edit);
							}
						}
					});
			});
	}

	fn selection_changed(&mut self, state: &mut State, edit: PreferenceEdit) {
		self.local_selection.capacity_reached = edit == PreferenceEdit::CapacityReached;
		if edit != PreferenceEdit::Changed {
			return;
		}
		self.channel_preferences_changed = true;
		self.channel_cache.invalidate();
		self.switcher.invalidate();
		if state
			.selected
			.and_then(|id| state.channel(id))
			.is_some_and(|channel| !self.channel_preferences.channel_included(channel))
		{
			state.open_home();
			self.guild = None;
			self.search.open = false;
		} else if self
			.guild
			.is_some_and(|id| !self.channel_preferences.guild_included(id))
		{
			self.guild = None;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn excluding_selected_conversation_preserves_draft_catalog_and_membership() {
		for (id, guild) in [(Id(10), true), (Id(29), false), (Id(40), false)] {
			let mut state = test_support::demo_state();
			let selected = if guild { Id(20) } else { id };
			state.selected = Some(selected);
			state.drafts.insert(selected, "Unsent draft".into());
			let channels = state.channels.clone();
			let guilds = state.guilds.clone();
			let mut view = MessagingUi {
				guild: guild.then_some(id),
				..Default::default()
			};
			let edit = if guild {
				view.channel_preferences.set_guild_included(id, false)
			} else {
				view.channel_preferences.set_channel_included(id, false)
			};
			view.selection_changed(&mut state, edit);
			assert_eq!(state.selected, None);
			assert_eq!(view.guild, None);
			assert_eq!(state.drafts[&selected], "Unsent draft");
			assert!(state.channels == channels);
			assert!(state.guilds == guilds);
			assert!(view.channel_preferences_changed);
			assert!(
				!view
					.channel_preferences
					.channel_included(state.channel(selected).unwrap())
			);
			// A deliberate deep link can still open it; this is not a permission boundary.
			state.select(selected);
			assert_eq!(state.selected, Some(selected));
			assert_eq!(state.drafts[&selected], "Unsent draft");
		}
	}

	#[test]
	fn checkbox_changes_only_local_selection_in_both_themes_and_widths() {
		for dark in [true, false] {
			for width in [360.0, 900.0] {
				for guild in [true, false] {
					let ctx = egui::Context::default();
					ctx.set_theme(if dark {
						egui::ThemePreference::Dark
					} else {
						egui::ThemePreference::Light
					});
					crate::design::apply(&ctx);
					let mut state = test_support::demo_state();
					let mut view = MessagingUi::default();
					let id = if guild { Id(10) } else { Id(29) };
					let rows = [(id, "Selection target".into())];
					let mut frame = |view: &mut MessagingUi, events| {
						let output = ctx.run_ui(
							egui::RawInput {
								screen_rect: Some(egui::Rect::from_min_size(
									egui::Pos2::ZERO,
									egui::vec2(width, 600.0),
								)),
								events,
								..Default::default()
							},
							|ui| view.selection_rows(ui, &mut state, "Conversations", guild, &rows),
						);
						fn find(shape: &egui::Shape) -> Option<egui::Pos2> {
							match shape {
								egui::Shape::Text(text)
									if text.galley.job.text == "Selection target" =>
								{
									Some(text.pos + text.galley.size() / 2.0)
								}
								egui::Shape::Vec(shapes) => shapes.iter().find_map(find),
								_ => None,
							}
						}
						let target = output.shapes.iter().find_map(|shape| find(&shape.shape));
						assert!(output.platform_output.commands.is_empty());
						output.drop_without_applying_deltas();
						target
					};
					for _ in 0..3 {
						frame(&mut view, vec![]);
					}
					let pos = frame(&mut view, vec![]).expect("checkbox label");
					for pressed in [true, false] {
						frame(
							&mut view,
							vec![
								egui::Event::PointerMoved(pos),
								egui::Event::PointerButton {
									pos,
									button: egui::PointerButton::Primary,
									pressed,
									modifiers: Default::default(),
								},
							],
						);
					}
					assert!(view.channel_preferences_changed);
					assert!(!view.local_selection.capacity_reached);
					assert!(if guild {
						view.channel_preferences.excluded_guilds.contains(&id)
					} else {
						view.channel_preferences.excluded_channels.contains(&id)
					});
				}
			}
		}
	}
}
