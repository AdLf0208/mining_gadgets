# Mining Gadgets — a Horizon port

**Mining Gadgets** is a Minecraft mod. Because Mining with Lasers is cool. This repository is a **port of it to [Horizon](https://github.com/horizon-rust/horizon)**, the Rust engine for Minecraft Java 26.3. It carries the complete source of the port plus the content files it ships, and installs as a single `.hmod` package.

Upstream: [Direwolf20-MC/MiningGadgets](https://github.com/Direwolf20-MC/MiningGadgets) — licence **MIT**.

| | |
|---|---|
| Upstream mod | Mining Gadgets ([source](https://github.com/Direwolf20-MC/MiningGadgets)) |
| Ported to | Horizon 0b14f8a, Minecraft Java 26.3, mod API `0.2` |
| Package | `mininggadgets.hmod` (install it with the Horizon launcher) |
| Licence | **MIT** — the upstream mod's licence; see [Licence and credit](#licence-and-credit) |

## What this port implements

### What the slice does

Right click a block with the gadget: the aimed face's plane (one block, or a
3×3/5×5/7×7 square with a size upgrade) is mined, each block costing 200 FE plus
every active upgrade's own price, rolled with silk touch and fortune when those
upgrades are active, and the gadget locks for the plane's dig time. Right click
in the air ray-casts five blocks first. `/mininggadgets info` reports the charge.

**Not ported yet:** the held-use trigger (one pulse per click instead), the
renderblock ghost, the client beam/particles/sounds, the settings and filter
screens, the Modification Table, and the magnet, freezing, light placer and void
junk effects. `notes.md` has the full gap list.

## Not yet ported

- **One pulse per right click, not a held use.** Upstream starts a 72,000-tick
- **No renderblock ghost.** Upstream replaces each target with a ghost block
- **No client half at all**: no beam, no laser sound loop, no particles, no
- **No Modification Table** (`modificationtable` block, its menu, the ghost
- **Upgrade effects not implemented**: Magnet (drops go to the inventory),
- **No energy interop beyond the buffer**: the gadget answers
- **The battery upgrades do not enlarge the buffer.** Upstream stores capacity
- **A container target still opens its screen** (pin gap 2 above); the gadget

## Installing

1. Install the [Horizon launcher](https://github.com/horizon-rust/horizon/releases) (0.2.70 or newer).
2. Add `mininggadgets.hmod` to the instance's mods.
3. The package is pinned to a specific Horizon build; the launcher checks that pin.

## Licence and credit

Mining Gadgets was created by its upstream authors; the code, models and assets in the original mod
remain theirs and are used here under **MIT**. The Rust in this repository is a clean-room
rewrite for Horizon, not a translation of the original source.

- `LICENSE` — the upstream licence text.
- `LICENSE-NOTICE.md` — what this port carries and where it came from.
- The upstream project: https://github.com/Direwolf20-MC/MiningGadgets

## Technical details

```
horizon-mod.toml   package manifest (id, version, api, [native] pin)
Cargo.toml         the crate; depends on `horizon-mod-api` from the fetched SDK
src/               lib.rs (the one `impl Hooks`), runtime.rs (pure logic + tests), glue
assets/ data/      the content the package ships
tools/             content generators and the e2e scenario
```

- Build: `cargo horizon-mod build` (packages; it does not compile against the game).
- Test: `cargo test`, `cargo clippy --all-targets -- -D warnings`.
- In-game: `ports/tools/launcher-game.sh run mininggadgets` in the campaign tree, or install the
  `.hmod` and play; the harness scenario is `tools/e2e.sh`.

This repository is generated from the campaign's port at `ports/mininggadgets/`; the port's own
`notes.md` is the full record (decisions, traps, evidence).
