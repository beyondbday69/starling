// A click on Item Physics's settings page: change the setting and keep it.
fn item_physics_click(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    button: usize,
) {
    use crate::game::item_physics as physics;
    if mod_id != physics::ID {
        return;
    }
    let mut settings = physics::settings(|key| game.native_setting(physics::ID, key));
    settings.click(button);
    physics::set_settings(settings);
    for (key, value) in settings.stored() {
        game.native_set_setting(physics::ID, key, &value);
    }
}
