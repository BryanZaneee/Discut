use crate::{DiscordApi, Failure};
use client_core::group_actions::Action;
use model::{ChannelPatch, Patch};
use reqwest::Method;

impl DiscordApi {
	pub(super) async fn execute_group_action(
		&self,
		action: Action,
		request: u64,
	) -> client_core::group_actions::Event {
		use client_core::group_actions::Event;
		let channel = action.channel();
		if matches!(&action, Action::Create(_) | Action::Recipient { .. }) {
			Event::Managed {
				channel,
				request,
				result: self.manage_group(action).await.map(Box::new),
			}
		} else {
			Event::Written {
				channel,
				request,
				result: self.group_action(action).await,
			}
		}
	}

	/// Ordinary-user recipients payload is unofficial; live acceptance is unverified.
	/// References: Discord's User/Channel resources and docs.discord.food/resources/channel.
	/// Never retry a write or bypass a challenge. Confirm membership with a bounded read.
	async fn manage_group(&self, action: Action) -> Result<model::Channel, Failure> {
		if !action.valid() {
			return Err(Failure::Protocol);
		}
		let (method, path, body) = match &action {
			Action::Create(users) => (
				Method::POST,
				"/users/@me/channels".to_owned(),
				Some(serde_json::json!({"recipients":users})),
			),
			Action::Recipient { channel, user, add } => (
				if *add { Method::PUT } else { Method::DELETE },
				format!("/channels/{channel}/recipients/{user}"),
				None,
			),
			_ => return Err(Failure::Protocol),
		};
		let bytes = self
			.request_limited(method, &path, body, 64 * 1024)
			.await
			.map_err(|f| {
				if f == Failure::Capacity {
					Failure::Ambiguous
				} else {
					f
				}
			})?;
		let bytes = if let Action::Recipient { channel, .. } = &action {
			if !bytes.is_empty() {
				return Err(Failure::Ambiguous);
			}
			self.request_limited(
				Method::GET,
				&format!("/channels/{channel}"),
				None,
				64 * 1024,
			)
			.await
			.map_err(|f| {
				if f.ends_session() {
					f
				} else {
					Failure::Ambiguous
				}
			})?
		} else {
			bytes
		};
		let dto: discord_protocol::ChannelDto =
			discord_protocol::decode(&bytes).map_err(|_| Failure::Ambiguous)?;
		let group = dto.into_model();
		let users: std::collections::BTreeSet<_> = group.recipients.iter().map(|u| u.id).collect();
		let valid = group.id.0 != 0
			&& group.guild.is_none()
			&& group.kind == 3
			&& group.bytes() <= 64 * 1024
			&& group.recipients.len() <= 10
			&& users.len() == group.recipients.len()
			&& group.recipients.iter().all(|u| u.id.0 != 0 && !u.webhook)
			&& match action {
				Action::Create(expected) => expected.iter().all(|id| users.contains(id)),
				Action::Recipient { channel, user, add } => {
					group.id == channel && users.contains(&user) == add
				}
				_ => false,
			};
		if valid {
			Ok(group)
		} else {
			Err(Failure::Ambiguous)
		}
	}

