# Timber Fall: Tree Falling Mod Concept

Nov 26, 2026 · @Ganesh

## Overview

Timber Fall makes a whole tree tip over like a real one when you chop its base, then turns it into item drops where it lands. This concept targets [Horizon](https://playhorizon.dev), a Rust rewrite of Minecraft Java Edition 26.3.

1. **Chop:** hit a bottom log with an axe. Bigger trees take longer to cut.
2. **Creak:** the trunk cracks and leans for about half a second, as a warning.
3. **Fall:** the whole tree rotates away from you, slowly at first, then fast.
4. **Crash:** leaves burst, dust rises, the screen shakes slightly and the trunk bounces once.
5. **Collect:** logs, saplings, sticks and apples drop along the fallen trunk.

Sneak while chopping to break only one block, as in vanilla.

## Tree detection

The mod fells a structure only when it passes every check, so wooden houses are not destroyed.

&#91;embedded content: chop to drops flow · 5 steps, 1 check\]

| Check | Default rule |
| --- | --- |
| Connection | Same log type, including diagonal neighbours, from the cut upward |
| Leaves | At least 5 naturally grown leaves touching the logs; player-placed leaves do not count |
| Size | Up to 256 logs; larger structures break normally |
| 2×2 trunks | All 4 logs at cutting height must be cut |
| Special trees | Nether fungi count wart blocks as leaves; mangrove and cherry need their own branch rules |
| Cut height | Logs below the cut stay as a stump |

## Falling animation

The trunk and leaves rotate as one rigid body around a hinge at the stump, so taller trees fall visibly slower.

```latex
\alpha = \frac{3g}{2L}\sin\theta
```

Here α is angular acceleration, g is gravity, L is trunk height and θ is the lean from vertical. This is the motion of a uniform pole pivoting at its base.

| Trunk height (blocks) | Realistic fall (s) | Game default, 1.5× speed (s) |
| --- | --- | --- |
| 5 | 2.4 | 1.6 |
| 10 | 3.4 | 2.3 |
| 20 | 4.8 | 3.2 |
| 30 | 5.9 | 4.0 |

Times assume 1 block = 1 m, g = 9.8 m/s² and a 3° starting lean.

| Phase | What the player sees and hears |
| --- | --- |
| Warning (0.5 s) | Wood cracking, 1–3° lean, a few leaves drop |
| Fall | Accelerating rotation, leaf particles trailing, rising whoosh |
| Impact | Stops on first solid ground, one 3° bounce, dust, thud, light camera shake |
| Break-up (1 s later) | Logs and leaves burst into items from base to tip |

- **Direction:** away from the player who made the cut.
- **Slopes:** the tree keeps rotating downhill until it touches ground, and stops earlier uphill.
- **Obstacles:** the falling tree passes through grass and flowers; solid blocks stop the trunk and are not damaged.
- **Danger:** players and mobs under the trunk take damage that grows with impact speed. Servers can turn this off.

## Drops, tools and config

The player gets the same loot as breaking every block by hand, and the axe pays the same wear.

- **Logs:** one item per log, merged into stacks near where each section lands to reduce lag.
- **Leaves:** saplings, sticks and apples use the normal leaf loot tables.
- **Durability:** 1 point per log, with Unbreaking applied. If the axe cannot cover the whole tree, only one log breaks.
- **Chop time:** normal time × (1 + logs ÷ 10), capped at 5×.
- **Hunger:** normal block-breaking exhaustion for every log.

| Config option | Default |
| --- | --- |
| Sneak disables felling | On |
| Require axe | On |
| Maximum logs | 256 |
| Fall speed | 1.5× |
| Falling damage | On |
| Camera shake | On |
| Auto-replant sapling | Off |

## Build plan

Build it as a native Horizon mod in Rust; Java mod jars do not run on Horizon ([Make a mod](https://playhorizon.dev/docs/make-a-mod)). Code is one crate that implements `mod_api::Hooks`, with separate server and client sides ([Mod code](https://playhorizon.dev/docs/mod-code)).

| Module | Job |
| --- | --- |
| `scanner` | Breadth-first search for logs and leaves, then validity checks |
| `chop` | Handles block breaking; checks sneak, axe, durability and permission |
| `falling` | Stores block states and offsets, fall direction, angle and speed |
| `render` | Client-side drawing of all blocks rotated around the hinge |
| `impact` | Collision, entity damage, particles, sound and drops |
| `config` | Loads the options listed above |

Build in this order:

1. Prove that a mod can draw a group of blocks rotating; this is the biggest unconfirmed point.
2. Detect a tree and break it with correct drops.
3. Join the two: straight 90° fall, then physics, collision, bounce, particles and sound.
4. Handle 2×2 trunks and special trees, then add config.
5. Test LAN play, large forests and performance.

Existing work to study:

- [TreeFall](https://github.com/d3grando/treefall) (MIT) already fells natural trees with normal drops and wear. Its page describes no falling animation. It hooks four game functions directly because hook points were missing.
- A [fallingtrees port](https://github.com/horizon-rust/horizon-mods/pull/1406) has been submitted to the mod registry. Its features are not confirmed here.

Risks to handle early:

- **Mod type:** a [pull request](https://github.com/horizon-rust/horizon/pull/3371) says WebAssembly code mods were removed, although the guide still lists them.
- **Trust:** native mods run with the user's full rights.
- **Platform:** Horizon lists Windows, macOS and Linux, not Android.
- **Item duplication:** remove the world blocks and start the fall in the same tick.
- **Quit mid-fall:** save the falling blocks, or drop the items immediately.
