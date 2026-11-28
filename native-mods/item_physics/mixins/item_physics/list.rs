// Item Physics's row in Options > Mods.
fn item_physics_listed(mods: &mut Vec<crate::game::native_hooks::NativeMod>) {
    use crate::game::item_physics as physics;
    mods.push(crate::game::native_hooks::NativeMod {
        id: physics::ID.into(),
        title: format!("Item Physics {}", physics::VERSION),
        description: physics::DESCRIPTION.into(),
        keys: Vec::new(),
    });
}
