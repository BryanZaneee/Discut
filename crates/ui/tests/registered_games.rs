use model::registered_games::{RegisteredGame, RunningGame};
use ui::MessagingUi;

#[test]
fn removed_game_settings_never_scan_or_modify_previous_game_preferences() {
	for dark in [true, false] {
		for width in [560.0, 1120.0] {
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
			view.share_game_activity = true;
			view.running_game = Some(RunningGame {
				executable: "synthetic.exe".into(),
				name: "Synthetic game".into(),
				application: Some(model::Id(7)),
				renamed: false,
			});
			view.registered_games = vec![RegisteredGame {
				executable: "synthetic.exe".into(),
				name: "Synthetic game".into(),
				application: Some(model::Id(7)),
				hidden: false,
				last_played: None,
			}];
			let original = view.registered_games.clone();
			for keyword in ["registered", "activity"] {
				view.preview_settings(keyword);
				for _ in 0..3 {
					let output = ctx.run_ui(
						egui::RawInput {
							screen_rect: Some(egui::Rect::from_min_size(
								egui::Pos2::ZERO,
								egui::vec2(width, 760.0),
							)),
							..Default::default()
						},
						|ui| {
							view.show(ui, &mut state);
						},
					);
					fn check(shape: &egui::Shape) {
						match shape {
							egui::Shape::Text(text) => assert!(
								![
									"Registered Games",
									"Current Game",
									"Added Games",
									"Add it!",
									"Add Game",
									"Share game activity",
									"Synthetic game"
								]
								.contains(&text.galley.job.text.as_str())
							),
							egui::Shape::Vec(shapes) => {
								for shape in shapes {
									check(shape);
								}
							}
							_ => {}
						}
					}
					for shape in &output.shapes {
						check(&shape.shape);
					}
					assert!(output.platform_output.commands.is_empty());
					output.drop_without_applying_deltas();
					assert!(!view.running_processes_request);
					assert_eq!(view.discord_activity_sharing_request, None);
					assert!(
						view.registered_games == original,
						"Hidden game preferences must not be rewritten"
					);
				}
			}
		}
	}
}
