use ui::MessagingUi;

#[test]
fn session_clear_keeps_window_preference_but_removes_account_sharing_actions() {
	let mut view = MessagingUi::default();
	view.minimize_to_tray = true;
	view.tray_available = true;
	view.discord_activity_sharing = Some(false);
	view.discord_activity_sharing_request = Some(true);
	view.clear();
	assert!(view.minimize_to_tray && view.tray_available);
	assert_eq!(view.discord_activity_sharing, None);
	assert_eq!(view.discord_activity_sharing_request, None);
}

#[test]
fn settings_search_cannot_enable_removed_activity_sharing() {
	for dark in [true, false] {
		for width in [560.0, 1120.0] {
			for query in ["activity", "registered"] {
				let ctx = egui::Context::default();
				ctx.set_theme(if dark {
					egui::ThemePreference::Dark
				} else {
					egui::ThemePreference::Light
				});
				ui::design::apply(&ctx);
				let mut state = test_support::demo_state();
				state.demo = false;
				state.gateway_connected = true;
				let mut view = MessagingUi::default();
				// Even a previous enabled preference must not expose a settings write.
				view.share_game_activity = true;
				view.discord_activity_sharing = Some(false);
				view.preview_settings("appearance");
				let mut frame = |view: &mut MessagingUi, events| {
					let output = ctx.run_ui(
						egui::RawInput {
							screen_rect: Some(egui::Rect::from_min_size(
								egui::Pos2::ZERO,
								egui::vec2(width, 760.0),
							)),
							events,
							..Default::default()
						},
						|ui| {
							// Account activity writes use the request field checked below.
							view.show(ui, &mut state);
						},
					);
					fn collect(shape: &egui::Shape, text: &mut Vec<(String, egui::Pos2)>) {
						match shape {
							egui::Shape::Text(value) => text.push((
								value.galley.job.text.clone(),
								value.pos + value.galley.size() / 2.0,
							)),
							egui::Shape::Vec(shapes) => {
								for shape in shapes {
									collect(shape, text);
								}
							}
							_ => {}
						}
					}
					let mut text = vec![];
					for shape in &output.shapes {
						collect(&shape.shape, &mut text);
					}
					for forbidden in [
						"Registered Games",
						"Game Activity",
						"Share game activity",
						"Enable on Discord",
						"Disable on Discord",
						"Current Game",
						"Added Games",
					] {
						assert!(
							!text.iter().any(|(label, _)| label == forbidden),
							"Removed control visible: {forbidden}"
						);
					}
					assert_eq!(view.discord_activity_sharing_request, None);
					assert!(!view.running_processes_request);
					assert!(output.platform_output.commands.is_empty());
					output.drop_without_applying_deltas();
					text
				};
				for _ in 0..3 {
					frame(&mut view, vec![]);
				}
				let text = frame(&mut view, vec![]);
				let position = text
					.iter()
					.rev()
					.find(|(label, _)| label == "Search")
					.expect("Settings search is visible")
					.1;
				for pressed in [true, false] {
					frame(
						&mut view,
						vec![
							egui::Event::PointerMoved(position),
							egui::Event::PointerButton {
								pos: position,
								button: egui::PointerButton::Primary,
								pressed,
								modifiers: Default::default(),
							},
						],
					);
				}
				frame(&mut view, vec![egui::Event::Text(query.into())]);
				let text = frame(&mut view, vec![]);
				assert!(
					text.iter().any(|(label, _)| label == "No settings found"),
					"Removed feature must not be searchable: {query}"
				);
			}
		}
	}
}
