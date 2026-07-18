# Changelog

All notable changes to **Zombies: Escape the Horde (Bevy edition)** are recorded
here. Versions follow the `MAJOR.MINOR.PATCH` scheme and match the version in
`Cargo.toml` (shown on the loading screen, the menu, and the in-game corner tag).

## [0.6.0]

### Added
- **Scene-lighting muzzle flash** — firing throws a soft burst of warm light that
  briefly illuminates the surrounding area (bigger for explosives).
- **Damage vignette** — taking a hit flashes a red gradient around the screen
  edges, its intensity scaled by the damage dealt to your health.
- **On-screen mobile controls** — a circular movement joystick (bottom-left) and
  a FIRE button (bottom-right) appear once touch is used, with **auto-aim** at the
  nearest zombie so you just tap to shoot.

### Changed
- **Start in a t-shirt with no head covering** — bare head (hair), short-sleeve
  t-shirt with bare forearms, no backpack. Helmet/armour/backpack are gained from
  pickups.
- **Softer, gradient ground shadows** for characters and walls (smootherstep
  radial falloff), for a more realistic look.
- **Turn lag** — the player eases into a new facing instead of snapping.
- **Shell casings eject further** with a bit of tumble.

## [0.5.0]

### Added
- **Equipment system** — the player's head, body and back are gear slots that
  can change: soft **cap** ↔ hard **helmet** ↔ **bare head**, field **jacket** ↔
  **body armour**, backpack on/off. The visuals swap to match.
- **Gear pickups** — helmets, body armour and medkits are scattered on the map
  and trickle in during play; walk over one to equip it (medkits heal).
- **Damage reduction** — a helmet soaks part of each hit and body armour soaks
  the bulk; both take the damage instead of your health.
- **Wear-and-tear indicators** — HUD durability bars for the helmet and armour
  that deplete and shift green → amber → red; when a piece is used up it breaks
  and is removed (helmet gone → bare head).

### Changed
- **Punchier walk/run** — bigger leg throw, vertical bob and a side-to-side body
  rock, more pronounced at a sprint.
- **Idle breathing** — the player visibly breathes when standing still, and the
  breathing quickens and deepens as stamina drops (fastest when winded).
- **Stamina-limited sprint** — top running speed now fades as the stamina bar
  empties instead of being a flat boost.

## [0.4.0]

### Changed
- **Blocky player construction** — the torso and backpack are now built from
  squares and rectangles instead of ellipses (a main body block plus a chest
  plate, shoulder blocks, back block and collar).
- **Articulated arms** — each arm is now two rectangle segments (upper arm +
  forearm) hinged at a **circular elbow**, ending in a circular hand, with the
  forearm bent inward so both hands meet the pistol. Driven from the shoulder so
  the whole limb swings/recoils as one.

## [0.3.0]

### Added
- **More detailed player** at higher pixel density, matching the reference art:
  a bulky segmented **backpack** with lid, seams, buckles and shoulder straps;
  a bigger padded **hat** on the crown; fuller **arms** with sleeve cuffs,
  ladder stitching and dark gloved hands; and torso highlight + collar shading.

### Changed
- Player jacket shifted to a sage field-green; wider ground shadow for the pack.

## [0.2.0]

### Added
- **Versioning** — the version is sourced from `Cargo.toml` and shown on the
  HTML loading screen, the start menu, and a bottom-right in-game tag.
- **Auto-reload** — firearms automatically start reloading when the magazine
  runs dry and reserve ammo is available (manual `R` still works).
- **Reload cycle indicator** — a ring of ticks above the player fills as the
  reload progresses. The cycle length reflects each weapon's own reload time
  (pistol 1.1s, machine gun 1.6s, assault rifle 1.8s, shotgun 2.2s, bazooka 2.6s).
- **This changelog.**

## [0.1.0]

### Added
- Initial Bevy (Rust → WebAssembly) recreation of the top-down pixel-art zombie
  shooter, hosted on Vercel.
- Tilemap "streets" world with circle/tile collision and a follow camera with
  screen shake.
- Player movement, stamina/sprint, aiming and reload; keyboard+mouse and dual
  virtual-stick touch controls.
- Weapons: knife, bat, 9mm pistol, machine gun, pump shotgun, assault rifle,
  bazooka — with projectiles, explosions, particles and decals.
- Zombie types (walker, runner, crawler, brute, spitter) with shambling-gait AI
  and a wave director.
- High-density procedural sprite rigs animated per frame, soft dynamic shadows,
  muzzle flashes, blood/gore, y-sorted depth.
- HUD, start and game-over screens.

### Changed
- Player restyled to an olive-drab field jacket, dark skin and a padded olive
  combat helmet, holding the 9mm in a two-handed grip.
- Brighter, textured asphalt with a tile-seam grid and grounded wall shading.

### Fixed
- The 2D camera was parked at `z=999`, frustum-clipping the floor layer; pinned
  to `z=0` so all sprite layers render.
- Documented that `wasm-opt` needs a modern binaryen (v108 miscompiles
  wasm-bindgen's externref table); the shipped bundle is un-optimized but the
  host serves it compressed.
