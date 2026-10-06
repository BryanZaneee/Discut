//! Explicit group creation and membership controls. No write occurs on selection alone.
use crate::MessagingUi;
use client_core::{Command, State};
use model::Id;

#[derive(Default)]
pub(super) struct GroupManager {
	open: bool,
	channel: Option<Id>,
	selected: Vec<Id>,
	remove: Option<Id>,
	generation: u64,
	submitted: Option<(Id, u64)>,
}

impl MessagingUi {
	pub(crate) fn group_controls(
		&mut self,
		ui: &mut egui::Ui,
		state: &mut State,
		commands: &mut Vec<Command>,
	) {
		if self.group_manager.generation != state.generation {
			self.group_manager = GroupManager {
				generation: state.generation,
				..Default::default()
			};
		}
		let editor = &mut self.group_manager;
		ui.menu_button("Groups", |ui| {
			if ui.button("Create group DM…").clicked() {
				editor.open = true;
				editor.channel = None;
				editor.selected.clear();
				editor.remove = None;
				editor.submitted = None;
				state.clear_group_action_result(Id(0));
				ui.close();
			}
			if let Some(channel) = state.selected.filter(|id| state.is_group_dm(*id))
				&& ui.button("Manage group members…").clicked()
			{
				editor.open = true;
				editor.channel = Some(channel);
				editor.selected.clear();
				editor.remove = None;
				editor.submitted = None;
				state.clear_group_action_result(channel);
				ui.close();
			}
		});
		editor.show(ui.ctx(), state, commands);
	}
}

