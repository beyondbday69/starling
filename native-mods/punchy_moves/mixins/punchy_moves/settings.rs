// Punchy Moves' settings page: the shoulder camera's buttons, the face's, and
// one button per screen the mod opens.
fn punchy_moves_page(game: &mut crate::game::Game<'_>, mod_id: &str, buttons: &mut Vec<String>) {
    use crate::mobs::punchy_moves::{ID, camera, faces, ui};
    if mod_id != ID {
        return;
    }
    let game = &*game;
    let camera = camera::settings(|key| game.native_setting(ID, key));
    buttons.extend(camera.labels());
    let face = faces::settings(|key| game.native_setting(ID, key));
    buttons.extend(face.labels());
    for (label, _) in ui::settings_buttons() {
        buttons.push(label.to_string());
    }
}
