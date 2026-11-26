// native_hooks::Game::native_mods (a hook point): the Timber Fall row in Options > Mods.
fn timber_fall_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    if !mods.iter().any(|m| m.id == "timber_fall") {
        mods.push(crate::game::Game::timber_fall_mod_entry());
    }
}