use crate::{Channel, Id};

/// Which device-local shortcut list a channel belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Shortcut {
	Pinned,
	Favorite,
}

/// Outcome of one shortcut write.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreferenceEdit {
	Unchanged,
	Changed,
	CapacityReached,
}

/// Device-local channel navigation preferences, isolated by account; never synchronized to Discord.
/// At most 256 IDs (2 KiB of live ID payload) across all lists; retained capacity is 10 KiB.
/// Vec order is display order: index 0 is shown first, and a new entry goes to the front.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ChannelPreferences {
	pub favorites: Vec<Id>,
	pub pinned: Vec<Id>,
	pub collapsed_categories: Vec<Id>,
	/// Local exclusions only; empty lists include every service-provided conversation.
	pub excluded_guilds: Vec<Id>,
	pub excluded_channels: Vec<Id>,
	/// Last opened channel per server, oldest first; bounded separately from the lists above.
	pub last_channels: Vec<(Id, Id)>,
}

impl ChannelPreferences {
	pub const MAX_ENTRIES: usize = 256;
	pub const MAX_JSON_BYTES: usize = 8192;
	/// Keeps the worst case (256 list IDs plus these pairs of 20-digit IDs) under the JSON cap.
	pub const MAX_LAST_CHANNELS: usize = 32;

	pub fn is_valid(&self) -> bool {
		// ponytail: duplicate scans are capped at 256 IDs; use a set if this limit grows.
		self.entry_count() <= Self::MAX_ENTRIES
			&& self.lists().iter().map(|ids| ids.capacity()).sum::<usize>() <= Self::MAX_ENTRIES * 5
			&& self.last_channels.len() <= Self::MAX_LAST_CHANNELS
			&& self.last_channels.capacity() <= Self::MAX_LAST_CHANNELS * 2
			&& self
				.last_channels
				.iter()
				.enumerate()
				.all(|(index, (guild, channel))| {
					guild.0 != 0
						&& channel.0 != 0 && !self.last_channels[..index]
						.iter()
						.any(|(id, _)| id == guild)
				}) && self.lists().into_iter().all(|ids| {
			ids.iter()
				.enumerate()
				.all(|(index, id)| id.0 != 0 && !ids[..index].contains(id))
		})
	}

	fn lists(&self) -> [&Vec<Id>; 5] {
		[
			&self.favorites,
			&self.pinned,
			&self.collapsed_categories,
			&self.excluded_guilds,
			&self.excluded_channels,
		]
	}

	fn entry_count(&self) -> usize {
		self.lists().iter().map(|ids| ids.len()).sum()
	}

	pub fn guild_included(&self, guild: Id) -> bool {
		!self.excluded_guilds.contains(&guild)
	}

	pub fn channel_included(&self, channel: &Channel) -> bool {
		channel.guild.is_none_or(|guild| self.guild_included(guild))
			&& !self.excluded_channels.contains(&channel.id)
	}

	pub fn set_guild_included(&mut self, guild: Id, included: bool) -> PreferenceEdit {
		self.set_included(guild, included, true)
	}

	pub fn set_channel_included(&mut self, channel: Id, included: bool) -> PreferenceEdit {
		self.set_included(channel, included, false)
	}

	fn set_included(&mut self, id: Id, included: bool, guild: bool) -> PreferenceEdit {
		if id.0 == 0 || !self.is_valid() {
			return PreferenceEdit::Unchanged;
		}
		let full = self.entry_count() == Self::MAX_ENTRIES;
		let excluded = if guild {
			&mut self.excluded_guilds
		} else {
			&mut self.excluded_channels
		};
		if excluded.contains(&id) != included {
			return PreferenceEdit::Unchanged;
		}
		if included {
			excluded.retain(|entry| *entry != id);
		} else {
			if full {
				return PreferenceEdit::CapacityReached;
			}
			excluded.push(id);
		}
		PreferenceEdit::Changed
	}

	fn list(&self, kind: Shortcut) -> &Vec<Id> {
		match kind {
			Shortcut::Pinned => &self.pinned,
			Shortcut::Favorite => &self.favorites,
		}
	}

	/// Display-ordered ids of one list.
	pub fn ids(&self, kind: Shortcut) -> &[Id] {
		self.list(kind)
	}

	pub fn contains(&self, kind: Shortcut, channel: Id) -> bool {
		self.list(kind).contains(&channel)
	}

	/// Home pins are 1:1 and group DMs only.
	pub fn can_pin(channel: &Channel) -> bool {
		channel.id.0 != 0 && channel.guild.is_none() && matches!(channel.kind, 1 | 3)
	}

	/// Server favorites are guild channels that are not categories.
	pub fn can_favorite(channel: &Channel) -> bool {
		channel.id.0 != 0 && channel.guild.is_some() && channel.kind != 4
	}

	/// Idempotent. Turning a shortcut on inserts it at the front of its list.
	pub fn set(&mut self, kind: Shortcut, channel: Id, on: bool) -> PreferenceEdit {
		if channel.0 == 0 || !self.is_valid() || self.contains(kind, channel) == on {
			return PreferenceEdit::Unchanged;
		}
		let full = self.entry_count() == Self::MAX_ENTRIES;
		let ids = match kind {
			Shortcut::Pinned => &mut self.pinned,
			Shortcut::Favorite => &mut self.favorites,
		};
		if on {
			if full {
				return PreferenceEdit::CapacityReached;
			}
			ids.insert(0, channel);
		} else {
			ids.retain(|id| *id != channel);
		}
		PreferenceEdit::Changed
	}

	pub fn category_collapsed(&self, category: Id) -> bool {
		self.collapsed_categories.contains(&category)
	}

	pub fn set_category_collapsed(&mut self, category: Id, collapsed: bool) -> PreferenceEdit {
		if category.0 == 0 || !self.is_valid() || self.category_collapsed(category) == collapsed {
			return PreferenceEdit::Unchanged;
		}
		if collapsed {
			if self.entry_count() == Self::MAX_ENTRIES {
				return PreferenceEdit::CapacityReached;
			}
			self.collapsed_categories.push(category);
		} else {
			self.collapsed_categories.retain(|id| *id != category);
		}
		PreferenceEdit::Changed
	}

	/// Records the channel a server reopens to; returns whether anything changed.
	pub fn remember_channel(&mut self, guild: Id, channel: Id) -> bool {
		if guild.0 == 0 || channel.0 == 0 || self.last_channels.last() == Some(&(guild, channel)) {
			return false;
		}
		self.last_channels.retain(|(id, _)| *id != guild);
		if self.last_channels.len() >= Self::MAX_LAST_CHANNELS {
			self.last_channels.remove(0);
		}
		self.last_channels.push((guild, channel));
		true
	}

	/// Drops a confirmed-deleted channel from every local navigation list.
	pub fn forget(&mut self, channel: Id) -> bool {
		let count = |value: &Self| {
			value.favorites.len()
				+ value.pinned.len()
				+ value.collapsed_categories.len()
				+ value.excluded_channels.len()
				+ value.last_channels.len()
		};
		let before = count(self);
		self.favorites.retain(|id| *id != channel);
		self.pinned.retain(|id| *id != channel);
		self.collapsed_categories.retain(|id| *id != channel);
		self.excluded_channels.retain(|id| *id != channel);
		self.last_channels.retain(|(_, id)| *id != channel);
		before != count(self)
	}
}
