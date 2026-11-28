// Once a frame: Punchy Moves' animation clock and the face settings its rigs
// read, so a change on Options > Mods reaches the faces the same frame.
fn punchy_moves_frame(game: &mut crate::game::Game<'_>) {
    use crate::mobs::punchy_moves::{ID, engine, faces};
    engine::begin_frame();
    faces::set_settings(faces::FaceSettings::read(|key| {
        game.native_setting(ID, key)
    }));
}
