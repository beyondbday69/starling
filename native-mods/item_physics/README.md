# Item Physics (Horizon native mod)

Horizon's official Item Physics: dropped items tumble while they fly, and once
they land a flat item lies down on the ground and a block sits up, both turned
to a yaw seeded by the item and resting flush on the surface — no floating, no
rigid spin. Its settings page (Options > Mods > Item Physics > Settings) has an
`Enabled: ON / OFF` button, on by default, kept in
`horizon-mods/.horizon-mod-data/item_physics/enabled`.

It is a [horizon-mixins](https://github.com/horizon-rust/horizon-mixins) bundle
(`mixins/item_physics/`) that the Horizon launcher builds into the game.

## Install

In the Horizon launcher: Mods → Item Physics → Install, then **Build** on the
same page (once built, switch it on or off in the game under Options → Mods).

## Made on the hook points

The bundle hooks the native hook points (`compatibility.json` names `"hooks": 1`)
and one hookable game method, `item_draw::Game::dropped_item_geometry`, whose
head it takes when the mod is on (vanilla's bob and spin run when it is off).
The draw itself reuses the game's own item pipeline (resolver, clustering,
lighting, entity culling, GPU skinning, outlines); only the pose is physics.

| File | What it is |
|---|---|
| `mixins/item_physics/horizon-mixin.toml` | The bundle: one module and the HEAD hooks |
| `mixins/item_physics/runtime.rs` | The mod's logic and state, added to the game as `game::item_physics`; also this crate's library, for its tests |
| `mixins/item_physics/dropped.rs` | The dropped-items hook: the draw with physics poses |
| `mixins/item_physics/{list,settings,click}.rs` | The Options > Mods row and the settings page |

License: MIT.
