// Item Physics's settings page: its on/off button.
fn item_physics_page(
    game: &mut crate::game::Game<'_>,
    mod_id: &str,
    buttons: &mut Vec<String>,
) {
    use crate::game::item_physics as physics;
    if mod_id != physics::ID {
        return;
    }
    let game = &*game;
    let settings = physics::settings(|key| game.native_setting(physics::ID, key));
    buttons.extend(settings.labels());
}
