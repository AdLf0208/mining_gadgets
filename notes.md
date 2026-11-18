# Mining Gadgets — port notes (slice 1: the gadget's laser and its FE)

Ported from **Mining Gadgets 1.18.8** (`mininggadgets-1.18.8.jar`, SHA-1
`2f97c2d52b384a59d54ec68bd6271608a775ad30` = the ATM10 record) for Minecraft
Java 1.21.1 / NeoForge. **MIT → licence rank 0**; local-only until publishing is
approved (ARCHITECTURE.md §8). Pin **`1d63e0a5`**
(`game.rs` `64813cf4…`), mixins **`5a64af93…`** (schema 6).

Status: **ported-partial** — slice 1 (the gadget item, its right-click laser,
the aimed-face plane, the per-block FE price and the upgrade data model) is
implemented and verified in-game. The held-use trigger, the renderblock ghost,
the client beam/particles/sounds, the settings and filter screens, the
Modification Table and the remaining upgrade effects are the gap list below.

**2026-11-18 — republished as an SDK package (`api = "0.2"`, version 0.2.0).**
Horizon's mod-API merge retired `.hnmod`; this port is now one flat
`mininggadgets.hmod` built against SDK nightly `0b14f8a` / mixins `e59da90`.
`src/runtime.rs` is byte-identical to the retired bundle's and its 24 tests are
unchanged; only the glue, the manifest and the packaging were rewritten. The
sections below are the 0.1.0 record (kept as the behaviour's evidence);
[`## Migration to the SDK format`](#migration-to-the-sdk-format-020-2026-11-18)
is the current layout, the wire table, the traps and the gap list. The e2e logs
in `ports/_shared/evidence/mininggadgets/` are **historical**: no 0.2.0 in-game
run exists (no `horizon-launcher` on this machine).

**Why this mod now:** it is the rank-0 `content`/`deep` core mod the queue
offered next (`precheck-mod.py mininggadgets` → `DISPATCHABLE`, jar licence MIT,
rank 0, no unported dependency). `ports/_shared/queue/brief-mininggadgets.md`
is the brief.

## Class + tier and the approach

- **Class:** `content`; **tier:** `deep` (source-map row, ledger).
- **Approach:** per `horizon/docs/modpacks.md` a deep content mod is an **agent
  port** — carried content (items, models, textures, lang, recipes) plus native
  Rust. This slice builds an item and its world effect; it adds no entity,
  renderer or dimension.

## What the slice does

- **The item.** `mininggadgets:mininggadget` (MK3), `_fancy` (MK2) and `_simple`
  (MK1), all one item class upstream and all three declared from
  `minecraft:stick` with `max_stack_size: 1` and `minecraft:max_damage:
  1_000_000`. The damage is the **charge bar**: upstream's `getMaxDamage`
  returns `MAXPOWER` and its bar reads the FE buffer, so the native half mirrors
  the charge into `damage` (`MAX_ENERGY - energy`).
- **The trigger.** Right click. `use_item_on` fires at the aimed block's face;
  `use_item` ray-casts to the beam range when the click meets no block. A held
  gadget **claims the click**, as upstream cancels `RightClickBlock`, so it
  never places the off-hand torch.
- **The plane.** `MiningCollect.collect` is reproduced exactly: `range == 1` is
  the aimed block; wider ranges are the square on the hit face's plane, oriented
  by the face (the player's facing stands in for "up" on a top/bottom face), with
  `NORMAL` centred, `PATHWAY` digging `range-2` up and 1 down, and `AUTO` doing
  that within 2 blocks of the player's feet. Targets are filtered by upstream's
  `isValid`: no air, no fluid that is not waterlogged, no hardness below 0, no
  block entity, no door.
- **The price.** `getEnergyCost = 200` base plus every *active* upgrade's
  `costPerBlock` (silk 100, void 10, magnet 25, fortune 1/2/3 = 30/60/100,
  efficiency 1..5 = 10/20/30/40/50); a `battery_creative` makes it 0. A shot
  needs `cost * range * range` in the buffer before it starts (`canMine`), then
  pays `cost` **per block actually broken**.
- **The break.** `native_machine_destroy_block(pos, Some(&tool), false)` — the
  engine's `Level.destroyBlock(pos, true)` with the loot rolled for `tool`,
  which is a gadget stack carrying the upgrades' own enchantments
  (`minecraft:silk_touch`, `minecraft:fortune <tier>`), exactly what upstream
  builds in `RenderBlockTileEntity.removeBlock`. Drops pop as item entities (no
  Magnet yet).
- **The dig time.** Upstream's `getHardness` — the plane's average
  `hardness * 30 / toolSpeed` (8, plus `efficiency²+1`, times Haste, over 3 under
  Mining Fatigue) scaled by the range — is the block's countdown. Slice 1 has no
  ghost block to count down, so it becomes the **item cooldown** of the shot
  instead: a harder plane locks the gadget for longer. That is a recorded
  deviation, not a silent substitution.
- **The energy store.** The gadget declares `Capabilities.Energy.ITEM` through
  `item_energy_buffer` (API 307): 1,000,000 FE, unlimited insert/extract per
  call, exactly upstream's `EnergyStorageItemstack` capacities. The engine keeps
  the amount in the stack's `minecraft:custom_data` as `horizon_energy`, so a
  charger or a battery from another port can fill the gadget.
- **The upgrade data model.** `mininggadgets:upgrades`, a list of
  `{name, active}`, read and written by base name (`fortune_2` is a
  `fortune_1`, `battery_creative` a `battery` of tier 4) exactly as
  `UpgradeTools` matches. The 24 cards are carried as content items whose ids
  are the jar's (`upgrade_silk` → the `silk` upgrade); upstream's card carries
  no data either.
- **The tooltip** shows the charge (`mininggadgets.gadget.energy`), the
  per-block price and the range, from the jar's own lang keys.

## Hooks (all stamped; no pending hook is named)

| Hook (API) | Helper | What it does |
|---|---|---|
| `component_types` (65) | `component_types.rs` | registers `mininggadgets:upgrades` and `mininggadgets:range` before content loads |
| `item_energy_buffer` (307) | `item_energy_buffer.rs` | the gadget's 1,000,000 FE `Capabilities.Energy.ITEM` buffer |
| `use_item_on` (13) | `use_item_on.rs` | right click on a block: fire at that face, claim the click |
| `use_item` (13) | `use_item.rs` | right click in the air: ray-cast to the beam range, fire |
| `tooltip` (12) | `tooltip.rs` | charge, per-block price, range |
| `register_commands` (8) | `register.rs` | `/mininggadgets info\|energy\|range\|upgrade` |
| `perform_command` (8) | `perform.rs` | the command bodies, in the acting player's state |

7 injections, 2 modules, 0 unstamped.

## The pin facts that shaped this slice

1. **No item-energy helpers were needed to invent anything.** ARCHITECTURE §3's
   gap row ("no energy/FE helper surface") is about moving FE *between block
   entities*; at this pin `item_energy_buffer` (API 307) **is** stamped and the
   engine has `native_item_energy`, `native_item_energy_insert` and
   `native_item_energy_extract` (`game/native_item_energy.rs`), keeping the
   amount in the stack's `custom_data`. So the gadget carries a real FE buffer
   instead of a stand-in.
2. **There is no hook before a block's own use.** `native_use_item_on` "hears a
   right click with an item on a block **once the block let it through**", so a
   chest still opens its screen before the gadget is asked (upstream cancels
   `RightClickBlock` earlier). The gadget still refuses the block entity and
   spends nothing — the e2e's phase B records the screen opening and closes it.
   Gap: "a pre-block-use interaction hook a mod can claim".
3. **`break_speed` is pending**, so per-stack tool stats cannot be read — not
   needed here: the gadget breaks through `native_machine_destroy_block` and
   mines nothing by hand.
4. **A content item naming a native component type requires the native half.**
   The first build gave each upgrade card a `mininggadgets:upgrade` component;
   the **content-only** launch refused the whole mod (`mininggadgets:upgrade is
   not a known component type`) and the `.hmod` was unusable on its own. The
   cards dropped it (upstream has no such data) and the `.hmod` now loads on an
   unmodded game — see `hmod-load.log`.

## Content (`.hmod`)

`content/` is generated by [`tools/assemble_content.py`](tools/assemble_content.py)
from the jar; [`content/README.md`](content/README.md) lists every carried and
every written file. 155 entries: 27 item model definitions, the jar's item
models and textures (the three Blockbench gadget models included), `en_us.json`,
26 recipes upgraded to 26.3, 27 item declarations and the nine `c:`
convention tags the recipes use.

## Verification

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6); checkouts
`ports/_shared/pins/horizon-1d63e0a` and `.../horizon-mixins-5a64af9`. Every
build/test/e2e ran through `ports/tools/build-slot.sh` with
`CARGO_INCREMENTAL=0`; no bare cargo, no `--target-dir`. Evidence:
`ports/_shared/evidence/mininggadgets/`.

| Step | Evidence | Result |
|---|---|---|
| runtime TDD (red) | `test-red.log` | **0 passed, 22 failed** — every body `todo!()` |
| runtime TDD (red, second cycle) | transcript | `face_of`/`known_upgrade` watched failing: **22 passed, 2 failed**, then green |
| runtime tests (green) | `test-green.log` | **24 passed**, 0 failed |
| fmt | `fmt.log` | `cargo fmt --all -- --check` exit 0 |
| clippy | `clippy.log` | `--all-targets -- -D warnings`, exit 0 |
| stage + check | `horizon-check.log` | `cargo_exit: 0`, **0 warnings** |
| embedded test | `horizon-test.log` | `test result: ok. **24 passed**; 0 failed; 1240 filtered out` |
| fast build | `horizon-build.log` | `cargo_exit: 0`, 0 warnings |
| e2e | `e2e.log` | **PASS**, `EXIT:0` |
| fail-capability | `e2e-selftest.log` | `SELFTEST=1` → **EXIT:1** (`the buffer is 9800 FE, expected 9700`) |
| log checker | `check-e2e-logs.py mininggadgets --strict` | `logs=2 ok=2 missing-exit=0 bad-exit=0` |
| `.hmod` build/check | `package-hmod.log` | 100,098 B, `62fbb4a6…2e9a`, 155 entries, “no code: content only” |
| `.hnmod` | `package-hnmod.log` | 16,372 B, `a4d503d4…38ea`, 11 entries |
| load-check | `hmod-load.log` | **PASS** with `--dev-mod <the .hmod>` on the shared unmodded game; the same ids are unknown without it |

### In-game transcript (quoted from `e2e.log`)

```
  Mining Gadget: 10000/1000000 FE, per block 200, range 1 of 1, upgrades empty, t=168
== A: one stone block, aimed west face -> air, one drop, 200 FE ==
  block 202,101,0 -> minecraft:air; cobblestone 1
  buffer: 9800 FE
== B: a chest (block entity) is not a target ==
  use-item: {"block":null,...,"ok":true,"screen":"container"}
  the chest's own use ran first: screen=container
  buffer: 9800 FE
  block 202,101,2 still minecraft:chest, buffer untouched
== C: bedrock (hardness -1) is not a target ==
  buffer: 9800 FE
  block 202,101,-2 still minecraft:bedrock, buffer untouched
== D: size_1 + range 3 mines the whole aimed face ==
  buffer: 8200 FE
== E: 300 FE is below the plane's 1,800 FE reserve ==
  buffer: 300 FE
  block 202,101,0 still minecraft:stone, buffer untouched
== F: a flat buffer does nothing ==
  buffer: 0 FE
  block 202,101,0 still minecraft:dirt
PASS: the laser broke the aimed stone block and dropped its cobblestone for 200 FE, left a chest and bedrock alone, mined a 3 x 3 face for 1,800 FE, refused an underfunded plane and an empty buffer, and every control left the buffer untouched
EXIT:0
```

The run is fail-capable three ways: `SELFTEST=1` flips A's energy assertion and
the run exits 1; phases B, C, E and F are controls that must *not* change the
world (a chest, bedrock, an underfunded plane, an empty buffer); and the
content-only leg (`hmod-load.sh`) shows the same ids unknown without the package.

### Packaged artifacts

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `content/mininggadgets.hmod` | 100,098 B | `62fbb4a64dc47f62f70ab56c0c08dbd7506959fe36af9fb727bd152cd7a02e9a` | 155 |
| `mininggadgets-0.1.0.hnmod` | 91,376 B | `b49bb4b25b2268acfc504e9db42aaf8b3134ea8c304968fef3349c74095cac0c` | 13 |

### Campaign gates at this touch

- `python3 ports/tools/precheck-mod.py mininggadgets` → exit 0, `DISPATCHABLE`,
  jar licence MIT (rank 0).
- `python3 ports/tools/check-skills.py --notes ports/mininggadgets` → the
  `Skills:` line below.
- `tools/test_template.py` / `tools/test_stage.py` are the inherited scaffold
  fixtures, written for a one-injection, one-module example bundle and the old
  `ports/horizon` layout; they do not validate this seven-injection bundle (the
  same limitation `quarryplus` and `powah` recorded). They are kept unmodified
  so the scaffold stays comparable with `ports/_template`; the real ladder above
  is the evidence.

## Review repairs (PR #569, 2026-11-11)

Review: `ports/_shared/horizon-mods/reviews/pr-569-mininggadgets.md` (CHANGES_REQUESTED, 3 items + findings).

| Reviewer's item | This job |
|---|---|
| artifact not staged; PR/release prose names the older hash | **orchestrator** (no write access; the entry and file are the new digest) |
| **P1: `fire` uses the playerless `native_machine_destroy_block`, which ignores the acting player's break rules** | **fixed**: every target is checked with `Game::block_action_restricted` (spectator breaks nothing; an adventure player only what `minecraft:can_break` allows) before it is charged or broken |
| **P2: `use_item_on` runs after the block interaction, so clicking a lever toggles it** | **fixed**: the block-click half now uses the `right_click_block` hook (API 21), which runs before `ServerPlayerGameMode.useItemOn` and cancels the click; `use_item_on.rs` is gone, `right_click_block.rs` replaces it at the same place in the plan |
| **P2: `amplifier("haste").unwrap_or(0)` conflates no Haste with Haste I** | **fixed**: the level is `amplifier + 1` when present and 0 only when absent, and `break_ticks` now treats `haste` as the level rather than the amplifier |

Tests: `efficiency_and_haste_shorten_the_break_and_fatigue_lengthens_it` now pins Haste I
(tool speed × 1.2) and Haste II (× 1.4) separately — the old code gave Haste I no effect at
all.

Verified: `python3 tools/horizon.py check` in a build slot — the six-injection plan with the
new `right_click_block` hook compiles against the pin, `cargo_exit: 0`
(`ports/_shared/evidence/mininggadgets/review-repairs-check.log`). The embedded test step
was still queued when the report was written.

Artifact: `mininggadgets-0.1.0.hnmod` **93,646 B** `a68c96252a0bf9cf7b81af633652141fed97faea158d5f5a8956ed484cd53d43`.

## Skills

## Embedded content — one `.hnmod` deliverable (2026-11-10)

The content half now travels **inside** the native bundle (the AE2 pattern, proved
on `charginggadgets`): the built `content/mininggadgets.hmod` is carried at
`mixins/mininggadgets/data/mininggadgets-content.hmod`; `embedded_content.rs` writes it to
`.horizon-cache/` at start and appends it to the dev list from one `content_root`
HEAD injection (`cancellable = true`; bindings `root`, `enabled`, `dev`). One
installed `.hnmod` installs both halves, so the split content+code blocker is
gone. `tools/package.py` packs `data/` and refuses a copy that differs from
`content/mininggadgets.hmod`.

Verification (2026-11-10, pin `1d63e0a`, Minecraft 26.3):
- **ladder**: `cargo fmt --check`, `clippy --all-targets -- -D warnings` (0
  warnings), `cargo test` **24 passed / 0 failed**, `horizon.py stage / check / test /
  build` all `cargo_exit: 0` (embedded tests included).
- **artifacts**: `content/mininggadgets.hmod` **100,098 B**, SHA-256 `62fbb4a64dc47f62f70ab56c0c08dbd7506959fe36af9fb727bd152cd7a02e9a`,
  **155** entries; `mininggadgets-0.1.0.hnmod` **91,376 B**, SHA-256 `b49bb4b25b2268acfc504e9db42aaf8b3134ea8c304968fef3349c74095cac0c`, **13**
  entries (rebuilt locally from these sources with `tools/package.py`; a second
  run is byte-identical and `horizon-mod check --kind native-mod` passes);
  `mixins/mininggadgets/data/mininggadgets-content.hmod` is byte-identical to the built `.hmod`.
- **e2e with no `--dev-mod`** (`EMBEDDED=1`): `PASS` + `EXIT:0`; the run printed
  `embedded content: 100098 B, identical to content/mininggadgets.hmod`.
- **control** (`e2e-control.log`, `.horizon-cache` made unusable): the sample id
  is unknown, `PASS` + `EXIT:0` - the content really comes from the bundle.
- **fail leg** (`SELFTEST=1`): `FAIL` + `EXIT:1`.
- `check-e2e-logs.py mininggadgets --strict` green (all logs end `EXIT:<n>`);
  `upstream_ready.py mininggadgets` -> `READY`, exit 0.

The compile ladder ran on the second build machine; the in-game legs ran locally.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices, rust-port

`cargo fmt`, `cargo clippy --all-targets -- -D warnings` and `cargo test` were run
before hand-back; `#[allow(..., reason = "…")]` is used only where the injected
module would otherwise report `private_interfaces` (the bundle's own `Game`
parameters) and the `SizeMode` variants slice 1 never constructs.

## Deviations and gaps (slice 1)

1. **One pulse per right click, not a held use.** Upstream starts a 72,000-tick
   use and mines a block every tick while the button is held. Slice 1 fires once
   per click; a repeat is a second click. The dig time becomes the item cooldown.
2. **No renderblock ghost.** Upstream replaces each target with a ghost block
   that draws the original, cracks down over `hardness*30/toolSpeed` ticks and
   regrows if you look away. That needs a block entity with a client renderer
   (`render_shape` + API 19/20) and a per-block countdown; slice 1 breaks the
   block when the shot lands.
3. **No client half at all**: no beam, no laser sound loop, no particles, no
   first-person model animation, no settings screen (`MiningSettingScreen`) and
   no filter screen.
4. **No Modification Table** (`modificationtable` block, its menu, the ghost
   filter inventory), so upgrades cannot be installed by gameplay; the
   `/mininggadgets upgrade <name>` command is a **harness control** that writes
   the same `mininggadgets:upgrades` list the table would (precedent:
   `/powah set`, `/quarryplus area`). `/mininggadgets energy` and `range` are the
   same kind of control.
5. **Upgrade effects not implemented**: Magnet (drops go to the inventory),
   Freezing, Light Placer, Void Junk. Silk Touch and Fortune **are** honoured
   (the drops are rolled with an enchanted gadget stack). Efficiency only
   shortens the cooldown, as upstream's does via the dig time.
6. **No energy interop beyond the buffer**: the gadget answers
   `Capabilities.Energy.ITEM` and keeps its charge in `custom_data`, so a
   charging block from another port can fill it, but this slice ships no charger
   and does not pull FE at all.
7. **The battery upgrades do not enlarge the buffer.** Upstream stores capacity
   in a second component (`forge_energy_max_energy`) and the bar follows it;
   slice 1 fixes the buffer at 1,000,000 (the base value) and records battery
   tiers in the upgrade list without effect.
8. **A container target still opens its screen** (pin gap 2 above); the gadget
   refuses to break block entities, so nothing is lost but the click.
9. **The blocks and the advancement/loot data** (`modificationtable`,
   `minerslight`, `renderblock`, recipe advancements) are not carried.

## Provenance (upstream analysis)

Repository `https://github.com/Direwolf20-MC/MiningGadgets`, tag **`v1.18.8`**
(commit `37b5fcc8f8615e216944f6ebdd6ba3ff4bfe4481`, `minecraft_version=1.21.1`);
clone deleted after the findings were written. Files read and behaviours
extracted: [`FINDINGS.md`](FINDINGS.md). The jar stays as licence evidence in
`ports/_shared/licence-cache/mininggadgets-1.18.8.jar` (provenance:
`ports/_shared/carry-work/mininggadgets/SOURCE.md`). Clean-room rewrite: no Java
was copied or translated and no jar was decompiled.

### Mode B + AE2 embedding (2026-11-10) — one listable artifact, re-verified

Pin unchanged (`1d63e0a5…` / `5a64af93…`, schema 6, `game.rs`
`64813cf4…f80f`). The session rebuilt the port from its current sources and
applied the **AE2 embedding pattern** so one `.hnmod` installs the content and
the code (the registry keeps one file per project): `mixins/mininggadgets/data/mininggadgets-content.hmod`
is the built `content/mininggadgets.hmod` byte for byte, `mixins/mininggadgets/embedded_content.rs`
compiles it in with `native_data!`, and the `embedded-content` `[[injection]]`
(`target = "horizon_client::game::mods::content_root"`, `kind = "HEAD"`,
`cancellable = true`, bindings `root`/`enabled`/`dev`) writes it to the game's
`.horizon-cache` and appends it to the dev-mod list. `tools/package.py` refuses
an embedded copy that differs from the built hmod.

All compile/e2e steps ran through `ports/tools/build-slot.sh` with
`CARGO_INCREMENTAL=0`; the ladder is the shared `tools/ladder-embed.sh`
(hmod build → embed → hnmod → fmt → clippy → test → stage/check/test/build),
evidence `ports/_shared/evidence/mininggadgets/ladder-*`.

| Step | Result |
|---|---|
| `cargo fmt --all --check` / clippy `-D warnings` | exit 0, **0 warnings** |
| `cargo test --locked --offline` | **24 passed**, 0 failed |
| `horizon.py stage/check/test/build` | `cargo_exit: 0` in every step |
| `content/mininggadgets.hmod` | **100,098 B**, `62fbb4a6…2e9a`, **155 entries**, "no code: content only" |
| `mininggadgets.hnmod` (single artifact, carries the content) | **91,376 B**, `b49bb4b2…cac0`, **13 entries** |
| e2e (with `EMBEDDED=1`, the shipped shape) | default **PASS + `EXIT:0`** / selftest **FAIL + `EXIT:1`** |
| `check-e2e-logs.py mininggadgets --strict` | green (`logs=… ok=…`, exit 0) |
| `upstream_ready.py --copyleft mininggadgets` | **READY**, exit 0 |

Fail-capability: the selftest/negative legs flip a real assertion and exit
non-zero; the embedded shape is the one the artifact ships (no `--dev-mod`), and
the e2e checks the bundle wrote its content out (or the loader reports
`content from 1 mod(s)` when the game consumes the file).

## Migration to the SDK format (0.2.0, 2026-11-18)

Horizon's mod-API merge (#16941, launcher ≥ 0.2.70; PR #17992 cleans up the old files)
retired `.hnmod`, and the registry closed this port's former submission **#833**
(`mininggadgets` 0.1.0) with that purge. **0.2.0** is the same behaviour republished as one
flat SDK `.hmod`: `horizon-mod.toml` with `api = "0.2"`, one `impl mod_api::Hooks`, the
content at the package root. `src/runtime.rs` and its tests carry over **byte-identical**;
only glue, manifest and packaging were rewritten.

Pins: SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (2026-11-18,
`package_format` 6, `content_api` "0.2"), mixins
**`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`**, game 26.3. The SDK is cached in-tree at
`ports/_shared/sdk/0b14f8a/` and lives in the port at `.horizon/sdk/` (gitignored) for
offline builds. `horizon-launcher` is **not installed** here, so there is no 0.2.0 in-game
run (see the gaps).

### The retired tree, and where it is

Everything the brief listed was **moved** (never copied) to
`ports/_shared/evidence/mininggadgets/retired-hnmod-0.1.0/` **before the first build**:
`mixins/` (the plan, `runtime.rs`, `support.rs`, the 7 hook helpers, the embedded content
and its helper), `compatibility.json`, `rust-toolchain.toml`, `content/` (after the carry
below), `tools/{audit,horizon.py,package.py,package.sh,test_stage.py,test_template.py}`,
`mininggadgets.hnmod`, `mininggadgets-0.1.0.hnmod`, and the port's own `AGENTS.md` (it told
the reader to open `compatibility.json` and run `tools/horizon.py`). `Cargo.lock` was
deleted, not kept — it is packed and changes the artifact.

Two more tools went the same way once their job was gone: `tools/hmod-load.sh` (loaded the
*content* `.hmod` on the shared unmodded game with `--dev-mod` — a native `.hmod` cannot
load on an unmodded game now, and `tools/e2e.sh`'s `CONTROL_GAME=` leg carries what was
left of it), `tools/ladder-embed.sh` (the old `stage → check → test → build` ladder) and
`tools/notice.py` (injected the notices into a content archive; the SDK build packs the root
`LICENSE`/`LICENSE-NOTICE.md` itself).

### File map (0.1.0 → 0.2.0)

| Old `.hnmod` layout | New SDK package |
|---|---|
| `horizon-mixin.toml` (`[mod]`, `[compatibility]`, 1 `[[module]]` pair, 8 `[[injection]]`) | `horizon-mod.toml` (id/name/version/api/side/capabilities + `[native]` game/horizon/mixins) |
| `Cargo.toml` (`[lib] path = "mixins/mininggadgets/runtime.rs"`, `[workspace] members = ["tools/audit"]`) | `Cargo.toml` (`[lib] path` default `src/lib.rs`, one dependency: `horizon-mod-api` from `.horizon/sdk/`) |
| `mixins/mininggadgets/runtime.rs` | `src/runtime.rs` — **byte-identical** (`ffdd0f07…726f`), `pub mod runtime;` at the crate root, 24 tests |
| `mixins/mininggadgets/support.rs` (`mininggadgets_state`, 503 lines) | `src/gadget.rs` (the same glue, rewritten against hosts/events) + `src/lib.rs` (the one `impl Hooks`) |
| 7 per-hook helper `.rs` files (`component_types`, `item_energy_buffer`, `right_click_block`, `use_item`, `tooltip`, `register`, `perform`) + `embedded_content.rs` | methods of `impl Hooks`, of the item Part, a `Registrar` declaration, or nothing (see the wire table) |
| `content/` (`assets/`, `data/`, its own `horizon-mod.toml`, the built `.hmod`, `README.md`) | package root: `assets/`, `data/`; `content/README.md` → `CONTENT.md` |
| `mixins/mininggadgets/data/mininggadgets-content.hmod` + `embedded_content.rs` (the AE2 write-out) | **gone**: code and content are one package, so there is nothing to embed |
| `compatibility.json`, `rust-toolchain.toml` | **gone**: `[native]` is the pin, and the launcher's toolchain builds |
| `tools/horizon.py`, `package.py`, `package.sh`, `audit/`, `test_stage.py`, `test_template.py`, `hmod-load.sh`, `ladder-embed.sh`, `notice.py` | **gone** (moved to evidence); the SDK ladder is `cargo test` + `cargo horizon-mod build/check/fits` |
| `mininggadgets-0.1.0.hnmod` (93,646 B, 13 entries) | `mininggadgets.hmod` (134,354 B, 163 entries) |

**Kept and updated:** `tools/assemble_content.py` (now writes `assets/` + `data/` at the
package root; re-run, it reproduced all 154 carried files byte for byte), `tools/e2e.sh`
(launcher-built game, `CONTROL_GAME=`), `FINDINGS.md`, `CONTENT.md`, `README.md`
(rewritten for the flat package), `LICENSE` (unchanged) and a new `LICENSE-NOTICE.md`
naming where the licence and the carried art were read from.

### Manifest

- `id = "mininggadgets"` (also the resource namespace), `name = "Mining Gadgets"`,
  `version = "0.2.0"` (the **port's** version, not upstream's 1.18.8),
  `authors = ["Direwolf20", "ErrorMikey", "AdLf0208"]`.
- `api = "0.2"` (exact), `side = "both"` (an integrated server belongs to the client
  installation, so a LAN host and single player need it), `capabilities = ["registries"]`
  (the schema's one value; this package really declares three items).
- `[native] game = "26.3"`, `horizon = "0b14f8a…"`, `mixins = "e59da90…"` replace
  `[compatibility] horizon_revision` and the bundle's mixins revision.
- `[mod] priority = 100` and per-injection kinds are gone; nothing in this slice needed
  ordering (`#[listener(priority = …)]` is the new lever).

### The wire: every 0.1.0 injection, one home each

| 0.1.0 `[[injection]]` / `[[module]]` (hook, API) | 0.2.0 | Mapping notes |
|---|---|---|
| `embedded-content` (`content_root` HEAD, cancellable, the AE2 pattern) | **none** | one flat `.hmod` carries `assets/` + `data/`; the helper existed only to work around the split |
| `component-types` (`component_types` 65) | `Hooks::component_types` | same `Vec<String>`; the two ids are kept for fidelity, though since 2026-11-10 the content no longer names them (the cards carry no component) |
| `item-energy-buffer` (`item_energy_buffer` 307) | `Registrar::item_capability(&ITEM_ENERGY, id, …)` for the three gadget ids, answered by `GadgetEnergy: EnergyHandler` | the retired hook answered a shared output for every item asked; a declaration is keyed by item. The charge stays the **same key** 0.1.0 wrote (`minecraft:custom_data.horizon_energy`), so a gadget charged before the migration keeps its FE — and it is a real `Capabilities.Energy.ITEM` buffer, so another port's charger fills it |
| `right-click-block` (`right_click_block` 21) | `Hooks::right_click_block` → `Flow::Cancel` + `cancellation_result = Success` | `Flow::Cancel` with `Pass` would let the item's own use run next (a double shot); `Success` ends the click as used, which is what upstream's cancel means |
| `use-item` (`use_item` 13) | `ItemPart::use_item` on `Gadget` (attached to the three gadget keys) | the Part is asked only for these items, so the id check the helper needed is gone. `Some(Success)` is the old `claimed = true` |
| `gadget-tooltip` (`tooltip` 12) | `ItemPart::tooltip` on `Gadget` | colours are `0xAARRGGBB` (as `rgba` packs them): `0xFF55FF55` / `0xFFFF5555` / `0xFFAAAAAA` |
| `register-commands` (8) | `Hooks::register_commands` (`RegisterCommandsEvent.commands`) | the same four usages, same permission levels |
| `perform-command` (8, split `&str` words) | `Hooks::run_mod_command` (`CommandInvocation`) | typed: `<amount:integer>` → `CommandArgumentValue::Integer`, `<name:word>` → `String`. The **readout text is byte-identical** to 0.1.0 (`Sight::line` is pinned by a test) |
| `[[module]] runtime` (`crate::game::mininggadgets`) | `pub mod runtime;` | ordinary Rust module, byte-identical |
| `[[module]] state` (`crate::game::mininggadgets_state`) | `src/gadget.rs` | no module plan section any more |

Host API substitutions (every one measured against the SDK source, not guessed):

| 0.1.0 | 0.2.0 |
|---|---|
| `game.held()`, `held_slot()`, `stack_in_mut()`, `hand_stack_replaced()` | `ItemPlace::PlayerHand { player, hand }`: `stack(host)` to read, `edit(host, tx, …)` inside `host.transaction(…)` to write (the place is written and journalled; the inventory sync is the game's) |
| `game.native_item_energy` / `…_insert` / `…_extract` | the declared `ITEM_ENERGY` capability on that place (`amount`/`insert`/`extract`), one implementation for both the trigger and `/mininggadgets energy` |
| `game.block_action_restricted(pos)` | `player.game_mode(host)`: a **spectator** and an **adventure** player break nothing (see the gaps: the `can_break` predicate is unreadable) |
| `game.native_machine_destroy_block(pos, Some(&tool), false)` | `level.destroy_block(host, pos, true, Some(&tool), false)` — same loot path with the enchanted tool, same item-entity drops |
| `game.native_view_hit()` | `player.entity().pick(host, MIN_BEAM_RANGE)` → `ClipHit { block, face }`; `miss` is `false` for a hit, and `BlockFace` is turned into the `[i32; 3]` the plane rules take |
| `game.world.block(pos)` + `game.res.facts()` | `level.block_state(host, pos)`, `level.block(host, pos).id(host)`, `level.fluid_state(host, pos).is_empty()`, `level.block_property(host, pos, "waterlogged")`, `host.block_states().destroy_speed(state)` (the -1 "unbreakable" the rule tests), `host.block_states().has_block_entity(state)`, and the id suffix `_door` for upstream's `DoorBlock` class test |
| `game.yaw`, `game.player.position.y` | `entity.head_yaw(host)` (`getYRot` for a player; `body_yaw` is None for one) and `entity.position(host)[1]` |
| `game.potions.effects.amplifier(…)` | `host.living(entity)` → `living.effects(host)` matched by `EffectKey::id`; the level is `amplifier + 1`, as the 2026-11-11 repair fixed it |
| `game.add_cooldown(item, ticks)` | `player.set_item_cooldown(host, &host.item_cooldown_group(&stack), ticks)` — the group the game files the cooldown under, not a hand-made id |
| `game.native_feedback(Text)` | `host.commands().feedback(Text)` |
| `game.world.game_time` | `level.game_time(host)` |
| `stack.data()` / `data_mut().set_other(k, v)` | `minecraft:custom_data` read whole from `stack.components().as_nbt()` and written back with `apply_components` (so the three keys in one compound survive each other, and the syntax `!minecraft:custom_data` removes it) |

### Build and verify (measured 2026-11-18)

Every compile ran in a `ports/tools/build-slot.sh` lease with `BUILD_LOCAL=1` (the job id
was `mininggadgets-sdk`, steps `test`/`fmt`/`build`/`check`), `CARGO_INCREMENTAL=0`, no
`--target-dir`. `BUILD_LOCAL=1` because the forwarding runner syncs the *port* and then
cannot see this command's relative `cd ports/mininggadgets` (the first attempt failed with
`cd: ports/mininggadgets: No such file or directory` on the far box).

| Step | Result | Evidence |
|---|---|---|
| runtime carried, glue absent | `cargo test` — **24 passed**, on the new crate before any glue existed | `sdk-test-runtime.log` |
| glue, watched red | **24 passed, 10 failed**, every body `not yet implemented` | `sdk-test-red.log` |
| tests | **35 passed**, 0 failed (24 `runtime::`, 11 `gadget::`) | `sdk-test.log` |
| lints | `cargo clippy --all-targets -- -D warnings` exit 0, **0 warnings** | `sdk-clippy.log` |
| format | `cargo fmt --check` exit 0, empty diff | `sdk-fmt.log` |
| package | `rm -f Cargo.lock && cargo horizon-mod build` → `Built ./target/horizon-mod/mininggadgets.hmod` | `sdk-build.log` |
| package, second run | **134,354 B**, sha256 `a1ce0b81…984d` both times (reproducible) | `sdk-build-second.log` |
| registry check | `mininggadgets 0.2.0 (native-mod, format native-1/0b14f8a…/e59da90…, for 26.3)` / `134354 bytes, SHA-256 25e81177…08ea, unsigned`, **EXIT 0** | `sdk-check.log`, `sdk-check-json.log` |
| fits the pinned build | `Mining Gadgets:` / `0b14f8a  made for`, **EXIT 0** | `sdk-fits.log` |
| what it uses | 341 surface entries: 23 hook entries over 8 hook names, 28 methods, 3 capabilities (`registries`, `horizon:energy`, `horizon:item_energy`), 5 part kinds, 2 macros, 97 handles, 20 traits, 163 types | `sdk-uses-from-package.json` |
| `runtime.rs` untouched | `unzip -p mininggadgets.hmod src/runtime.rs \| sha256sum` = `ffdd0f07…726f` = the retired file and `src/runtime.rs` | `sdk-runtime-sha256.txt` |
| content carry | re-ran `assemble_content.py` after the move: all **154** files byte-identical, nothing added | `sdk-content-carry.log` |
| skills line | `python3 ports/tools/check-skills.py --notes ports/mininggadgets --strict` | green (see below) |

The archive is **163 entries**: `horizon-mod.toml`, `Cargo.toml`, `LICENSE`,
`LICENSE-NOTICE.md`, `README.md`, `uses.json`, `src/{lib,runtime,gadget}.rs`, **91** under
`assets/` and **63** under `data/`. **No `Cargo.lock`** (trap 5), and no retired path.

### Traps this migration hit, or found waiting (new to this port)

1. **The relative `cd` in the brief's command is local-only.** `build-slot.sh run --step
   test` forwards a compile step to the second machine, which syncs the port and runs
   *there*: `bash: line 1: cd: ports/mininggadgets: No such file or directory`. Use
   `BUILD_LOCAL=1` for the SDK ladder (as `naturesaura` did), or drop the `cd` and pass an
   absolute path — not a port defect either way.
2. **`Compound` is `BTreeMap<JavaString, Tag>`, and `map.get(&str)` does not compile.**
   `Tag::get(&str)` exists, so a read of a compound's own key must go through
   `data.get(&KEY.into())`. This hit the *tests* first (`expected &JavaString, found
   &str`), which is the cheap place for it to hit.
3. **`Flow::Cancel` does not by itself stop the item's use.** `right_click_block`'s
   `cancellation_result` defaults to `Pass`, which "goes on to the item's own use" — with
   the gadget's `ItemPart::use_item` in place that is a **second shot per click**. Set
   `Success` (or `Fail`) before returning `Cancel`.
4. **A player's `body_yaw` is `None`.** `entity_body_yaw` reads `yBodyRot`, which Horizon
   does not keep for a player ("a player's body trails its head, so its facing yaw would be
   wrong"); `head_yaw` is the player's `getYRot`, which is what `Direction.fromYRot` wants.
5. **`ClipHit.face` is a `BlockFace` enum, not a vector.** The pure plane rules take
   `[i32; 3]`; one match turns `Up/Down/North/…` into `Direction`'s normal. `face_of` in
   `runtime.rs` is no longer needed by the glue and stays as tested public API.
6. **`HostBlockStates::block_states()` is an extension trait, not a host area.** `Host`'s
   supertraits do not bring it into scope, so `use mod_api::HostBlockStates;` is required
   (`no method named block_states ... items from traits can only be used if the trait is
   in scope`). `PlayerRef::game_mode`/`is_spectator` and `LivingRef::effects` are handle
   methods, and `player_game_mode` is **not** on a host trait a mod can name (it is a
   backend method).
7. **`host.players()` is the services list, which has no `player(uuid)`.** The handle-to-
   ref lookup is `host.player(uuid)` (`PlayersHost`); `players()` gives `acting_player`,
   `player_by_name` and `server_players` only.
8. **`ItemPlace::edit` is the right write for a stack with sibling keys.** `exchange`
   replaces a resource; `edit` mutates the stack in place (item and count kept) and
   journals in the caller's transaction — which is what lets the charge and the upgrade
   list live in the same `minecraft:custom_data` compound and survive each other.
9. **`Compound::remove` and `Tag::compound` need the `Into` form.** `data.remove(&KEY.into())`
   drops a key (the `!minecraft:custom_data` patch removes the whole component), and
   `Tag::compound([(KEY, tag)])` builds a nested one; `format!` must be turned into `&str`
   first (`removal.as_str().into()`), since `From<String>` is not implemented.
10. **The root is packed, the rest is not.** The build packs `horizon-mod.toml`,
    `Cargo.toml`, `README.md`, `LICENSE*` and `uses.json` beside `src/`, `assets/` and
    `data/`; `CONTENT.md`, `FINDINGS.md`, `notes.md` and `tools/` stay in the port (they
    are not rejected, just not packed). `LICENSE-NOTICE.md` had to be written: the port
    only had `LICENSE`, and the package must carry both.
11. **A stale `README.md` describes the retired layout.** It is *inside* the package, so it
    was rewritten before the first packaging build rather than after (it named `mixins/`,
    `compatibility.json` and `content/`).
12. **`assemble_content.py` wrote a `content/` folder.** Its `CONTENT` is the package root
    now; re-running it is the cheapest proof that the carry did not drift (154/154 files
    byte-identical). Its old file count printed every file under the port (2,539) because
    it globbed the root — it counts `assets/`+`data/` only now.
13. **`cargo fmt` must not touch `runtime.rs`.** `cargo fmt` (never `--all`: that formats
    the fetched SDK) rewrote only `src/lib.rs`'s signature wrapping; `cmp` against the
    retired file was re-checked after it, and again through `unzip -p`.
14. **The e2e has no default game any more.** `tools/horizon.py build` produced a game with
    the bundle compiled in, a shape that no longer loads; `GAME=` is now required and comes
    from `horizon-launcher native-build … --trust-native-code`, which is not installed
    here (trap 15 of `imfast`, again).

### Gaps after the migration

Everything in the 0.1.0 slice boundary still stands (the held-use trigger, the renderblock
ghost, the client beam/particles/sounds, the settings and filter screens, the Modification
Table, magnet/freezing/light placer/void junk, battery capacity, and the blocks and
advancements not carried). Added or sharpened by this format:

1. **No 0.2.0 in-game run.** `horizon-launcher` is not installed on this machine, so no
   game carries this package and `tools/e2e.sh` was not run. The 0.1.0 logs
   (`ports/_shared/evidence/mininggadgets/e2e*.log`, `hmod-load.log`) are **historical
   evidence for a `runtime.rs` that did not change**, not evidence for this package. What
   changed is the *transport* of the same calls: hosts and events instead of a `Game`
   handle. The next step is `horizon-launcher native-build ports/mininggadgets --instance
   <i> --trust-native-code`, then `GAME=<that client> tools/e2e.sh` (and `SELFTEST=1` for
   the fail leg); the assertions were kept byte-compatible on purpose.
2. **An adventure player's shot is refused.** 0.1.0 checked `game.block_action_restricted`,
   which honoured the held stack's `minecraft:can_break` predicate. The pin exposes no read
   of that predicate (`may_use_item_at` is the *placing* check), so the port refuses
   adventure players outright. Stricter than upstream, never looser; it becomes faithfully
   permissive the day a `can_break` read exists.
3. **The door filter is an id suffix.** Upstream's `DoorBlock` class test is now
   `id.ends_with("_door")`: every vanilla door matches, a modded door whose id does not
   ends with `_door` would be mined. `Candidate.door` is still the field the pure rules test.
4. **`/mininggadgets` needs a player.** `run_mod_command` answers "Hold a Mining Gadget
   first." when the source is not a player (a console or command block has no held gadget);
   0.1.0 ran every command in the acting player's state and had no such case to answer.
5. **The client half is still absent.** The air use and the block click are handled on both
   sides (the client cancels with `Success`, the server does the world work), but no beam,
   particles or sound is drawn, as before.
6. **The inventory write's client sync is unverified.** `ItemPlace::edit` on a player's hand
   is the documented path (`enderstorage` writes slots the same way), but whether the held
   stack's new charge reaches the client on the same tick is exactly what the in-game run
   would show. 0.1.0 needed a `hand_stack_replaced` call for it.

`## Upstream defects fixed`: none found in this migration. The 2026-11-11 review repairs
(spectator/adventure break rules, the pre-block click route, Haste's amplifier) are carried
forward in the new glue and are the reason traps 3 and 6 exist here.

### Changelog for the resubmission (supersedes #833)

> Supersedes **#833**, closed on 2026-11-18 because Horizon retired the `.hnmod` container
> (#16941; PR #17992 cleans up the old files). **0.2.0** is the flat SDK package:
> `api = "0.2"`, `[native]` pinned to nightly `0b14f8acfd97fa2c750e42795249dcc981178ca4`
> and mixins `e59da9094f0c2fdda6c627fe9eff22dec6a906f0`, for 26.3. The seven old
> injections became three typed hooks (`right_click_block`, `component_types`,
> `register_commands`) plus `run_mod_command`, an item Part (the air use and the tooltip)
> and a declared `Capabilities.Energy.ITEM` buffer; the content half now travels in the
> same package instead of an embedded `.hmod`; `src/runtime.rs` and its 24 tests are
> unchanged, byte for byte. Package: 163 entries, **134,354 B**, sha256
> `a1ce0b813ed603ea508d69fbd46f140fc54574aef19608171e2c13f0d72b984d`; `check` and `fits`
> exit 0; two builds hash identically. No 0.2.0 in-game run: the launcher is not installed
> here (gap 1).

### Publication route (campaign ruling 2026-11-18)

The submission form is gone; publishing is `horizon-rust/horizon-mods#1147` — the artifact
is attached to a release of the author's own repository under the entry's exact `file`
name, a PR adds the `[[versions]]` record and links the file, and a **maintainer** runs
`scripts/stage.sh` (a red `check` before that is the expected `fetch.sh` failure). The
orchestrator's `python3 ports/tools/publish-ready.py mininggadgets` does the bookkeeping;
this job owns no submission. The port stays **local-only until the maintainer approves**
(rank 0 MIT; approval not yet given for 0.2.0).

Skills: superpowers/using-superpowers,brainstorming,writing-plans,test-driven-development,systematic-debugging,verification-before-completion; BeautifulRust/rust-best-practices,rust-port
