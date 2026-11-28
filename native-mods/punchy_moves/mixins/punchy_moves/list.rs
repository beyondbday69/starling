// Punchy Moves' row in Options > Mods, with its keys: the shoulder camera, the
// orbit camera and the emote wheel.
fn punchy_moves_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::punchy_moves as punchy;
    mods.push(crate::game::native_hooks::NativeMod {
        id: punchy::ID.into(),
        title: format!("Punchy Moves {}", punchy::VERSION),
        description: punchy::DESCRIPTION.into(),
        keys: vec![
            [
                punchy::camera::TOGGLE_CAMERA.into(),
                "Shoulder camera".into(),
                punchy::camera::DEFAULT_TOGGLE_CAMERA.into(),
            ],
            [
                punchy::camera::TOGGLE_ORBIT.into(),
                "Orbit camera".into(),
                punchy::camera::DEFAULT_TOGGLE_ORBIT.into(),
            ],
            [
                punchy::ui::EMOTE_WHEEL_KEY.into(),
                "Emote wheel".into(),
                punchy::ui::DEFAULT_EMOTE_WHEEL_KEY.into(),
            ],
        ],
    });
}
