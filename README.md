# Zombies: Escape the Horde — Bevy edition

A top-down pixel-art zombie shooter, rebuilt from the ground up in **[Bevy](https://bevyengine.org/)** (Rust) and compiled to **WebAssembly** for the browser. It is a reimagining of the original vanilla-JS/Canvas
[Zombies](https://github.com/Bobs-Dev-Attic/Zombies), with **higher pixel density**, **smoother
skeletal animation**, and richer scene graphics.

Wade through a shambling horde, gather weapons, and survive escalating waves.

## Play

**▶ Live:** https://zombies-bevy-bobs-dev-attics-projects.vercel.app

(The live page streams the ~14 MB WebAssembly build from the public GitHub repo via
CDN, so first load takes a few seconds.)

Locally:

```bash
# one-time toolchain
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.126

# build the wasm bundle into web/pkg
./build.sh

# serve the static site
cd web && python3 -m http.server 8080
# open http://localhost:8080
```

## Controls

| Action        | Desktop                   | Mobile                          |
| ------------- | ------------------------- | ------------------------------- |
| Move          | `WASD` / arrow keys       | Left half — drag joystick       |
| Sprint        | Hold `Shift` / full stick | Push left stick fully           |
| Aim & fire    | Mouse + left click        | Right half — touch to aim/fire  |
| Reload        | `R` (auto when empty)     | Auto when empty                 |
| Swap weapon   | `1`–`7`, or `E`           | —                               |

Firearms **auto-reload** when the magazine runs dry; a **reload cycle indicator**
(a ring of ticks above the player) fills over the weapon's own reload time. The
current **version** is shown on the loading screen, the menu, and in-game — see
[`CHANGELOG.md`](CHANGELOG.md) for what changed in each release.

## What's improved over the original

- **Higher pixel density** — characters are drawn as smooth, sub-pixel-positioned
  sprite rigs rendered at full display resolution instead of a small upscaled buffer.
- **Skeletal animation** — every character is a hierarchy of body parts (torso,
  head, arms, legs, weapon) animated per-frame: scissoring legs, body bob,
  shambling sway, weapon recoil, and melee swing arcs.
- **Scene depth** — soft dynamic shadows, y-sorted rendering, wall top-lighting,
  ground decals (blood pools, scorch marks), and layered particles.
- **Juice** — muzzle flashes, blood bursts, gore, casing ejection, explosions and
  trauma-based screen shake.

## Architecture

Pure-ECS Bevy. Each module owns one slice of the game:

| Module        | Responsibility |
| ------------- | -------------- |
| `world.rs`    | Tilemap generation, circle-vs-tile collision, floor/wall sprites |
| `player.rs`   | Player state, movement, stamina, aiming, reload |
| `input.rs`    | Keyboard/mouse + dual virtual-stick touch input |
| `weapons.rs`  | Weapon catalogue (melee, pistol, SMG, shotgun, rifle, launcher) |
| `combat.rs`   | Firing, projectiles, explosions, particles, decals, death/gore |
| `enemy.rs`    | Zombie types, shambling AI, wave director |
| `art.rs`      | Procedural sprite-rig building + per-frame animation |
| `camera.rs`   | Follow camera with screen shake |
| `hud.rs`      | HUD bars, menus, game-over, run lifecycle |

## Deploy (Vercel)

The repo is configured for a **zero-build static deploy**: `vercel.json` serves the
`web/` directory (which contains `index.html` and the committed `pkg/` wasm bundle).
Rebuild the bundle with `./build.sh` and commit `web/pkg` whenever the Rust changes.

## License

MIT — same spirit as the original.
