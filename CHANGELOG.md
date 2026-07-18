# Changelog

All notable changes to **Zombies: Escape the Horde (Bevy edition)** are recorded
here. Versions follow the `MAJOR.MINOR.PATCH` scheme and match the version in
`Cargo.toml` (shown on the loading screen, the menu, and the in-game corner tag).

## [0.17.0]

### Added
- **Minimap** — a corner map of the arena showing the walls, your position
  (green), the objective (yellow) and nearby zombies (red).
- **Objective + direction indicator** — you're now sent to an **extraction
  point**: a banner shows the distance, a floating **arrow** by the player points
  the way, and the minimap marks it. Reach it and the next one is set.

### Changed
- **Zombie arms** hang from **under the shoulders** and spread **wider** to the
  sides.
- **Crawler arms crawl** — a dragging zombie now hauls itself along, each arm
  reaching out ahead and pulling back in turn.
- **Wider player collider** — the body keeps more distance from walls so the
  arms and gun don't bury into them.

## [0.16.0]

### Added
- **Headshots** — a shot has a chance to blow a zombie's head apart for an instant
  kill, and the odds climb the **closer** the zombie is to the player. On a
  headshot the **brains burst out the far side of the head** and the body drops
  into a sprawled ragdoll corpse.
- **Getting shot is disfiguring** — a solid hit can **blow a limb clean off**
  (it tumbles away as a gib with a nub of exposed bone and a spray of blood, and
  stays gone).
- **Detailed kill corpses** — fallen zombies now sprawl into a jointed corpse
  with splayed arms and legs, a blood pool, spilled guts and exposed rib bone;
  headshot corpses have a burst skull with a brain trail.
- **Crows** — some corpses have crows feeding on them. They **hop and peck**, and
  **flap off fast when you get close** (leaving a puff of feathers). Crows can be
  **shot out of the air** — they burst into feathers and a dead-bird corpse.

### Changed
- **Flies fly like real flies** — some trace lazy loops, others zig-zag in sharp
  erratic darts, all buzzing around and congregating on the corpses.

### Removed
- **The confusing yellow glow pools** (the flickering street-lamp lights) are
  gone.

## [0.15.1]

### Changed
- **Flies fly more naturally** — they now hover and **congregate around corpses**,
  holding near a spot and darting to a new nearby one now and then (a jittery
  hover, not a frantic swarm), and only give the player a small berth instead of
  zooming away.
- **Street-lamp pools read as lights** — the vague yellow gradients are toned
  down into a soft warm pool with a **bright bulb core**, so they look like a
  failing overhead light rather than a floating glow.
- **Blast knockout is a cloudy blur** — the concussion overlay is now a soft
  gradient haze that swells in and clears smoothly, instead of a hard pixelated
  flicker.

### Fixed
- **Ricocheting bullets point the right way** — a bounced round's tracer is now
  re-oriented along its new heading instead of keeping its original angle.

## [0.15.0]

### Added
- **Bat gets its own model and two swings** — the Bat now shows a **wooden club**
  (instead of the knife blade) and is swung **two-handed**, alternating between a
  horizontal **baseball swing** (the club whips across the front) and an overhead
  **executioner chop** (raised behind the head, then driven down forward). Both
  hands are solved with **2-bone inverse kinematics** so they actually grip the
  handle throughout the swing.

## [0.14.2]

### Fixed
- **Knife no longer floats** — the blade is now pinned to the right hand (its
  position and angle are derived from the arm and elbow so it stays gripped),
  and the **left arm is hidden** while the knife is out, so it reads as clean
  one-handed knife work. The right-arm slash/stab motion is unchanged.

## [0.14.1]

### Changed
- **Knife stance & attacks reworked** — the knife hand is now cocked **out to the
  right**, and the left arm hangs **low with the hand tucked by the waist**.
  Attacks alternate between a wide **slash** (the blade sweeps across the chest)
  and a forward **stab** (the blade thrusts straight out along the aim); the stab
  lunges a little further and hits a narrow arc, the slash covers a wide arc.

## [0.14.0]

### Added
- **Scroll-wheel weapon switching** — the mouse wheel now cycles weapons (scroll
  down for the next, up for the previous).
- **Bazooka backblast** — firing the launcher now shoots a plume of exhaust
  smoke (and a flare) out of the rear vent.
- **Blast shockwave & concussion** — a rocket's explosion now knocks the
  **player** back too (harder the closer they are), and a close-enough blast
  **knocks them out**: the screen washes into a disorienting pixelated haze and
  shakes, and the player can't act for a spell that scales with how close they
  were to the centre. The blast also throws everything nearby outward relative
  to distance from the centre.

### Changed
- **Shoulder-mounted bazooka** — the launcher is now hoisted up onto the right
  shoulder and aimed down the sights, instead of held at the waist.
