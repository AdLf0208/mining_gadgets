# Mining Gadgets 1.18.8 — upstream behaviour (analysis record)

Read from **`Direwolf20-MC/MiningGadgets` tag `v1.18.8`** (commit
`37b5fcc8f8615e216944f6ebdd6ba3ff4bfe4481`, the pack's exact jar) plus the jar's
own data files. Analysis only: the port is a clean-room Rust rewrite and no Java
was copied or translated. The clone was deleted after this file was written.

## Files read

`common/items/MiningGadget.java` (the item: `use`, `onUseTick`,
`releaseUsing`, `getEnergyCost`, `getHardness`, `canMine`, `canMineBlock`,
`getMaxDamage`/`getBarWidth`, `appendHoverText`), `common/items/UpgradeCard.java`,
`common/items/upgrade/Upgrade.java`, `UpgradeTools.java`,
`UpgradeBatteryLevels.java`, `common/items/gadget/MiningProperties.java`,
`common/items/gadget/MiningCollect.java`, `common/tiles/RenderBlockTileEntity.java`,
`common/capabilities/EnergyStorageItemstack.java`, `setup/Config.java`,
`setup/Registration.java`, `setup/MGDataComponents.java`,
`common/collectors/ShapelessWalker.java` (unused upstream),
`common/events/ServerTickHandler.java`, `common/MiningGadgets.java` (the
`RightClickBlock` cancel). Jar data: `recipe/*.json`, `lang/en_us.json`,
`models/item/*.json`, `textures/item/**`.

## The behaviour

**The trigger.** `MiningGadget.use(world, player, hand)`: sneaking sets
`canMine` (and opens the settings screen on the client); otherwise, when
`canMine(stack)` fails the click fails, and when it succeeds the player starts a
72,000-tick use. `onUseTick` then runs once per tick of the held use. A held-use
model, not a per-click pulse.

**What it mines.** Each use tick: `VectorHelper.getLookingAt(player,
Fluid.NONE, beamRange)` ray-casts the beam (`beamRange`, default
`MIN_RANGE = 5`), and if the aimed block is air nothing happens.
`MiningCollect.collect(player, hit, world, range, sizeMode)` builds the target
set: `range == 1` is just the aimed block; any wider range is the square of
blocks on the hit face's plane (`midRange = (range-1)/2` up/down and
left/right), oriented by `hit.getDirection()`, with `up` taken from the player's
facing when the face is vertical. `SizeMode.NORMAL` keeps it centred;
`PATHWAY` digs `range-2` up and 1 down; `AUTO` does that only when the aimed
block is within 2 blocks of the player's feet. `isValid` refuses air, a fluid
that is not a waterlogged block, hardness < 0, any block entity, doors, and the
mod's own miner's light.

**How a block goes.** Not broken directly: each target is replaced by a
`renderblock` ghost that draws the original block and counts down. Its first
durability is `floor(avg(hardness * 30 / toolSpeed) * range)` (minimum 1), where
`toolSpeed = 8 + (efficiency² + 1)` with the Efficiency upgrade, times
`1 + haste*20%`, divided by 3 under Mining Fatigue. Each further use tick
decrements it; unmined ghosts regrow after 10 ticks and reset. At 0 the ghost's
block entity posts `BlockEvent.BreakEvent`, rolls `Block.getDrops` with a **fresh
gadget stack carrying Silk Touch and/or Fortune from the upgrades**, adds the
block's experience, and either pops the drops or, with the Magnet upgrade,
puts them in the player's inventory.

**The energy.** FE lives on the stack behind `Capabilities.Energy.ITEM`
(`EnergyStorageItemstack`): the component `mininggadgets:forge_energy`, capacity
`Config.MININGGADGET_MAXPOWER` (**1,000,000** by default, also what
`getMaxDamage` returns so the item bar *is* the charge bar). `canMine` asks for
`cost * range * range` when `range > 1`; the charge itself is taken **once per
block** when its ghost reaches 0: `getEnergyCost = baseCost (200) + the sum of
every active upgrade's costPerBlock`. A creative battery makes both 0.

**Upgrade costs** (`CostPerBlock`): silk 100, void junk 10, magnet 25, fortune
1/2/3 = 30/60/100, efficiency 1..5 = 10/20/30/40/50. Size, range and battery
upgrades cost nothing per block; freezing and light placer charge per operation
(100 each). Upgrades are stored as `mininggadgets:upgrade_data`, a list of
`{upgradeName, isActive}`, and are matched by **base name** (`fortune_2` is a
`fortune_1`, `battery_creative` a `battery` of tier 4). The cards are plain
items — the item *is* the upgrade — installed by the Modification Table block
(not ported here).

**Interaction.** `MiningGadgets.rightClickEvent` cancels
`PlayerInteractEvent.RightClickBlock` whenever a gadget is held, so a gadget
never places the off-hand torch or opens what it clicks.

## What this slice takes, and what it leaves

Taken: the per-block price and the upgrade cost table, the `canMine` reserve,
the aimed-plane target set with all three size modes, the validity rules, the
dig-time formula (as the item cooldown — see below), the FE buffer in the
stack, the bar mirror, the drops rolled with the upgrades' silk/fortune, and the
tooltip.

Left: the held-use trigger (slice 1 fires once per right click), the renderblock
ghost and its regrow/shrink animation, the client beam, particles, sounds and
settings/filter screens, the Modification Table and the filter inventory, the
Magnet's pull, freezing, light placing, void junk, and the
creative battery's independence from the cost. See `notes.md` for the pin gaps
that force the first two.