impl GroupManager {
	fn show(&mut self, ctx: &egui::Context, state: &mut State, commands: &mut Vec<Command>) {
		if !self.open {
			return;
		}
		if self.channel.is_some_and(|id| !state.is_group_dm(id)) {
			self.open = false;
			return;
		}
		let target = self.channel.unwrap_or(Id(0));
		if let Some((channel, request)) = self.submitted
			&& let Some(success) = state.group_action_completed(channel, request)
		{
			self.submitted = None;
			if success {
				self.selected.clear();
				self.remove = None;
			}
		}
		let busy = state.group_action_pending();
		let connected = state.demo || state.gateway_connected;
		let mut request = None;
		let mut open = self.open;
		egui::Window::new(if self.channel.is_some() {
			"Group members"
		} else {
			"Create group DM"
		})
		.id(egui::Id::unique("discut-group-members"))
		.open(&mut open)
		.collapsible(false)
		.resizable(true)
		.default_width(360.0)
		.show(ctx, |ui| {
			if let Some(channel) = self.channel {
				ui.label(
					"Only the group owner can remove other members. Discord checks permission.",
				);
				if let Some(group) = state.channel(channel) {
					egui::ScrollArea::vertical()
						.id_salt("current-members")
						.max_height(180.0)
						.show(ui, |ui| {
							for user in &group.recipients {
								ui.horizontal(|ui| {
									ui.label(&user.name);
									if state.user.as_ref().is_none_or(|me| me.id != user.id)
										&& ui
											.add_enabled(
												!busy && connected,
												egui::Button::new("Remove…"),
											)
											.clicked()
									{
										self.remove = Some(user.id);
									}
								});
							}
						});
				}
				if let Some(user) = self.remove {
					ui.separator();
					ui.label("Remove this member from the group conversation?");
					ui.horizontal(|ui| {
						if ui
							.add_enabled(!busy && connected, egui::Button::new("Confirm removal"))
							.clicked()
						{
							request = state.change_group_recipient(channel, user, false);
						}
						if ui.button("Cancel removal").clicked() {
							self.remove = None;
						}
					});
				}
				ui.separator();
				ui.label("Select one friend to add:");
			} else {
				ui.label("Select 2–9 friends. Creating the group invites those people.");
			}
			let friends: Vec<_> = state
				.friends()
				.filter(|user| {
					state.user.as_ref().is_none_or(|me| me.id != user.id)
						&& self
							.channel
							.and_then(|id| state.channel(id))
							.is_none_or(|group| {
								!group.recipients.iter().any(|member| member.id == user.id)
							})
				})
				.collect();
			self.selected
				.retain(|id| friends.iter().any(|user| user.id == *id));
			if friends.is_empty() {
				ui.label(
					"No eligible friends available. Add friends or reconnect to refresh the list.",
				);
			}
			egui::ScrollArea::vertical()
				.id_salt("group-friends")
				.max_height(240.0)
				.show_rows(ui, 26.0, friends.len(), |ui, rows| {
					for index in rows {
						let user = friends[index];
						let mut selected = self.selected.contains(&user.id);
						let limit = if self.channel.is_some() { 1 } else { 9 };
						if ui
							.add_enabled(
								!busy && (selected || self.selected.len() < limit),
								egui::Checkbox::new(&mut selected, &user.name),
							)
							.changed()
						{
							if selected {
								self.selected.push(user.id);
							} else {
								self.selected.retain(|id| *id != user.id);
							}
						}
					}
				});
			let valid = if self.channel.is_some() {
				self.selected.len() == 1
			} else {
				(2..=9).contains(&self.selected.len())
			};
			if ui
				.add_enabled(
					!busy && connected && valid,
					egui::Button::new(if self.channel.is_some() {
						"Add selected friend"
					} else {
						"Create group"
					}),
				)
				.clicked()
			{
				request = if let Some(channel) = self.channel {
					state.change_group_recipient(channel, self.selected[0], true)
				} else {
					state.create_group(self.selected.to_vec())
				};
			}
			if busy {
				ui.spinner();
				ui.label("Waiting for Discord confirmation…");
			}
			if !connected {
				ui.label("Reconnect before changing groups.");
			}
			if let Some(status) = state.group_action_status(target) {
				ui.label(status);
			}
		});
		self.open = open;
		if let Some(command) = request {
			if let Command::GroupAction { request, .. } = &command {
				self.submitted = Some((target, *request));
			}
			commands.push(command);
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	fn frame(
		ctx: &egui::Context,
		editor: &mut GroupManager,
		state: &mut State,
		commands: &mut Vec<Command>,
		events: Vec<egui::Event>,
	) -> Option<egui::Pos2> {
		let output = ctx.run_ui(
			egui::RawInput {
				screen_rect: Some(egui::Rect::from_min_size(
					egui::Pos2::ZERO,
					egui::vec2(800.0, 700.0),
				)),
				events,
				..Default::default()
			},
			|ui| editor.show(ui.ctx(), state, commands),
		);
		fn button(shape: &egui::Shape) -> Option<egui::Pos2> {
			match shape {
				egui::Shape::Text(text) if text.galley.job.text == "Create group" => {
					Some(text.galley.rect.translate(text.pos.to_vec2()).center())
				}
				egui::Shape::Vec(shapes) => shapes.iter().find_map(button),
				_ => None,
			}
		}
		let pos = output.shapes.iter().find_map(|shape| button(&shape.shape));
		output.drop_without_applying_deltas();
		pos
	}
	#[test]
	fn nine_selected_friends_only_send_after_create_click() {
		let ctx = egui::Context::default();
		let mut state = test_support::demo_state();
		let mut editor = GroupManager {
			open: true,
			..Default::default()
		};
		for user in state.friends().take(9) {
			editor.selected.push(user.id);
		}
		assert_eq!(editor.selected.len(), 9);
		let mut commands = vec![];
		frame(&ctx, &mut editor, &mut state, &mut commands, vec![]);
		let pos = frame(&ctx, &mut editor, &mut state, &mut commands, vec![])
			.expect("Create button visible");
		assert!(commands.is_empty());
		for pressed in [true, false] {
			frame(
				&ctx,
				&mut editor,
				&mut state,
				&mut commands,
				vec![
					egui::Event::PointerMoved(pos),
					egui::Event::PointerButton {
						pos,
						button: egui::PointerButton::Primary,
						pressed,
						modifiers: egui::Modifiers::NONE,
					},
				],
			);
		}
		assert_eq!(commands.len(), 1);
		assert!(
			matches!(&commands[0], Command::GroupAction { action: client_core::group_actions::Action::Create(users), .. } if users.len() == 9)
		);
	}
}