- **Fixed the knife grip** — the knife is now held in the **right hand** like a
  butcher's cleaver: the right arm is slightly bent with the blade cocked across
  the chest ready to sweep out and across, while the left arm is bent out with
  the hand resting near the waist.

## [0.13.0]

### Added
- **Side-by-side shotgun** — a new break-action double-barrel. It fires two fast
  shells, then **breaks open on reload**: the barrels hinge down toward the
  ground while the support hand feeds two fresh shells into the breech before it
  snaps shut.
- **Buckshot ricochets** — pump-shotgun and side-by-side pellets now **bounce off
  walls** (losing bite on each bounce), like the other small calibres.
- **Atmosphere / dressing** across the arena:
  - **Debris and garbage** — crushed cans, scrap paper, rubble, gravel and grime
    stains litter the streets.
  - **Blood pools and corpses** — bodies laid out in pools of blood; some
    **still twitch**, others have their **guts spread out** in a gory mess.
  - **Flies** buzz erratically over the corpses and **scatter when you approach**.
  - **Fog of war** — the streets fade into gloom beyond your immediate
    surroundings.
  - **Flickering lights** — failing street-lamp pools waver like dying bulbs.

### Changed
- **Pixelated gore trails** — crawlers' drag-smears and wounded walkers' bloody
  footprints are now chunky pixel-art clusters instead of smooth smears.
- **More varied zombies**:
  - Some drag **one limp, bloodied leg** behind them (leaving a blood streak).
  - Some trail **torn clothing** that flaps as they move.
  - **Arm movement varies per zombie and per arm** — each arm blends between a
    swing and a reaching grasp at its own rate and phase, so some swing one arm
    while clawing forward with the other and no two shamble alike.

## [0.12.0]

### Added
- **Anatomical zombie limbs** — zombie arms and legs are now built like the
  player's: two-segment limbs with a **shoulder, elbow, hip, knee** and a
  **foot**. Elbows fold and knees flex through the walk, drag and reach
  animations so the horde moves with real joints instead of stiff planks. (Each
  joint is now a separately-addressable part — the groundwork for the upcoming
  feature where limbs can be missing or get shot off.)
- **Game Settings (cheats)** in the Options menu — a list of toggles:
  - **All Weapons** — every gun kept loaded and ready to fire on switch.
  - **Unlimited Ammo** — reserves never run dry (and no reloads interrupt you).
  - **Super Stamina** — never tire; sprint forever.
  - **Super Armor** — 1000% better protection (you take a tenth of all damage).

## [0.11.0]

### Added
- **Caliber-based ballistics** — small calibres **ricochet off walls** (pistol and
  machine gun bounce, throwing sparks), while big powerful rounds **punch through
  multiple targets and even walls** (assault rifle), **losing killing power** with
  each penetration or bounce.

### Changed
- **Per-gun reload animations** — each firearm reloads its own way. The pistol
  and assault rifle now play a **magazine change**: the support hand drops to the
  well at the **bottom of the grip** and drives a fresh mag up (the rifle's mag is
  bigger) instead of loading from the side; the shotgun still racks its pump; the
  bazooka reloads its rocket.
- **Assault-rifle muzzle flash** is now **larger** and sits **further out at the
  barrel tip**, well clear of the body.

## [0.10.4]

### Fixed
- **Body no longer drifts off after firing** — the recoil kick used an
  accumulating offset on the torso (which was never reset), so the shirt slid
  away from the head/arms with every shot. It now resets each frame.

### Changed
- **Shotgun hold** — the butt tucks back into the right armpit again (angled
  stock) with the barrel still level so it fires straight; the left hand sits on
  the pump and the right on the trigger.
- **Zombies don't enter the player's space** — they're held at the edge of the
  player's body instead of overlapping it.
- **More player-like zombies** — thinner torsos, **feet** on the legs, and their
  **arms vary**: some shamble with swinging arms, others hold their arms out
  and grasp toward the player.

### Added
- **Gear swapping** — walking over a new helmet while already wearing one now
  **drops the worn helmet (keeping its damage) and equips the new one**; same for
  body armour. Re-picking a dropped piece restores its remaining durability.

## [0.10.3]

### Changed
- **Shotgun fires straight** — the barrel now runs level along the aim line (the
  stock drops to the shoulder instead of angling the whole gun), so shots leave
  the muzzle straight down the sights.
- **Pump-action animation** — firing racks the fore-end: the left hand pulls the
  pump back to chamber a fresh shell while the spent shell ejects.
- **Firing kickback** — the head and upper body rock back a little on each shot.
- **Slimmer player** — the body is thinner front-to-back, and the body-armour
  plate carrier was reshaped to match.

## [0.10.2]

### Changed
- **Shotgun grip refined** — the gun sits a little further forward with the butt
  just in front of the right armpit; the player's elbows now actually **fold** so
  the **left hand rests on the pump** and the **right hand is cocked back on the
  trigger** (the forearm joints are now posable, not fixed).

## [0.10.1]

