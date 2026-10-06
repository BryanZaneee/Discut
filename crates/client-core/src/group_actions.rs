//! One explicit group write at a time; image data is never retained in state or echoed back.
use crate::{
	Command, State,
	auth::{AuthState, Failure},
};
use model::{ChannelPatch, Id, Patch};

pub const MAX_ICON_DATA_URI: usize = 22 + 4 * (256_usize * 1024).div_ceil(3);

#[derive(Clone)]
pub enum Action {
	Create(Vec<Id>),
	Recipient {
		channel: Id,
		user: Id,
		add: bool,
	},
	Leave(Id),
	Edit {
		channel: Id,
		name: Option<String>,
		icon: Patch<String>,
	},
}

impl Action {
	pub fn channel(&self) -> Id {
		match self {
			Self::Create(_) => Id(0),
			Self::Leave(channel) | Self::Edit { channel, .. } | Self::Recipient { channel, .. } => {
				*channel
			}
		}
	}
	pub fn valid(&self) -> bool {
		if let Self::Create(users) = self {
			return (2..=9).contains(&users.len())
				&& users.capacity() <= 16
				&& users.iter().all(|id| id.0 != 0)
				&& users
					.iter()
					.collect::<std::collections::BTreeSet<_>>()
					.len() == users.len();
		}
		self.channel().0 != 0
			&& match self {
				Self::Create(_) => unreachable!(),
				Self::Recipient { user, .. } => user.0 != 0,
				Self::Leave(_) => true,
				Self::Edit { name, icon, .. } => {
					(name.is_some() || !matches!(icon, Patch::Absent))
						&& name.as_ref().is_none_or(|name| {
							!name.trim().is_empty()
								&& name.chars().count() <= 100
								&& name.capacity() <= 400
								&& !name.chars().any(char::is_control)
						}) && match icon {
						Patch::Value(uri) => {
							uri.capacity() <= MAX_ICON_DATA_URI
								&& uri
									.strip_prefix("data:image/png;base64,")
									.is_some_and(|data| {
										!data.is_empty()
											&& data.bytes().all(|b| {
												b.is_ascii_alphanumeric() || b"+/=".contains(&b)
											})
									})
						}
						_ => true,
					}
				}
			}
	}
}
pub enum Event {
	Managed {
		channel: Id,
		request: u64,
		result: Result<Box<model::Channel>, Failure>,
	},
	Written {
		channel: Id,
		request: u64,
		result: Result<Option<ChannelPatch>, Failure>,
	},
}
#[derive(Default)]
pub struct Actions {
	creation_invalidated: bool,
	management: Option<Action>,
	sequence: u64,
	// channel, request, leaving, renaming, newer service change observed
	pending: Option<(Id, u64, bool, bool, bool)>,
	status: Option<(Id, &'static str)>,
	completed: Option<(Id, u64, bool)>,
}
impl Actions {
	pub(crate) fn reset(&mut self) {
		*self = Self {
			management: None,
			sequence: self.sequence,
			// READY can follow a canceled write without changing session generation.
			// Keep its bounded outcome so an open editor can leave the busy state.
			completed: self.completed,
			status: self.status,
			..Self::default()
		};
	}
}
impl State {
	/// Offline adapter for native fixtures; callers must already be in demo mode.
	pub fn demo_group_action(&self, action: Action, request: u64) -> Event {
		let channel = action.channel();
		if !self.demo {
			return Event::Written {
				channel,
				request,
				result: Err(Failure::Forbidden),
			};
		}
		match action {
			Action::Create(users) => {
				let result = self
					.channels
					.iter()
					.map(|c| c.id.0)
					.max()
					.unwrap_or(0)
					.checked_add(1)
					.ok_or(Failure::Capacity)
					.and_then(|id| {
						let recipients: Option<Vec<_>> =
							users.iter().map(|id| self.friend(*id).cloned()).collect();
						let recipients = recipients.ok_or(Failure::Forbidden)?;
						Ok(Box::new(model::Channel {
							id: Id(id),
							guild: None,
							kind: 3,
							name: "New group".into(),
							icon: None,
							last_message: None,
							parent_id: None,
							position: 0,
							recipients,
							member_list_id: None,
							tags: None,
							message_count: None,
						}))
					});
				Event::Managed {
					channel,
					request,
					result,
				}
			}
			Action::Recipient { user, add, .. } => {
				let result = self
					.channel(channel)
					.cloned()
					.ok_or(Failure::Forbidden)
					.and_then(|mut group| {
						group.recipients.retain(|member| member.id != user);
						if add {
							group
								.recipients
								.push(self.friend(user).cloned().ok_or(Failure::Forbidden)?);
						}
						Ok(Box::new(group))
					});
				Event::Managed {
					channel,
					request,
					result,
				}
			}
			Action::Leave(_) => Event::Written {
				channel,
				request,
				result: Ok(None),
			},
			Action::Edit { name, icon, .. } => Event::Written {
				channel,
				request,
				result: Ok(Some(ChannelPatch {
					id: channel,
					name: name.map_or(Patch::Absent, Patch::Value),
					icon: match icon {
						Patch::Absent => self
							.channel(channel)
							.and_then(|c| c.icon.clone())
							.map_or(Patch::Null, Patch::Value),
						Patch::Null => Patch::Null,
						Patch::Value(_) => Patch::Value("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into()),
					},
					last_message: Patch::Absent,
					parent_id: Patch::Absent,
					position: Patch::Absent,
					kind: Patch::Absent,
					message_count: Patch::Absent,
					tags: Patch::Absent,
				})),
			},
		}
	}

	pub fn group_action_pending(&self) -> bool {
		self.group_actions.pending.is_some()
	}
	pub fn group_action_status(&self, channel: Id) -> Option<&'static str> {
		self.group_actions
			.status
			.filter(|(id, _)| *id == channel)
			.map(|(_, text)| text)
	}
	pub fn group_action_completed(&self, channel: Id, request: u64) -> Option<bool> {
		self.group_actions
			.completed
			.filter(|(id, seq, _)| *id == channel && *seq == request)
			.map(|(_, _, success)| success)
	}
	pub fn clear_group_action_result(&mut self, channel: Id) {
		if self
			.group_actions
			.status
			.is_some_and(|(id, _)| id == channel)
		{
			self.group_actions.status = None;
		}
	}
	pub fn is_group_dm(&self, channel: Id) -> bool {
		self.channel(channel)
			.is_some_and(|c| c.guild.is_none() && c.kind == 3)
	}
	pub fn leave_group_reason(&self, channel: Id) -> Option<&'static str> {
		if !self.is_group_dm(channel) {
			return Some("Group is no longer available");
		}
		if self.pending.iter().any(|p| p.channel == channel)
			|| self
				.voice
				.active
				.as_ref()
				.is_some_and(|call| call.channel == channel)
		{
			return Some("Finish pending messages and leave the call before leaving this group");
		}
		None
	}
	pub fn leave_group(&mut self, channel: Id) -> Option<Command> {
		if let Some(reason) = self.leave_group_reason(channel) {
			self.group_actions.status = Some((channel, reason));
			return None;
		}
		self.request_group_action(Action::Leave(channel))
	}
	pub fn edit_group(
		&mut self,
		channel: Id,
		name: Option<String>,
		icon: Patch<String>,
	) -> Option<Command> {
		self.request_group_action(Action::Edit {
			channel,
			name,
			icon,
		})
	}

