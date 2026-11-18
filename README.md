# Mining Gadgets — Horizon port (slice 1: the laser and its FE)

A native Horizon port of **Mining Gadgets 1.18.8**
(`Direwolf20-MC/MiningGadgets`, **MIT**): the Mining Gadget, a laser that mines
the block you aim at — or, with a size upgrade, the plane of the aimed face —
and pays for each block out of its own FE buffer.

**LOCAL-ONLY** until the maintainer approves publishing (ARCHITECTURE.md §8 of
the porting campaign). MIT is licence rank 0 (permissive); approval is still
required.

This is the **SDK package** layout (mod API 0.2, one flat `.hmod`): the code and
the content travel in one file. Provenance, hashes and the upstream commit are in
`ports/_shared/carry-work/mininggadgets/SOURCE.md` of the porting tree; the
behaviour reading is in `FINDINGS.md`; the ladder, deviations, gaps and the
migration record are in `notes.md`; what is carried and what is written is in
`CONTENT.md`.

## Layout

```
horizon-mod.toml           the only manifest: id, version 0.2.0, api 0.2, [native] pins
Cargo.toml                 one crate; its only dependency is horizon-mod-api from .horizon/sdk/
src/lib.rs                 the one #[mod_api::horizon_mod] impl Hooks, and the gadget item Part
src/gadget.rs              the glue: component data, the FE capability, the laser, the commands
src/runtime.rs             pure logic (cost, reserve, plane, dig time) — 24 tests, byte-identical to 0.1.0
assets/mininggadgets/…     the jar's item models, textures, names and 26.3 model definitions
data/mininggadgets/…       the item declarations, recipes, creative tab and the c: convention tags
LICENSE, LICENSE-NOTICE.md the MIT text and where it was read from, inside the package
mininggadgets.hmod         the deliverable (built by `cargo horizon-mod build`)
tools/{assemble_content.py,e2e.sh}  the content assembler and the in-game scenario
```

`src/runtime.rs` is the campaign's rule: the pure decisions are a plain module
with no `mod_api` types, so they are tested by `cargo test` alone and did not
change when the container did.

## Building

The launcher fetches the SDK into `.horizon/sdk/` (gitignored) and compiles the
crate against the pinned game build:

```sh
ports/tools/build-slot.sh run --job mininggadgets-sdk --port ports/mininggadgets --step build -- \
    bash -c 'cd ports/mininggadgets && CARGO_INCREMENTAL=0 cargo test --offline'
rm -f Cargo.lock            # a packed lock changes the artifact
cargo horizon-mod build     # -> target/horizon-mod/mininggadgets.hmod
cargo horizon-mod check mininggadgets.hmod
cargo horizon-mod fits mininggadgets.hmod --build nightly
```

Every compile goes through the campaign's build-slot runner (never a bare
`cargo` outside a lease, never `--target-dir`). The in-game scenario
(`tools/e2e.sh`) needs a game built by `horizon-launcher native-build` for this
package; run it with `GAME=<that binary>` (and `SELFTEST=1` for the fail leg).
`horizon-launcher` is not installed on the machine this version was built on, so
no 0.2.0 in-game run is claimed — see `notes.md`.

## What the slice does

Right click a block with the gadget: the aimed face's plane (one block, or a
3×3/5×5/7×7 square with a size upgrade) is mined, each block costing 200 FE plus
every active upgrade's own price, rolled with silk touch and fortune when those
upgrades are active, and the gadget locks for the plane's dig time. Right click
in the air ray-casts five blocks first. `/mininggadgets info` reports the charge.

**Not ported yet:** the held-use trigger (one pulse per click instead), the
renderblock ghost, the client beam/particles/sounds, the settings and filter
screens, the Modification Table, and the magnet, freezing, light placer and void
junk effects. `notes.md` has the full gap list.
