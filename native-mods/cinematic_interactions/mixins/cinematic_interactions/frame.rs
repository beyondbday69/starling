// Once a frame: the settings are read at the start, and the mod follows its switch in Options > Mods.
fn cinematic_frame(game: &mut crate::game::Game<'_>) {
    use horizon_client_lib::native_cinematic_interactions_scene as ci;
    if !ci::loaded() {
        ci::load(crate::game::Game::native_config_dir().ok());
    }
    ci::set_active(game.native_on(ci::ID));

    if ci::controls() {
        if let Some(pos) = ci::poll_pending_tap() {
            game.cursor = pos;
            game.inventory_press(false);
            game.inventory_mouse_release(false);
            game.audio.play_button_click();
            if let Some(key) = game.container {
                game.container_changed(key);
            }
        }
    }
}
