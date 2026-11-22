// native_hooks::Game::native_mods (a hook point): the Falling Trees row in Options > Mods.
fn fallingtrees_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    if !mods.iter().any(|m| m.id == "fallingtrees") {
        mods.push(crate::game::Game::fallingtrees_mod_entry());
    }
}