### Fixed
- **Bazooka no longer crashes the game** — firing a zero-spread weapon sampled an
  empty random range (`-0.0..0.0`) and panicked, freezing everything. Zero-spread
  weapons now fire straight without sampling.

### Changed
- **Corrected shotgun hold** — the butt now tucks into the right armpit with the
  barrel angled up so its tip is centred; the left arm starts under the left
  shoulder on the fore-end and the right arm is bent inward to the grip.

## [0.10.0]

### Added
- **Pump-shotgun handling** — the shotgun is now fired from the hip: held low and
  angled across the waist, with a **pumping animation** (the fore-end racks back
  and forward) on every shot and through the reload.
- **Zombie overhaul**
  - Zombies are built with **player-like proportions** (rounded torso, longer
    bare arms, blocky legs, head) and vary in **size and colour**.
  - **Disfigurement** — some are missing an arm or a leg (hidden limb replaced by
    a bloody stump and a nub of exposed bone) and many have a **gash with exposed
    ribs** on the torso.
  - **Movement matches the body** — a missing leg makes them limp (uneven stride,
    head bob) and move slower; **crawlers drag themselves** along the ground,
    hauling with their arms while their legs trail, and move much slower.
  - **Blood on the ground** — crawlers leave a continuous **blood smear trail**;
    wounded walkers leave **alternating bloody footprints**.
  - **Varied gaits** — per-zombie arm-swing amplitude, stride and **turning
    radius**, so the horde no longer moves in lockstep.
- **Zombie separation** — zombies push apart instead of stacking on the same spot.

## [0.9.1]

### Added
- **Mobile weapon-swap button** — a SWAP button above FIRE cycles through the
  weapons with a tap (touch only).

### Changed
- **Pistol reload polish** — the other hand now does the magazine change, with a
  smaller, more contained motion.
- **Slimmer player** — the back, chest and shoulders are thinner so the body no
  longer looks so bulky.

### Fixed
- **Stray square on the pistol** — the magazine is tucked in the grip and only
  shows while a fresh mag is being seated during a reload, so there's no black
  block hanging off the side.

## [0.9.0]

### Added
- **Distinct weapon models** — each weapon kind now has its own top-down model
  (pistol, SMG, shotgun, assault rifle, bazooka, melee blade) instead of one
  shared bar, all drawn barrel-forward with the grip in the hands so nothing
  reads as being held sideways.
- **Per-gun reload animation** — reloading now plays out visibly and its length
  matches each weapon's own reload cycle time: the support hand dips to the mag
  well and back, the barrel dips, and the spent magazine is flung out and drops
  away. For the pistol the **slide racks back, the clip falls out, and the arm
  seats a fresh magazine** before the slide slams home.

### Changed
- **Shell casings eject from the breech** — spent casings now fly out of the
  side of the gun's slide instead of from near the body.

## [0.8.3]

### Changed
- **Pistol held further out** — the gun sits well clear of the body now (barrel,
  muzzle and flash pushed further forward).
- **Bigger upper arms** — the shirt-sleeve upper arm is longer and noticeably
  thicker than the forearm, and reaches out to the further-held gun.
- **Removed the neck** — dropped the skin-coloured collar piece so the head sits
  straight on the torso.

## [0.8.2]

### Changed
- **Player hold & fit tweaks** — the held gun sits further forward so it lines up
  with the hands (muzzle/flash moved to the new barrel tip); the arms mount
  directly under the shoulders and a touch wider; the upper arm is now a
  short **shirt-coloured sleeve** with the bare forearm below; the combat
  **helmet is smaller and re-centred** to fit the head; and the head diameter is
  a little larger.

## [0.8.1]

### Changed
- **Arms reworked** — longer upper-arm and forearm segments, and the shoulders
  are set wider so the arms spread out further from the head for a broader,
  more natural two-handed stance.
- **Lighter skin tone** — the player's skin (and bare arms/hands) is a lighter
  shade.

## [0.8.0]

### Changed
- **Player redesign** — smaller head that now sits at the player's exact centre;
  designed hair (crown + tufts + fringe) shown by default since you start
  bare-headed; bigger, longer, fully skin-toned arms; and softly **rounded back
  and shoulders** (new rounded-rectangle sprites).
- **Breathing is more rhythmic** — the idle breath now follows a shaped
  inhale / hold / exhale / rest cycle at a steady cadence (faster and deeper as
  stamina drops) instead of a plain sine wobble.

## [0.7.0]

### Added
- **Options menu** — a new PLAY / OPTIONS menu with an **Aim Assist accuracy
  slider** (0% = fully manual aiming, 100% = instant lock-on). Adjust it by
  dragging, tapping − / +, or with the ← / → keys; touch auto-aim snappiness
  now follows the slider.

### Changed
- **Pixelated muzzle flash** — replaced the soft gradient glow with a blocky
  star of squares that flashes from the barrel tip, plus forward sparks.

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
