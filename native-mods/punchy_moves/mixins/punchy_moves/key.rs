// The camera's, the editor's, the free camera's, the emote menu's and the
// emote slots' keys, in the Java `handleKey` order; a changed setting is kept
// as a file line.
fn punchy_moves_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::mobs::punchy_moves::{ID, camera, ui};
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

    // The camera key flips the controller and the settings are kept as file
    // lines; the editor key opens the camera's screen; the free-camera key
    // toggles the orbit, which remembers the player's rotation to orbit
    // around. The controller's message and the free-camera restore pair go
    // back with the glue that owns the overlay and the player's rotation.
    let camera_key = game.native_mod_key(ID, camera::TOGGLE_CAMERA, camera::DEFAULT_TOGGLE_CAMERA);
    let editor_key = game.native_mod_key(ID, camera::TOGGLE_EDITOR, camera::DEFAULT_TOGGLE_EDITOR);
    let orbit_key = game.native_mod_key(ID, camera::TOGGLE_ORBIT, camera::DEFAULT_TOGGLE_ORBIT);
    if pressed && key == camera_key {
        let (settings, _message, _restore) =
            camera::toggle_camera(|name| game.native_setting(ID, name));
        for (name, value) in settings.stored() {
            game.native_set_setting(ID, name, &value);
        }
        return;
    }
    if pressed && key == editor_key {
        ui::ui(|screens| screens.camera.toggle());
        return;
    }
    if pressed && key == orbit_key {
        let (_message, _restore) = camera::toggle_orbit(
            |name| game.native_setting(ID, name),
            game.native_player().map_or(0.0, |player| player.yaw),
            game.native_player().map_or(0.0, |player| player.pitch),
        );
        return;
    }

    // The emote menu key: held down it is up, let go it plays what it points
    // at.
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
    if pressed && let Some(index) = ui::ui(|screens| screens.emote_keys.index_of_key(key)) {
        let emote = ui::ui(|screens| screens.emote_wheel.emotes().get(index).copied());
        if let Some(emote) = emote {
            ui::play_emote(game.native_player_entity.map(|id| id as u64), emote);
        }
    }
}
