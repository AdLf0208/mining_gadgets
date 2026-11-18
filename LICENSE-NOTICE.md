# Licence notice — Mining Gadgets for Horizon

This package is a **clean-room Rust rewrite** of *Mining Gadgets* for the Horizon
engine. It is not a translation of the original's code: no Java was decompiled or
copied.

## Where the licence was read from

| What | Where |
|---|---|
| The mod's licence | `mininggadgets-1.18.8.jar` → `META-INF/neoforge.mods.toml`, `license="MIT (https://github.com/Direwolf20-MC/MiningGadgets/blob/master/LICENSE.txt)"` |
| The jar's identity | SHA-1 `2f97c2d52b384a59d54ec68bd6271608a775ad30` = the ATM10 record (`ports/_shared/licence-cache/mininggadgets-1.18.8.jar`) |
| Upstream source read | `https://github.com/Direwolf20-MC/MiningGadgets`, tag `v1.18.8` (commit `37b5fcc8f8615e216944f6ebdd6ba3ff4bfe4481`) |

The jar carries no `LICENSE` file of its own; the declaration above and the
repository's `LICENSE.txt` are the terms. Licence rank **0** (permissive, MIT).

## Copyright

- Mining Gadgets, and the models, textures, names and recipe data carried in
  `assets/` and `data/`: **Copyright (c) Direwolf20 and ErrorMikey**, under the
  MIT licence. Credit also to CPW and the NeoForge team, as the jar's
  `credits` line names.
- The Rust rewrite (everything in `src/`) and this package's build files:
  **Copyright (c) 2026 AdLf0208**, under the same MIT licence.

Both notices are reproduced in `LICENSE`, which travels inside this package.

## What is carried, and from where

`assets/` and `data/` are the jar's own item models, textures and names, its
recipes upgraded from the pack's 1.21.1 shape to 26.3, plus the item
declarations and the creative tab that name them. Nothing is invented;
`CONTENT.md` lists every carried and every written file.

## Publishing

This port stays **local-only until the maintainer approves publishing**
(`ARCHITECTURE.md` §8 of the porting campaign). MIT is a permissive licence, so
the approval is the only bar; it has not been given for this version.
