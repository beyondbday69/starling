// A click on Punchy Moves' settings page: change the setting and keep it, or
// open the screen the button names.
fn punchy_moves_click(game: &mut crate::game::Game<'_>, mod_id: &str, button: usize) {
    use crate::mobs::punchy_moves::{ID, camera, faces, ui};
    if mod_id != ID {
        return;
    }
    // The camera's buttons come first, then the face's, then the screens'.
    if button < camera::BUTTONS {
        let mut settings = camera::settings(|key| game.native_setting(ID, key));
        settings.click(button);
        camera::set_settings(settings);
        for (key, value) in settings.stored() {
            game.native_set_setting(ID, key, &value);
        }
        return;
    }
    let face_at = button - camera::BUTTONS;
    if face_at < faces::BUTTONS {
        let mut settings = faces::settings(|key| game.native_setting(ID, key));
        settings.click(face_at);
        faces::set_settings(settings);
        for (key, value) in settings.stored() {
            game.native_set_setting(ID, key, &value);
        }
        return;
    }
    // A screen button: open it (the tutorial starts at its first step).
    let screen_at = face_at - faces::BUTTONS;
    match ui::ui(|screens| screens.click_settings(screen_at)) {
        ui::UiRequest::ReplayTutorial => ui::ui(|screens| screens.tutorial.start()),
        _ => {}
    }
}