	// Documented channel routes; ordinary-user session interoperability remains unverified.
	pub(super) async fn group_action(
		&self,
		action: Action,
	) -> Result<Option<ChannelPatch>, Failure> {
		if !action.valid() {
			return Err(Failure::Protocol);
		}
		let channel = action.channel();
		let leaving = matches!(action, Action::Leave(_));
		let renaming = matches!(&action, Action::Edit { name: Some(_), .. });
		let payload = match action {
			Action::Create(_) | Action::Recipient { .. } => return Err(Failure::Protocol),
			Action::Leave(_) => None,
			Action::Edit { name, icon, .. } => {
				let mut body = serde_json::json!({});
				if let Some(name) = name {
					body["name"] = name.into();
				}
				match icon {
					Patch::Absent => {}
					Patch::Null => body["icon"] = serde_json::Value::Null,
					Patch::Value(data) => {
						if !discord_protocol::group_actions::valid_icon_data_uri(&data) {
							return Err(Failure::Protocol);
						}
						body["icon"] = data.into();
					}
				}
				Some(body)
			}
		};
		let bytes = self
			.request_limited(
				if leaving {
					Method::DELETE
				} else {
					Method::PATCH
				},
				&format!("/channels/{channel}"),
				payload,
				64 * 1024,
			)
			.await
			.map_err(|f| {
				if f == Failure::Capacity {
					Failure::Ambiguous
				} else {
					f
				}
			})?;
		if leaving && bytes.is_empty() {
			return Ok(None);
		}
		let dto: discord_protocol::ChannelDto =
			discord_protocol::decode(&bytes).map_err(|_| Failure::Ambiguous)?;
		if dto.id != channel || dto.guild_id.is_some() || dto.kind != 3 {
			return Err(Failure::Ambiguous);
		}
		if leaving {
			return Ok(None);
		}
		let name = match dto.name {
			Some(name)
				if !name.is_empty() && name.chars().count() <= 100 && name.capacity() <= 400 =>
			{
				Patch::Value(name)
			}
			None if !renaming => Patch::Absent,
			_ => return Err(Failure::Ambiguous),
		};
		let icon = match dto.icon {
			Some(hash) if model::valid_avatar_hash(&hash) && hash.capacity() <= 128 => {
				Patch::Value(hash)
			}
			Some(_) => return Err(Failure::Ambiguous),
			None => Patch::Null,
		};
		Ok(Some(ChannelPatch {
			id: channel,
			name,
			icon,
			last_message: Patch::Absent,
			parent_id: Patch::Absent,
			position: Patch::Absent,
			kind: Patch::Absent,
			message_count: Patch::Absent,
			tags: Patch::Absent,
		}))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use client_core::{Command, Event, auth::SessionSecret};
	use model::Id;
	use std::sync::Arc;
	use tokio::{
		io::{AsyncReadExt, AsyncWriteExt},
		net::TcpListener,
	};
	async fn reply(
		listener: &TcpListener,
		method: &str,
		path: &str,
		payload: Option<serde_json::Value>,
		status: u16,
		body: &str,
	) {
		let (mut socket, _) = listener.accept().await.unwrap();
		let mut bytes = Vec::new();
		loop {
			let mut chunk = [0; 1024];
			let count = socket.read(&mut chunk).await.unwrap();
			assert!(count > 0);
			bytes.extend_from_slice(&chunk[..count]);
			assert!(bytes.len() <= 4096);
			if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
				let headers = std::str::from_utf8(&bytes[..end]).unwrap();
				let len = headers
					.lines()
					.find_map(|line| {
						line.to_ascii_lowercase()
							.strip_prefix("content-length: ")
							.map(str::to_owned)
					})
					.map_or(0, |value| value.parse::<usize>().unwrap());
				if bytes.len() < end + 4 + len {
					continue;
				}
				assert!(headers.starts_with(&format!("{method} {path} HTTP/1.1\r\n")));
				let body = (len > 0).then(|| {
					serde_json::from_slice::<serde_json::Value>(&bytes[end + 4..end + 4 + len])
						.unwrap()
				});
				assert_eq!(body, payload);
				break;
			}
		}
		socket
			.write_all(
				format!(
					"HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
					body.len()
				)
				.as_bytes(),
			)
			.await
			.unwrap();
	}
	#[tokio::test]
	async fn group_creation_and_membership_confirm_identity_without_write_retries() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let mut api = DiscordApi::new(Arc::new(
			SessionSecret::from_owner_input("SYNTHETIC_GROUP_MANAGEMENT".into()).unwrap(),
		))
		.unwrap();
		api.base = format!("http://{}", listener.local_addr().unwrap());
		let initial = r#"{"id":"10","type":3,"recipients":[{"id":"2","username":"Two"},{"id":"3","username":"Three"}]}"#;
		let added = r#"{"id":"10","type":3,"recipients":[{"id":"2","username":"Two"},{"id":"3","username":"Three"},{"id":"4","username":"Four"}]}"#;
		let removed = r#"{"id":"10","type":3,"recipients":[{"id":"2","username":"Two"}]}"#;
		for (action, method, path, payload, response, read, success) in [
			(
				Action::Create(vec![Id(2), Id(3)]),
				"POST",
				"/users/@me/channels",
				Some(serde_json::json!({"recipients":["2","3"]})),
				initial,
				None,
				true,
			),
			(
				Action::Create(vec![Id(2), Id(3)]),
				"POST",
				"/users/@me/channels",
				Some(serde_json::json!({"recipients":["2","3"]})),
				removed,
				None,
				false,
			),
			(
				Action::Recipient {
					channel: Id(10),
					user: Id(4),
					add: true,
				},
				"PUT",
				"/channels/10/recipients/4",
				None,
				"",
				Some(added),
				true,
			),
			(
				Action::Recipient {
					channel: Id(10),
					user: Id(3),
					add: false,
				},
				"DELETE",
				"/channels/10/recipients/3",
				None,
				"",
				Some(removed),
				true,
			),
			(
				Action::Recipient {
					channel: Id(10),
					user: Id(3),
					add: false,
				},
				"DELETE",
				"/channels/10/recipients/3",
				None,
				"",
				Some(initial),
				false,
			),
		] {
			let server = async {
				reply(
					&listener,
					method,
					path,
					payload,
					if response.is_empty() { 204 } else { 200 },
					response,
				)
				.await;
				if let Some(body) = read {
					reply(&listener, "GET", "/channels/10", None, 200, body).await;
				}
			};
			let (event, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
				tokio::join!(
					api.execute(Command::GroupAction { action, request: 7 }),
					server
				)
			})
			.await
			.unwrap();
			let Event::GroupAction(client_core::group_actions::Event::Managed {
				request,
				result,
				..
			}) = event
			else {
				panic!("wrong event")
			};
			assert_eq!(request, 7);
			assert_eq!(result.is_ok(), success);
		}
		let action = Action::Recipient {
			channel: Id(10),
			user: Id(3),
			add: false,
		};
		let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
			tokio::join!(
				api.manage_group(action),
				reply(
					&listener,
					"DELETE",
					"/channels/10/recipients/3",
					None,
					403,
					"{}"
				)
			)
		})
		.await
		.unwrap();
		assert!(matches!(result, Err(Failure::Forbidden)));
		assert!(
			tokio::time::timeout(std::time::Duration::from_millis(30), listener.accept())
				.await
				.is_err()
		);
		assert!(matches!(
			api.manage_group(Action::Create(vec![Id(2)])).await,
			Err(Failure::Protocol)
		));
	}

	#[tokio::test]
	async fn group_routes_validate_outcomes_and_never_retry_uncertain_writes() {
		let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
		let mut api = DiscordApi::new(Arc::new(
			SessionSecret::from_owner_input("SYNTHETIC_GROUP_TOKEN".into()).unwrap(),
		))
		.unwrap();
		api.base = format!("http://{}", listener.local_addr().unwrap());
		for (action, method, payload, status, body, success) in [
			(
				Action::Edit {
					channel: Id(10),
					name: None,
					icon: Patch::Null,
				},
				"PATCH",
				Some(serde_json::json!({"icon":null})),
				200,
				r#"{"id":"10","type":3,"name":null,"icon":null}"#,
				true,
			),
			(
				Action::Edit {
					channel: Id(10),
					name: Some("Renamed".into()),
					icon: Patch::Absent,
				},
				"PATCH",
				Some(serde_json::json!({"name":"Renamed"})),
				200,
				r#"{"id":"10","type":3,"name":null,"icon":null}"#,
				false,
			),
			(
				Action::Edit {
					channel: Id(10),
					name: Some("Renamed".into()),
					icon: Patch::Absent,
				},
				"PATCH",
				Some(serde_json::json!({"name":"Renamed"})),
				200,
				r#"{"id":"10","type":3,"name":"Renamed","icon":null}"#,
				true,
			),
			(
				Action::Edit {
					channel: Id(10),
					name: Some("Renamed".into()),
					icon: Patch::Null,
				},
				"PATCH",
				Some(serde_json::json!({"name":"Renamed","icon":null})),
				200,
				r#"{"id":"11","type":3,"name":"Wrong"}"#,
				false,
			),
			(
				Action::Leave(Id(10)),
				"DELETE",
				None,
				200,
				r#"{"id":"10","type":3}"#,
				true,
			),
			(Action::Leave(Id(10)), "DELETE", None, 500, r#"{}"#, false),
		] {
			let server = async {
				let (mut socket, _) = listener.accept().await.unwrap();
				let mut bytes = Vec::new();
				loop {
					let mut chunk = [0; 1024];
					let n = socket.read(&mut chunk).await.unwrap();
					assert!(n > 0);
					bytes.extend_from_slice(&chunk[..n]);
					assert!(bytes.len() <= 4096);
					if let Some(end) = bytes.windows(4).position(|b| b == b"\r\n\r\n") {
						let headers = std::str::from_utf8(&bytes[..end]).unwrap();
						let len = headers
							.lines()
							.find_map(|h| {
								h.to_ascii_lowercase()
									.strip_prefix("content-length: ")
									.map(str::to_owned)
							})
							.map_or(0, |n| n.parse::<usize>().unwrap());
						if bytes.len() < end + 4 + len {
							continue;
						}
						assert!(
							headers.starts_with(&format!("{method} /channels/10 HTTP/1.1\r\n"))
						);
						assert_eq!(
							if len == 0 {
								None
							} else {
								Some(
									serde_json::from_slice::<serde_json::Value>(&bytes[end + 4..])
										.unwrap(),
								)
							},
							payload
						);
						break;
					}
				}
				socket
					.write_all(
						format!(
							"HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
							body.len()
						)
						.as_bytes(),
					)
					.await
					.unwrap();
			};
			let (event, ()) = tokio::join!(
				api.execute(Command::GroupAction { action, request: 1 }),
				server
			);
			let Event::GroupAction(client_core::group_actions::Event::Written { result, .. }) =
				event
			else {
				panic!("wrong event");
			};
			assert_eq!(result.is_ok(), success);
			if success && body.contains("\"name\":null") {
				assert!(matches!(
					result,
					Ok(Some(ChannelPatch {
						name: Patch::Absent,
						..
					}))
				));
			}
		}
		assert!(matches!(
			api.group_action(Action::Edit {
				channel: Id(10),
				name: Some("Valid".into()),
				icon: Patch::Value("data:image/png;base64,bad".into())
			})
			.await,
			Err(Failure::Protocol)
		));
		assert!(
			tokio::time::timeout(std::time::Duration::from_millis(10), listener.accept())
				.await
				.is_err()
		);
	}
}
