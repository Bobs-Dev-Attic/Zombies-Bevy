# Changelog

All notable changes to **Zombies: Escape the Horde (Bevy edition)** are recorded
here. Versions follow the `MAJOR.MINOR.PATCH` scheme and match the version in
`Cargo.toml` (shown on the loading screen, the menu, and the in-game corner tag).

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
