// native_hooks::Game::native_mods (a hook point): Sodium's row in Options > Mods.
fn sodium_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::Game::sodium_mod_entry());
}
