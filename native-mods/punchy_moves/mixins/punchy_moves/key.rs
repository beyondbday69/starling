// The camera's, the orbit's, the emote wheel's and the emote slots' keys, and
// the keys an open screen answers to; a changed setting is kept as a file line.
fn punchy_moves_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::punchy_moves::{ID, camera, ui};
    if !game.native_on(ID) || !game.native_playing() {
        return;
    }
    // A screen that is up takes the key before the world does.
    if pressed && ui::ui(|screens| screens.any_open()) {
        match ui::ui(|screens| screens.key(key, pressed)) {
            Some(ui::UiRequest::PlayEmote(index)) => {
                let emote = ui::ui(|screens| screens.emote_wheel.emotes().get(index).copied());
                if let Some(emote) = emote {
                    ui::play_emote(game.native_player_entity.map(|id| id as u64), emote);
                }
            }
            Some(ui::UiRequest::ReplayTutorial) => ui::ui(|screens| screens.tutorial.start()),
            Some(ui::UiRequest::CloseAll | ui::UiRequest::None | ui::UiRequest::RebindKey(_)) => {}
            None => return,
        }
        return;
    }

    // The emote wheel: held down it is up, let go it plays what it points at.
    let wheel = game.native_mod_key(ID, ui::EMOTE_WHEEL_KEY, ui::DEFAULT_EMOTE_WHEEL_KEY);
    if key == wheel && wheel != "key.keyboard.unknown" {
        if pressed {
            ui::ui(|screens| screens.emote_wheel.open());
        } else if let Some(emote) = ui::ui(|screens| screens.emote_wheel.close()) {
            ui::play_emote(game.native_player_entity.map(|id| id as u64), emote);
        }
        return;
    }
    // An emote slot's own key plays that emote at once.
    if pressed {
        if let Some(index) = ui::ui(|screens| screens.emote_keys.index_of_key(key)) {
            let emote = ui::ui(|screens| screens.emote_wheel.emotes().get(index).copied());
            if let Some(emote) = emote {
                ui::play_emote(game.native_player_entity.map(|id| id as u64), emote);
            }
            return;
        }
    }

    // The camera and orbit keys flip their settings, kept as file lines.
    let mut settings = camera::settings(|name| game.native_setting(ID, name));
    let camera_key = game.native_mod_key(ID, camera::TOGGLE_CAMERA, camera::DEFAULT_TOGGLE_CAMERA);
    let orbit_key = game.native_mod_key(ID, camera::TOGGLE_ORBIT, camera::DEFAULT_TOGGLE_ORBIT);
    if key == camera_key {
        if pressed {
            settings.toggle();
        } else {
            return;
        }
    } else if key == orbit_key {
        if pressed {
            settings.free_camera = !settings.free_camera;
        } else {
            return;
        }
    } else {
        return;
    }
    camera::set_settings(settings);
    for (name, value) in settings.stored() {
        game.native_set_setting(ID, name, &value);
    }
}