	/// Explicit creation only, limited to existing friends and Discord's ordinary group size.
	pub fn create_group(&mut self, users: Vec<Id>) -> Option<Command> {
		self.request_group_action(Action::Create(users))
	}
	pub fn change_group_recipient(&mut self, channel: Id, user: Id, add: bool) -> Option<Command> {
		self.request_group_action(Action::Recipient { channel, user, add })
	}
	fn request_group_action(&mut self, action: Action) -> Option<Command> {
		let channel = action.channel();
		if self.group_action_pending() || (channel != Id(0) && !self.is_group_dm(channel)) {
			return None;
		}
		if !action.valid() {
			self.group_actions.status = Some((
				channel,
				"Check the group name, icon, and selection of 2–9 distinct friends",
			));
			return None;
		}
		let allowed = match &action {
			Action::Create(users) => users.iter().all(|id| {
				self.friend(*id).is_some() && self.user.as_ref().is_none_or(|me| me.id != *id)
			}),
			Action::Recipient { user, add, .. } => {
				self.user.as_ref().is_none_or(|me| me.id != *user)
					&& self.channel(channel).is_some_and(|group| {
						let present = group.recipients.iter().any(|u| u.id == *user);
						if *add {
							!present
								&& self.friend(*user).is_some()
								&& group
									.recipients
									.iter()
									.filter(|u| self.user.as_ref().is_none_or(|me| me.id != u.id))
									.count() < 9
						} else {
							present
						}
					})
			}
			_ => true,
		};
		if !allowed {
			self.group_actions.status = Some((
				channel,
				"Choose an eligible friend or group member; use Leave group to remove yourself",
			));
			return None;
		}
		if !self.demo && (self.auth != AuthState::Authenticated || !self.gateway_connected) {
			self.group_actions.status =
				Some((channel, "Group actions unavailable while disconnected"));
			return None;
		}
		self.group_actions.management =
			matches!(&action, Action::Create(_) | Action::Recipient { .. }).then(|| action.clone());
		self.group_actions.creation_invalidated = false;
		self.group_actions.sequence = self.group_actions.sequence.wrapping_add(1);
		let request = self.group_actions.sequence;
		self.group_actions.pending = Some((
			channel,
			request,
			matches!(action, Action::Leave(_)),
			matches!(&action, Action::Edit { name: Some(_), .. }),
			false,
		));
		self.group_actions.status = None;
		self.group_actions.completed = None;
		Some(Command::GroupAction { action, request })
	}
	pub(crate) fn cancel_group_action(&mut self) {
		self.group_actions.management = None;
		if let Some((channel, request, _, _, _)) = self.group_actions.pending.take() {
			self.group_actions.status =
				Some((channel, "Outcome unknown; check Discord before retrying"));
			self.group_actions.completed = Some((channel, request, false));
		}
	}
	/// The create response ID is unknown until REST returns. Conservatively reject a late
	/// response if any channel disappears during the request; do not resurrect lost access.
	pub(crate) fn invalidate_pending_group_creation(&mut self) {
		if matches!(&self.group_actions.management, Some(Action::Create(_))) {
			self.group_actions.creation_invalidated = true;
		}
	}
	pub(crate) fn observe_group_recipient(&mut self, channel: Id, user: Id) {
		if matches!(&self.group_actions.management, Some(Action::Recipient { channel: target, user: recipient, .. }) if *target == channel && *recipient == user)
			&& let Some((_, _, _, _, observed)) = &mut self.group_actions.pending
		{
			*observed = true;
		}
	}
	pub(crate) fn observe_group_change(&mut self, channel: Id, recreated: bool) {
		if self.group_actions.management.is_some() && !recreated {
			return;
		}
		if let Some((target, _, leaving, _, observed)) = &mut self.group_actions.pending
			&& *target == channel
			&& (recreated || !*leaving)
		{
			*observed = true;
		}
	}
	fn apply_group_management(
		&mut self,
		channel: Id,
		request: u64,
		result: Result<Box<model::Channel>, Failure>,
	) -> Result<(), &'static str> {
		let Some((target, sequence, _, _, observed)) = self.group_actions.pending else {
			return Ok(());
		};
		if channel != target || request != sequence {
			return Ok(());
		}
		let Some(action) = self.group_actions.management.take() else {
			return Ok(());
		};
		self.group_actions.pending = None;
		let result = result.and_then(|group| {
			let unique: std::collections::BTreeSet<_> =
				group.recipients.iter().map(|u| u.id).collect();
			let shape = group.id.0 != 0
				&& group.guild.is_none()
				&& group.kind == 3
				&& group.recipients.len() <= 10
				&& group.bytes() <= 64 * 1024
				&& unique.len() == group.recipients.len()
				&& group.recipients.iter().all(|u| u.id.0 != 0 && !u.webhook);
			let matches = match &action {
				Action::Create(users) => {
					!self.group_actions.creation_invalidated
						&& users.iter().all(|id| unique.contains(id))
						&& unique.iter().all(|id| {
							users.contains(id) || self.user.as_ref().is_some_and(|me| me.id == *id)
						})
				}
				Action::Recipient { user, add, .. } => {
					group.id == channel && unique.contains(user) == *add
				}
				_ => false,
			};
			if shape && matches {
				Ok(group)
			} else {
				Err(Failure::Ambiguous)
			}
		});
		self.group_actions.completed = Some((channel, request, result.is_ok()));
		let status = match result {
			Err(failure) => {
				if failure.ends_session() {
					self.fail(failure);
				}
				failure.label()
			}
			Ok(group) => {
				let event = match action {
					Action::Create(_) if self.channel(group.id).is_none() => {
						Some(crate::Event::ChannelCreated(*group))
					}
					Action::Recipient {
						user, add: true, ..
					} if !observed => group
						.recipients
						.iter()
						.find(|u| u.id == user)
						.cloned()
						.map(|user| crate::Event::RecipientAdded { channel, user }),
					Action::Recipient {
						user, add: false, ..
					} if !observed => Some(crate::Event::RecipientRemoved { channel, user }),
					_ => None,
				};
				if let Some(event) = event {
					self.apply(crate::Envelope {
						generation: self.generation,
						event,
					});
				}
				"Group updated; open it from Direct Messages"
			}
		};
		self.group_actions.status = Some((channel, status));
		self.status = status;
		Ok(())
	}

	pub(crate) fn apply_group_action(&mut self, event: Event) -> Result<(), &'static str> {
		if let Event::Managed {
			channel,
			request,
			result,
		} = event
		{
			return self.apply_group_management(channel, request, result);
		}
		let Event::Written {
			channel,
			request,
			result,
		} = event
		else {
			unreachable!()
		};
		let Some((target, sequence, leaving, renaming, observed)) = self.group_actions.pending
		else {
			return Ok(());
		};
		if channel != target || request != sequence {
			return Ok(());
		}
		self.group_actions.pending = None;
		self.group_actions.management = None;
		let result = result.and_then(|patch| {
			if leaving && patch.is_none() {
				return Ok(None);
			}
			if !leaving
				&& patch.as_ref().is_some_and(|p| {
					p.id == channel
						&& match &p.name {
							Patch::Value(name) => {
								!name.is_empty()
									&& name.chars().count() <= 100
									&& name.capacity() <= 400
							}
							Patch::Absent => !renaming,
							Patch::Null => false,
						} && match &p.icon {
						Patch::Value(hash) => {
							model::valid_avatar_hash(hash) && hash.capacity() <= 128
						}
						Patch::Null => true,
						Patch::Absent => false,
					}
				}) {
				Ok(patch)
			} else {
				Err(Failure::Ambiguous)
			}
		});
		self.group_actions.completed = Some((channel, request, result.is_ok()));
		let status = match result {
			Err(failure) => {
				if failure.ends_session() {
					self.fail(failure);
				}
				failure.label()
			}
			Ok(_) if observed => "Request completed; latest group settings shown",
			Ok(Some(patch)) => {
				if let Some(group) = self
					.channels
					.iter_mut()
					.find(|c| c.id == channel && c.kind == 3 && c.guild.is_none())
				{
					if let Patch::Value(name) = patch.name {
						group.name = name;
					}
					group.icon = match patch.icon {
						Patch::Value(hash) => Some(hash),
						_ => None,
					};
					self.invalidate_navigation();
				}
				"Group updated"
			}
			Ok(None) => {
				self.remove_channels(&std::collections::BTreeSet::from([channel]));
				if self.selected == Some(channel) {
					self.arrived_home();
				}
				"Left group"
			}
		};
		self.group_actions.status = Some((channel, status));
		self.status = status;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{Envelope, Event as CoreEvent};
	fn user(id: u64) -> model::User {
		model::User {
			id: Id(id),
			name: format!("Friend{id}"),
			avatar: None,
			webhook: false,
			kind: Default::default(),
			discriminator: 0,
			primary_guild: None,
		}
	}
	fn friends_state() -> State {
		let mut s = state();
		s.user = Some(user(1));
		s.apply_user_action(crate::user_actions::Event::Relationships(Some(vec![])))
			.unwrap();
		s.apply_user_action(crate::user_actions::Event::Friends(Some(
			(2..=12)
				.map(|id| (user(id), format!("friend{id}")))
				.collect(),
		)))
		.unwrap();
		s
	}
	fn finish_demo(s: &mut State, command: Command) {
		let Command::GroupAction { action, request } = command else {
			panic!("group command expected")
		};
		let event = s.demo_group_action(action, request);
		s.apply(Envelope {
			generation: s.generation,
			event: CoreEvent::GroupAction(event),
		});
	}
	#[test]
	fn create_and_manage_groups_require_explicit_bounded_eligible_actions() {
		let mut s = friends_state();
		assert!(s.create_group(vec![Id(2)]).is_none());
		assert!(s.create_group(vec![Id(2), Id(2)]).is_none());
		assert!(s.create_group(vec![Id(1), Id(2)]).is_none());
		assert!(s.create_group(vec![Id(2), Id(99)]).is_none());
		assert!(s.create_group((2..=11).map(Id).collect()).is_none());
		let command = s.create_group(vec![Id(2), Id(3)]).unwrap();
		assert_eq!(s.channels.len(), 1); // No optimistic creation.
		assert!(s.create_group(vec![Id(4), Id(5)]).is_none());
		finish_demo(&mut s, command);
		assert_eq!(s.channels.len(), 2);
		let channel = s.channels.iter().find(|g| g.id != Id(10)).unwrap().id;
		assert_eq!(s.channel(channel).unwrap().recipients.len(), 2);
		assert!(s.change_group_recipient(channel, Id(1), false).is_none());
		assert!(s.change_group_recipient(channel, Id(2), true).is_none());
		assert!(s.change_group_recipient(channel, Id(99), true).is_none());
		let add = s.change_group_recipient(channel, Id(4), true).unwrap();
		assert_eq!(s.channel(channel).unwrap().recipients.len(), 2);
		finish_demo(&mut s, add);
		assert_eq!(s.channel(channel).unwrap().recipients.len(), 3);
		let remove = s.change_group_recipient(channel, Id(4), false).unwrap();
		finish_demo(&mut s, remove);
		assert_eq!(s.channel(channel).unwrap().recipients.len(), 2);
		s.demo = false;
		assert!(s.create_group(vec![Id(2), Id(3)]).is_none());
		assert!(s.change_group_recipient(channel, Id(4), true).is_none());
	}
	#[test]
	fn delayed_create_never_resurrects_group_after_gateway_access_loss() {
		for removed_by_self_event in [false, true] {
			let mut s = friends_state();
			let Command::GroupAction { action, request } =
				s.create_group(vec![Id(2), Id(3)]).unwrap()
			else {
				unreachable!()
			};
			let response = s.demo_group_action(action, request);
			let Event::Managed {
				result: Ok(group), ..
			} = &response
			else {
				unreachable!()
			};
			let channel = group.id;
			s.apply(Envelope {
				generation: s.generation,
				event: CoreEvent::ChannelCreated((**group).clone()),
			});
			assert!(s.channel(channel).is_some());
			let removal = if removed_by_self_event {
				CoreEvent::RecipientRemoved {
					channel,
					user: Id(1),
				}
			} else {
				CoreEvent::Unavailable(channel)
			};
			s.apply(Envelope {
				generation: s.generation,
				event: removal,
			});
			assert!(s.channel(channel).is_none());
			s.apply(Envelope {
				generation: s.generation,
				event: CoreEvent::GroupAction(response),
			});
			assert!(s.channel(channel).is_none());
			assert_eq!(s.group_action_completed(Id(0), request), Some(false));
			assert_eq!(
				s.group_action_status(Id(0)),
				Some(Failure::Ambiguous.label())
			);
		}
	}

	#[test]
	fn full_group_accepts_a_naturally_grown_bounded_selection() {
		let mut selected = Vec::new();
		for id in 2..=10 {
			selected.push(Id(id));
		}
		assert_eq!(selected.len(), 9);
		let mut s = friends_state();
		let command = s.create_group(selected).unwrap();
		finish_demo(&mut s, command);
		assert!(s.channels.iter().any(|group| group.recipients.len() == 9));
	}

	#[test]
	fn group_management_rejects_wrong_roster_stale_completion_and_preserves_newer_events() {
		let mut s = friends_state();
		let first = s.create_group(vec![Id(2), Id(3)]).unwrap();
		let Command::GroupAction { action, request } = first else {
			unreachable!()
		};
		let mut event = s.demo_group_action(action, request);
		if let Event::Managed {
			result: Ok(group), ..
		} = &mut event
		{
			group.recipients.push(user(99));
		}
		s.apply_group_action(event).unwrap();
		assert_eq!(s.channels.len(), 1);
		assert_eq!(s.group_action_completed(Id(0), request), Some(false));
		let first = s.create_group(vec![Id(2), Id(3)]).unwrap();
		s.cancel_group_action();
		let second = s.create_group(vec![Id(4), Id(5)]).unwrap();
		finish_demo(&mut s, first);
		assert_eq!(s.channels.len(), 1);
		assert!(s.group_action_pending());
		finish_demo(&mut s, second);
		let channel = s.channels.iter().find(|g| g.id != Id(10)).unwrap().id;
		let add = s.change_group_recipient(channel, Id(6), true).unwrap();
		let Command::GroupAction { action, request } = add else {
			unreachable!()
		};
		let stale = s.demo_group_action(action, request);
		s.apply(Envelope {
			generation: s.generation,
			event: CoreEvent::RecipientAdded {
				channel,
				user: user(6),
			},
		});
		s.apply(Envelope {
			generation: s.generation,
			event: CoreEvent::RecipientRemoved {
				channel,
				user: Id(6),
			},
		});
		s.apply_group_action(stale).unwrap();
		assert!(
			!s.channel(channel)
				.unwrap()
				.recipients
				.iter()
				.any(|u| u.id == Id(6))
		);
		let add = s.change_group_recipient(channel, Id(6), true).unwrap();
		s.command_rejected(add);
		assert!(!s.group_action_pending());
		assert!(
			!s.channel(channel)
				.unwrap()
				.recipients
				.iter()
				.any(|u| u.id == Id(6))
		);
	}

	fn state() -> State {
		State {
			demo: true,
			selected: Some(Id(10)),
			channels: vec![model::Channel {
				id: Id(10),
				guild: None,
				kind: 3,
				name: "Group".into(),
				icon: None,
				last_message: None,
				parent_id: None,
				position: 0,
				recipients: vec![],
				member_list_id: None,
				tags: None,
				message_count: None,
			}],
			..State::default()
		}
	}
	fn patch(name: &str) -> ChannelPatch {
		ChannelPatch {
			id: Id(10),
			name: Patch::Value(name.into()),
			icon: Patch::Null,
			last_message: Patch::Absent,
			parent_id: Patch::Absent,
			position: Patch::Absent,
			kind: Patch::Absent,
			message_count: Patch::Absent,
			tags: Patch::Absent,
		}
	}
	fn finish(state: &mut State, command: Command, result: Result<Option<ChannelPatch>, Failure>) {
		let Command::GroupAction { action, request } = command else {
			panic!("wrong command");
		};
		state.apply(Envelope {
			generation: state.generation,
			event: CoreEvent::GroupAction(Event::Written {
				channel: action.channel(),
				request,
				result,
			}),
		});
	}
	#[test]
	fn group_actions_confirm_writes_keep_drafts_and_respect_newer_service_state() {
		{
			let mut state = state();
			assert!(state.close_dm(Id(10)).is_none());
			assert!(state.set_dm_muted(Id(10), true).is_some());
			assert!(
				state
					.edit_group(Id(10), Some(" ".into()), Patch::Absent)
					.is_none()
			);
			assert!(
				state
					.edit_group(Id(10), Some("x".repeat(101)), Patch::Absent)
					.is_none()
			);
			let edit = state
				.edit_group(Id(10), Some("New name".into()), Patch::Absent)
				.unwrap();
			assert_eq!(state.channel(Id(10)).unwrap().name, "Group");
			assert!(state.leave_group(Id(10)).is_none());
			finish(&mut state, edit, Err(Failure::Forbidden));
			assert_eq!(state.group_action_completed(Id(10), 1), Some(false));
			assert_eq!(state.channel(Id(10)).unwrap().name, "Group");
			let edit = state
				.edit_group(Id(10), Some("New name".into()), Patch::Absent)
				.unwrap();
			finish(&mut state, edit, Ok(Some(patch("New name"))));
			assert_eq!(state.channel(Id(10)).unwrap().name, "New name");
			assert_eq!(state.group_action_completed(Id(10), 2), Some(true));
			let edit = state
				.edit_group(Id(10), Some("Older".into()), Patch::Absent)
				.unwrap();
			state.apply(Envelope {
				generation: state.generation,
				event: CoreEvent::ChannelChanged(patch("Latest")),
			});
			finish(&mut state, edit, Ok(Some(patch("Older"))));
			assert_eq!(state.channel(Id(10)).unwrap().name, "Latest");
			let stale = state.leave_group(Id(10)).unwrap();
			state.cancel_group_action();
			let current = state.leave_group(Id(10)).unwrap();
			finish(&mut state, stale, Ok(None));
			assert!(state.group_action_pending());
			state.drafts.insert(Id(10), "Keep draft".into());
			finish(&mut state, current, Ok(None));
			assert!(state.channel(Id(10)).is_none());
			assert_eq!(state.selected, None);
			assert_eq!(state.drafts[&Id(10)], "Keep draft");
		}
		{
			let mut state = state();
			let name = "A long recipient name, ".repeat(8);
			state.channels[0].name = name.clone();
			state.channels[0].icon = Some("0123456789abcdef0123456789abcdef".into());
			assert!(state.edit_group(Id(10), None, Patch::Absent).is_none());
			let edit = state.edit_group(Id(10), None, Patch::Null).unwrap();
			let mut response = patch("unused");
			response.name = Patch::Absent;
			finish(&mut state, edit, Ok(Some(response)));
			assert_eq!(state.channels[0].name, name);
			assert!(state.channels[0].icon.is_none());
			let rename = state
				.edit_group(Id(10), Some("Renamed".into()), Patch::Absent)
				.unwrap();
			let mut response = patch("unused");
			response.name = Patch::Absent;
			finish(&mut state, rename, Ok(Some(response)));
			assert_eq!(state.group_action_completed(Id(10), 2), Some(false));
			assert_eq!(state.channels[0].name, name);
		}
	}
	#[test]
	fn group_leave_guards_rejoined_channel_pending_messages_and_session_reset() {
		let mut state = state();
		state.pending.push(crate::Pending {
			sticker: None,
			channel: Id(10),
			nonce: "pending".into(),
			content: "pending".into(),
			attachments: vec![],
			delivery: crate::Delivery::Sending,
			confirmed: None,
		});
		assert!(state.leave_group(Id(10)).is_none());
		state.pending.clear();
		let leave = state.leave_group(Id(10)).unwrap();
		let channel = state.channel(Id(10)).unwrap().clone();
		state.apply(Envelope {
			generation: state.generation,
			event: CoreEvent::Unavailable(Id(10)),
		});
		state.apply(Envelope {
			generation: state.generation,
			event: CoreEvent::ChannelCreated(channel),
		});
		finish(&mut state, leave, Ok(None));
		assert!(state.channel(Id(10)).is_some());
		let edit = state
			.edit_group(Id(10), Some("New name".into()), Patch::Absent)
			.unwrap();
		state.command_rejected(edit);
		assert!(!state.group_action_pending());
		let interrupted = state
			.edit_group(Id(10), Some("Interrupted".into()), Patch::Absent)
			.unwrap();
		let request = match &interrupted {
			Command::GroupAction { request, .. } => *request,
			_ => unreachable!(),
		};
		state.apply(Envelope {
			generation: state.generation,
			event: CoreEvent::Ready {
				user: model::User {
					primary_guild: None,
					id: Id(1),
					name: "Synthetic".into(),
					avatar: None,
					webhook: false,
					kind: Default::default(),
					discriminator: 0,
				},
				channels: state.channels.clone(),
				guilds: vec![],
				permissions: Default::default(),
			},
		});
		assert_eq!(state.group_action_completed(Id(10), request), Some(false));
		finish(&mut state, interrupted, Ok(Some(patch("Interrupted"))));
		assert_eq!(state.channel(Id(10)).unwrap().name, "Group");
		state.demo = false;
		state.gateway_connected = false;
		assert!(state.leave_group(Id(10)).is_none());
		state.demo = true;
		let leave = state.leave_group(Id(10)).unwrap();
		let Command::GroupAction { action, request } = leave else {
			unreachable!()
		};
		let generation = state.generation;
		state.logout();
		state.apply(Envelope {
			generation,
			event: CoreEvent::GroupAction(Event::Written {
				channel: action.channel(),
				request,
				result: Ok(None),
			}),
		});
		assert_eq!(state.group_action_completed(Id(10), request), None);
	}
}
