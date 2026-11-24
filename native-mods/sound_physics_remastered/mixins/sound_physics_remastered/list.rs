// native_hooks::Game::native_mods (a hook point): Sound Physics Remastered's row in Options > Mods.
fn sound_physics_remastered_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    mods.push(crate::game::Game::sound_physics_remastered_mod_entry());
}
