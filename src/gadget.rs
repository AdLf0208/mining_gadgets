//! The Mining Gadget's glue: the component data it keeps, the FE capability, the
//! laser's target read, the tooltip and the `/mininggadgets` bodies.
//!
//! Every rule lives in [`crate::runtime`] (the retired bundle's `runtime.rs`,
//! byte-identical); this file reads the game, calls it and writes the result
//! back. Its tests pin what it *keeps*: the `minecraft:custom_data` codec 0.1.0
//! wrote (so a gadget made before the migration keeps its charge, its upgrades
//! and its range), the tool stack's 26.3 enchantment patch, and the exact line
//! `/mininggadgets info` prints.
//!
//! What the retired bundle did through the engine's own item-energy helpers
//! (API 307, the int `horizon_energy` in the stack's `minecraft:custom_data`) is
//! here a declared `Capabilities.Energy.ITEM` handler, [`GadgetEnergy`], over
//! that same key — so a charger from another mod still fills the gadget and the
//! charge stays where 0.1.0 wrote it.
use std::convert::Infallible;

use mod_api::{
    BlockFace, ClientHost, Compound, EnergyHandler, Flow, GameMode, Hand, Host, HostBlockStates,
    HostSide, HostTransaction, InteractionResult, ItemPlace, ItemStack, LevelRef, PlayerRef,
    RightClickBlockEvent, ServerHost, Tag, Text, TooltipEvent, Transaction, UseItemEvent,
};

use crate::runtime::{
    Candidate, MAX_ENERGY, MIN_BEAM_RANGE, Pos, SizeMode, UpgradeEntry, active, affordable,
    bar_damage, break_ticks, clamp_range, collect, cost_per_block, facing_from_yaw, is_valid,
    max_range, reserve, tier,
};

/// The three gadget tiers of the jar, all the same item class upstream.
pub(crate) const GADGETS: [&str; 3] = [
    "mininggadgets:mininggadget",
    "mininggadgets:mininggadget_fancy",
    "mininggadgets:mininggadget_simple",
];

/// The gadget's upgrade list, as `UpgradeTools` keeps `UPGRADE_DATA`.
pub(crate) const UPGRADES: &str = "mininggadgets:upgrades";
/// The gadget's chosen mining range (`MiningProperties.RANGE`).
pub(crate) const RANGE: &str = "mininggadgets:range";
/// The FE buffer's key inside `minecraft:custom_data`, the shape API 307 wrote.
pub(crate) const ENERGY: &str = "horizon_energy";
/// The component every one of those keys lives in.
pub(crate) const CUSTOM_DATA: &str = "minecraft:custom_data";
/// The item energy capability's component, as the retired `item_energy_buffer`
/// hook answered it.
const ENCHANTMENTS: &str = "minecraft:enchantments";
/// One upgrade entry's name.
const NAME: &str = "name";
/// Whether the player left that upgrade active.
const ACTIVE: &str = "active";

/// Whether `item` is one of the gadget tiers.
pub(crate) fn is_gadget(item: &str) -> bool {
    GADGETS.contains(&item)
}

/// The upgrades a gadget's `minecraft:custom_data` carries, in the order they
/// were installed (`UpgradeTools.getUpgradesNBT`).
pub(crate) fn read_upgrades(data: &Compound) -> Vec<UpgradeEntry> {
    let Some(Tag::List(entries)) = data.get(&UPGRADES.into()) else {
        return Vec::new();
    };
    entries
        .iter()
        .filter_map(|entry| {
            let Tag::Compound(fields) = entry else {
                return None;
            };
            let Tag::String(name) = fields.get(&NAME.into())? else {
                return None;
            };
            let active = !matches!(fields.get(&ACTIVE.into()), Some(Tag::Byte(0)));
            Some(UpgradeEntry {
                name: name.to_string_lossy(),
                active,
            })
        })
        .collect()
}

/// Write the upgrade list back, as `UpgradeTools.setUpgradesNBT` lays it out: a
/// list of `{name, active}`, and no key at all for an empty list.
pub(crate) fn write_upgrades(data: &mut Compound, upgrades: &[UpgradeEntry]) {
    if upgrades.is_empty() {
        data.remove(&UPGRADES.into());
        return;
    }
    let list: Vec<Tag> = upgrades
        .iter()
        .map(|entry| {
            Tag::compound([
                (NAME, Tag::String(entry.name.as_str().into())),
                (ACTIVE, Tag::Byte(i8::from(entry.active))),
            ])
        })
        .collect();
    data.insert(UPGRADES.into(), Tag::List(list));
}

/// The mining range the gadget carries (`MiningProperties.RANGE`), 1 by default.
pub(crate) fn read_range(data: &Compound) -> i32 {
    match data.get(&RANGE.into()) {
        Some(Tag::Int(range)) => *range,
        _ => 1,
    }
}

/// Write the range; 1 is the default and removes the key, as 0.1.0 wrote it.
pub(crate) fn write_range(data: &mut Compound, range: i32) {
    if range == 1 {
        data.remove(&RANGE.into());
    } else {
        data.insert(RANGE.into(), Tag::Int(range));
    }
}

/// The FE one gadget holds: the int `horizon_energy`, 0 when it holds none.
pub(crate) fn energy_of(data: &Compound) -> i32 {
    match data.get(&ENERGY.into()) {
        Some(Tag::Int(energy)) => (*energy).clamp(0, MAX_ENERGY),
        _ => 0,
    }
}

/// Keep `energy` as the gadget's charge, clamped to the buffer; 0 removes the
/// key, so a flat gadget is the data of a new one again.
pub(crate) fn apply_energy(data: &mut Compound, energy: i32) {
    match energy.clamp(0, MAX_ENERGY) {
        0 => {
            data.remove(&ENERGY.into());
        }
        energy => {
            data.insert(ENERGY.into(), Tag::Int(energy));
        }
    }
}

/// The `minecraft:enchantments` patch the drops are rolled with, as upstream
/// builds `tempTool` in `RenderBlockTileEntity.removeBlock`: silk touch when the
/// silk upgrade is active, fortune at its tier, nothing at all otherwise.
pub(crate) fn tool_enchantments(upgrades: &[UpgradeEntry]) -> Option<Compound> {
    let mut levels = Compound::new();
    if active(upgrades, "silk") {
        levels.insert("minecraft:silk_touch".into(), Tag::Int(1));
    }
    let fortune = tier(upgrades, "fortune_1");
    if fortune > 0 {
        levels.insert("minecraft:fortune".into(), Tag::Int(fortune));
    }
    if levels.is_empty() {
        return None;
    }
    let mut patch = Compound::new();
    patch.insert(
        ENCHANTMENTS.into(),
        Tag::compound([("levels", Tag::Compound(levels))]),
    );
    Some(patch)
}

/// The upgrades `/mininggadgets info` lists: `empty`, or the names in order with
/// an inactive one marked `(off)`.
pub(crate) fn upgrades_summary(upgrades: &[UpgradeEntry]) -> String {
    if upgrades.is_empty() {
        return "empty".to_owned();
    }
    upgrades
        .iter()
        .map(|entry| {
            if entry.active {
                entry.name.clone()
            } else {
                format!("{} (off)", entry.name)
            }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// What `/mininggadgets info` reports, as the in-game check reads it.
pub(crate) struct Sight {
    pub(crate) energy: i32,
    pub(crate) cost: i32,
    pub(crate) range: i32,
    pub(crate) max_range: i32,
    pub(crate) upgrades: Vec<UpgradeEntry>,
    pub(crate) time: i64,
}

impl Sight {
    /// The gadget's answers for a stack, read out of its own component data.
    fn of(stack: &ItemStack, time: i64) -> Self {
        Self::of_data(&custom_data_of(stack), time)
    }

    /// The same, from the component data alone (what the tests can build).
    fn of_data(data: &Compound, time: i64) -> Self {
        let upgrades = read_upgrades(data);
        Self {
            energy: energy_of(data),
            cost: cost_per_block(&upgrades),
            range: clamp_range(read_range(data).min(max_range(&upgrades))),
            max_range: max_range(&upgrades),
            upgrades,
            time,
        }
    }

    /// The report line. Its text is the retired bundle's, so the assertions the
    /// in-game check makes still match.
    pub(crate) fn line(&self) -> String {
        format!(
            "Mining Gadget: {}/{} FE, per block {}, range {} of {}, upgrades {}, t={}",
            self.energy,
            MAX_ENERGY,
            self.cost,
            self.range,
            self.max_range,
            upgrades_summary(&self.upgrades),
            self.time
        )
    }
}

// ---------------------------------------------------------------------------
// The stack's `minecraft:custom_data`, read and written whole
// ---------------------------------------------------------------------------

/// The `minecraft:custom_data` compound of `stack`, empty when it carries none.
fn custom_data_of(stack: &ItemStack) -> Compound {
    match data_value(stack) {
        Some(Tag::Compound(data)) => data,
        _ => Compound::new(),
    }
}

/// The stack's `minecraft:custom_data` value, as the component the game keeps.
fn data_value(stack: &ItemStack) -> Option<Tag> {
    stack
        .components()?
        .as_nbt()
        .get(&CUSTOM_DATA.into())
        .cloned()
}

/// Replace `stack`'s `minecraft:custom_data` with `data` (removed when empty),
/// keeping every other component: the 0.1.0 `set_other` wrote one key, so the
/// keys beside it must survive.
fn write_custom_data(stack: &mut ItemStack, data: Compound) {
    let mut patch = Compound::new();
    if data.is_empty() {
        let removal = format!("!{CUSTOM_DATA}");
        patch.insert(removal.as_str().into(), Tag::Compound(Compound::new()));
    } else {
        patch.insert(CUSTOM_DATA.into(), Tag::Compound(data));
    }
    let _ = stack.apply_components(&patch);
}

/// Change `stack`'s component data and mirror the charge onto its bar, which is
/// the durability bar upstream's `getBarWidth` draws from the FE buffer.
fn edit_data(stack: &mut ItemStack, change: impl FnOnce(&mut Compound)) {
    let mut data = custom_data_of(stack);
    change(&mut data);
    let energy = energy_of(&data);
    write_custom_data(stack, data);
    stack.set_damage(bar_damage(energy));
}

/// One item of `stack` (a stack of one) with `amount` more or fewer FE: what it
/// moved. `applied` is the item's new energy.
fn moved_toward(stack: &mut ItemStack, target: i32) -> i32 {
    let before = energy_of(&custom_data_of(stack));
    edit_data(stack, |data| apply_energy(data, target));
    energy_of(&custom_data_of(stack)) - before
}

// ---------------------------------------------------------------------------
// The item energy capability (`Capabilities.Energy.ITEM`, API 307)
// ---------------------------------------------------------------------------

/// The gadget's FE buffer as `Capabilities.Energy.ITEM`: 1,000,000 FE, as much
/// per call as the buffer's room or contents, kept in the item where it lives
/// (the write rewrites that item in the caller's transaction).
pub(crate) struct GadgetEnergy {
    place: ItemPlace,
}

impl GadgetEnergy {
    /// The buffer of the gadget at `place`.
    pub(crate) fn new(place: ItemPlace) -> Self {
        Self { place }
    }

    /// What the gadget at the place holds now.
    fn held(&self, host: &mut dyn ServerHost) -> i32 {
        self.place
            .stack(host)
            .map_or(0, |stack| energy_of(&custom_data_of(&stack)))
    }

    /// Move the gadget to `target` FE and answer how much that moved (negative
    /// when it drained), or 0 when the write was refused.
    fn move_to(&self, host: &mut dyn ServerHost, tx: &Transaction, target: i32) -> i32 {
        let mut moved = 0;
        let went = self.place.edit(host, tx, |stack| {
            moved = moved_toward(stack, target);
        });
        if went { moved } else { 0 }
    }
}

impl EnergyHandler for GadgetEnergy {
    fn amount(&self, host: &mut dyn ServerHost) -> u32 {
        u32::try_from(self.held(host)).unwrap_or(0)
    }

    fn capacity(&self, _: &mut dyn ServerHost) -> u32 {
        u32::try_from(MAX_ENERGY).unwrap_or(u32::MAX)
    }

    fn insert(&self, host: &mut dyn ServerHost, tx: &Transaction, amount: u32) -> u32 {
        let held = self.held(host);
        let room = MAX_ENERGY.saturating_sub(held);
        let went = i32::try_from(amount)
            .unwrap_or(i32::MAX)
            .min(MAX_ENERGY)
            .min(room);
        if went <= 0 {
            return 0;
        }
        u32::try_from(self.move_to(host, tx, held + went)).unwrap_or(0)
    }

    fn extract(&self, host: &mut dyn ServerHost, tx: &Transaction, amount: u32) -> u32 {
        let held = self.held(host);
        let took = i32::try_from(amount)
            .unwrap_or(i32::MAX)
            .min(MAX_ENERGY)
            .min(held);
        if took <= 0 {
            return 0;
        }
        u32::try_from(-self.move_to(host, tx, held - took)).unwrap_or(0)
    }
}

/// Pay `cost` FE out of the gadget at `place`; false when it cannot pay all of
/// it (nothing is spent then).
fn spend(host: &mut dyn ServerHost, place: &ItemPlace, cost: i32) -> bool {
    if cost <= 0 {
        return true;
    }
    let Some(energy) = place.capability(host, &mod_api::ITEM_ENERGY) else {
        return false;
    };
    let want = u32::try_from(cost).unwrap_or(u32::MAX);
    let took = host
        .transaction(|host, tx| Ok::<u32, Infallible>(energy.extract(host, tx, want)))
        .unwrap_or(0);
    if i32::try_from(took).unwrap_or(0) >= cost {
        return true;
    }
    // Short by a part: the buffer gave what it had, so give it back.
    let _ = host.transaction(|host, tx| Ok::<u32, Infallible>(energy.insert(host, tx, took)));
    false
}

/// Give `cost` FE back to the gadget at `place`, when a break it paid for did
/// not happen.
fn refund(host: &mut dyn ServerHost, place: &ItemPlace, cost: i32) {
    if cost <= 0 {
        return;
    }
    if let Some(energy) = place.capability(host, &mod_api::ITEM_ENERGY) {
        let back = u32::try_from(cost).unwrap_or(u32::MAX);
        let _ = host.transaction(|host, tx| Ok::<u32, Infallible>(energy.insert(host, tx, back)));
    }
}

// ---------------------------------------------------------------------------
// Who may break what
// ---------------------------------------------------------------------------

/// Whether `player` may break the block at `pos`, as `Player.blockActionRestricted`
/// asked it in 0.1.0: a spectator breaks nothing and an adventure player only
/// what its tool allows.
///
/// The pin exposes no read of an adventure stack's `minecraft:can_break`
/// predicate, so an adventure player's shot is refused rather than guessed —
/// stricter than upstream, and recorded as a gap in `notes.md`.
fn may_break(host: &dyn Host, player: PlayerRef, _pos: Pos) -> bool {
    !matches!(
        player.game_mode(host),
        Some(GameMode::Spectator | GameMode::Adventure)
    )
}

// ---------------------------------------------------------------------------
// The block a target is, for the pure rules
// ---------------------------------------------------------------------------

/// The block at `pos`, as the pure target rules take it.
fn candidate(host: &mut dyn ServerHost, level: LevelRef, pos: Pos) -> Candidate {
    let Some(state) = level.block_state(&*host, pos) else {
        // Not loaded, out of height: nothing a player could mine.
        return Candidate {
            air: true,
            ..Candidate::default()
        };
    };
    let id = level.block(&*host, pos).map(|block| block.id(&*host));
    let states = host.block_states();
    Candidate {
        air: id.as_deref().is_some_and(|id| {
            matches!(
                id,
                "minecraft:air" | "minecraft:cave_air" | "minecraft:void_air"
            )
        }),
        fluid: level
            .fluid_state(&*host, pos)
            .is_some_and(|fluid| !fluid.is_empty()),
        waterlogged: level.block_property(&*host, pos, "waterlogged").as_deref() == Some("true"),
        hardness: states.destroy_speed(state).unwrap_or(-1.0),
        block_entity: states.has_block_entity(state).unwrap_or(false),
        // Upstream's `DoorBlock` class test, by id: every door is `<x>_door`.
        door: id.as_deref().is_some_and(|id| id.ends_with("_door")),
    }
}

/// A face as the unit vector the pure plane rules take (`BlockFace` is the
/// enum, `Direction`'s normal is what `MiningCollect.collect` reads).
fn face_vector(face: BlockFace) -> Pos {
    match face {
        BlockFace::Down => [0, -1, 0],
        BlockFace::Up => [0, 1, 0],
        BlockFace::North => [0, 0, -1],
        BlockFace::South => [0, 0, 1],
        BlockFace::West => [-1, 0, 0],
        BlockFace::East => [1, 0, 0],
        _ => [0, 1, 0],
    }
}

/// The Haste level (1 for Haste I) `player` carries, 0 when it has none.
fn haste_level(host: &dyn Host, player: PlayerRef) -> i32 {
    effect_level(host, player, "minecraft:haste").unwrap_or(0)
}

/// Whether `player` carries Mining Fatigue.
fn has_fatigue(host: &dyn Host, player: PlayerRef) -> bool {
    effect_level(host, player, "minecraft:mining_fatigue").is_some()
}

/// The level (amplifier + 1) of `effect` on `player`, `None` when absent.
fn effect_level(host: &dyn Host, player: PlayerRef, effect: &str) -> Option<i32> {
    let living = host.living(player.entity())?;
    let effects = living.effects(host)?;
    effects
        .iter()
        .find(|active| active.id.id(host) == effect)
        .map(|active| active.amplifier + 1)
}

// ---------------------------------------------------------------------------
// The laser
// ---------------------------------------------------------------------------

/// One right click of a held gadget: collect the aimed plane, drop what the game
/// will not let this player break, break what the buffer pays for, and start the
/// item's cooldown for the dig time.
///
/// 0.1.0's `native_machine_destroy_block(pos, Some(&tool), false)` is
/// `LevelRef::destroy_block` here, with the acting player's own break rules
/// checked before each block is charged for.
fn fire(host: &mut dyn ServerHost, player: PlayerRef, hand: Hand, start: Pos, face: Pos) {
    let place = ItemPlace::PlayerHand {
        player: player.uuid(),
        hand,
    };
    let Some(stack) = place.stack(host) else {
        return;
    };
    if !is_gadget(&stack.item().id(&*host)) {
        return;
    }
    let data = custom_data_of(&stack);
    let upgrades = read_upgrades(&data);
    let range = clamp_range(read_range(&data).min(max_range(&upgrades)));
    let body = player.entity();
    let facing = body
        .head_yaw(&*host)
        .map_or([0, 0, 1], |yaw| facing_from_yaw(f64::from(yaw)));
    let player_y = body.position(&*host).map_or(f64::NAN, |at| at[1]);
    let Some(level) = player.level(&*host) else {
        return;
    };
    let targets: Vec<(Pos, f32)> = collect(start, face, facing, range, SizeMode::Auto, player_y)
        .into_iter()
        .filter_map(|pos| {
            let candidate = candidate(host, level, pos);
            is_valid(&candidate).then_some((pos, candidate.hardness.max(0.0)))
        })
        .collect();
    if targets.is_empty() {
        return;
    }
    let energy = energy_of(&data);
    if energy < reserve(&upgrades, range) {
        return;
    }
    let cost = cost_per_block(&upgrades);
    let paid = affordable(energy, cost, targets.len());
    // The drops are rolled with a gadget stack carrying the upgrades' own
    // enchantments, as upstream builds `tempTool` in `RenderBlockTileEntity`.
    let tool = {
        let mut tool = stack.clone();
        if let Some(patch) = tool_enchantments(&upgrades) {
            let _ = tool.apply_components(&patch);
        }
        tool
    };
    let mut broken = 0;
    let mut hardness = Vec::with_capacity(paid);
    for (pos, of_block) in targets.iter().take(paid) {
        if !may_break(&*host, player, *pos) {
            continue;
        }
        if !spend(host, &place, cost) {
            break;
        }
        let dropped = matches!(
            level.destroy_block(host, *pos, true, Some(&tool), false),
            Ok(Some(_))
        );
        if !dropped {
            // The world refused the break (an unloaded chunk, a parked level):
            // nothing happened, so nothing is paid.
            refund(host, &place, cost);
            continue;
        }
        hardness.push(*of_block);
        broken += 1;
    }
    if broken == 0 {
        return;
    }
    let ticks = break_ticks(
        &hardness,
        range,
        tier(&upgrades, "efficiency_1"),
        haste_level(&*host, player),
        has_fatigue(&*host, player),
    );
    let group = host.item_cooldown_group(&stack);
    let _ = player.set_item_cooldown(host, &group, ticks);
}

/// `Hooks::right_click_block`: a held gadget fires at the aimed face and the
/// click is consumed *before* the block's own use, so a lever or a door in front
/// of the gadget is not also toggled by the shot. The retired bundle cancelled
/// with a success, which is what `cancellation_result` says here.
pub(crate) fn right_click(host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
    let pos = event.click.pos;
    let face = event.click.hit.face;
    let hand = event.click.hand;
    let uuid = event.click.player;
    let held = match host {
        HostSide::Server(host) => {
            let held = event
                .click
                .stack
                .is_some_and(|stack| is_gadget(&stack.item().id(&*host)));
            if held && let Some(player) = host.player(uuid) {
                fire(host, player, hand, pos, face);
            }
            held
        }
        HostSide::Client(host) => event
            .click
            .stack
            .is_some_and(|stack| is_gadget(&stack.item().id(host))),
    };
    if !held {
        return Flow::Continue;
    }
    *event.cancellation_result = InteractionResult::Success;
    Flow::Cancel
}

/// `ItemPart::use_item`: a right click in the air ray-casts to the beam range and
/// fires at the face the ray met. The click is claimed (the item's own use never
/// runs), as the retired `use_item` hook's `claimed` was.
pub(crate) fn use_in_air(
    host: HostSide<'_>,
    event: &mut UseItemEvent<'_>,
) -> Option<InteractionResult> {
    if let HostSide::Server(host) = host
        && let Some(player) = host.player(event.player)
        && let Some(hit) = player.entity().pick(&*host, f64::from(MIN_BEAM_RANGE))
        && !hit.miss
    {
        fire(host, player, event.hand, hit.block, face_vector(hit.face));
    }
    Some(InteractionResult::Success)
}

/// The gadget's tooltip: its charge, its per-block price and its range.
pub(crate) fn tooltip(_host: &dyn ClientHost, event: &mut TooltipEvent<'_>) {
    let data = custom_data_of(event.stack);
    let upgrades = read_upgrades(&data);
    let energy = energy_of(&data);
    let range = clamp_range(read_range(&data).min(max_range(&upgrades)));
    event.lines.push((
        Text::translate(
            "mininggadgets.gadget.energy",
            vec![
                Text::literal(energy.to_string()),
                Text::literal(MAX_ENERGY.to_string()),
            ],
        ),
        0xFF55FF55,
    ));
    event.lines.push((
        Text::translate(
            "mininggadgets.tooltip.item.break_cost",
            vec![Text::literal(cost_per_block(&upgrades).to_string())],
        ),
        0xFFFF5555,
    ));
    event.lines.push((
        Text::translate(
            "mininggadgets.gadget.range_change",
            vec![Text::literal(range.to_string())],
        ),
        0xFFAAAAAA,
    ));
}

// ---------------------------------------------------------------------------
// The commands
// ---------------------------------------------------------------------------

/// The acting player's main-hand gadget, `None` when the player holds anything
/// else (or is not a player at all).
fn held_gadget(host: &mut dyn ServerHost, player: PlayerRef) -> Option<ItemStack> {
    let place = ItemPlace::PlayerHand {
        player: player.uuid(),
        hand: Hand::Main,
    };
    let stack = place.stack(host)?;
    is_gadget(&stack.item().id(&*host)).then_some(stack)
}

/// `/mininggadgets info`: the report line, or `None` when the player holds no
/// gadget.
pub(crate) fn info_line(host: &mut dyn ServerHost, player: PlayerRef) -> Option<String> {
    let stack = held_gadget(host, player)?;
    let time = player
        .level(&*host)
        .and_then(|level| level.game_time(&*host))
        .unwrap_or(0);
    Some(Sight::of(&stack, time).line())
}

/// `/mininggadgets energy <amount>`: charge or drain the held gadget to `amount`
/// FE, answering what it now holds.
pub(crate) fn set_energy(host: &mut dyn ServerHost, player: PlayerRef, target: i32) -> Option<i32> {
    held_gadget(host, player)?;
    let place = ItemPlace::PlayerHand {
        player: player.uuid(),
        hand: Hand::Main,
    };
    let energy = place.capability(host, &mod_api::ITEM_ENERGY)?;
    let held = i32::try_from(energy.amount(host)).unwrap_or(i32::MAX);
    let target = target.clamp(0, MAX_ENERGY);
    if target > held {
        let offer = u32::try_from(target - held).unwrap_or(u32::MAX);
        let _ = host.transaction(|host, tx| Ok::<u32, Infallible>(energy.insert(host, tx, offer)));
    } else if target < held {
        let want = u32::try_from(held - target).unwrap_or(u32::MAX);
        let _ = host.transaction(|host, tx| Ok::<u32, Infallible>(energy.extract(host, tx, want)));
    }
    Some(i32::try_from(energy.amount(host)).unwrap_or(0))
}

/// `/mininggadgets range <size>`: set the held gadget's mining range, never past
/// what its size upgrades allow. Answers the range it stored and the maximum.
pub(crate) fn set_range(
    host: &mut dyn ServerHost,
    player: PlayerRef,
    size: i32,
) -> Option<(i32, i32)> {
    let stack = held_gadget(host, player)?;
    let allowed = max_range(&read_upgrades(&custom_data_of(&stack)));
    let range = clamp_range(size).min(allowed);
    edit_held(host, player, |data| write_range(data, range));
    Some((range, allowed))
}

/// `/mininggadgets upgrade <name>`: add an upgrade to the held gadget, or clear
/// the whole list with `clear`. Refuses a duplicate and a name the mod does not
/// know. Answers the summary, the new price and the range.
pub(crate) fn add_upgrade(
    host: &mut dyn ServerHost,
    player: PlayerRef,
    name: &str,
) -> Result<String, String> {
    let stack =
        held_gadget(host, player).ok_or_else(|| "Hold a Mining Gadget first.".to_owned())?;
    let data = custom_data_of(&stack);
    let mut upgrades = read_upgrades(&data);
    if name == "clear" {
        upgrades.clear();
    } else if crate::runtime::find(&upgrades, name).is_some() {
        return Err(format!("The gadget already has {name}."));
    } else if !crate::runtime::known_upgrade(name) {
        return Err(format!("{name} is not an upgrade."));
    } else {
        upgrades.push(UpgradeEntry::new(name));
    }
    // The chosen range never exceeds what the new size allows.
    let range = clamp_range(read_range(&data).min(max_range(&upgrades)));
    edit_held(host, player, |data| {
        write_upgrades(data, &upgrades);
        write_range(data, range);
    });
    Ok(format!(
        "Mining Gadget upgrades: {}, per block {}, range {range} of {}.",
        upgrades_summary(&upgrades),
        cost_per_block(&upgrades),
        max_range(&upgrades)
    ))
}

/// Change the held gadget's component data in one transaction.
fn edit_held(
    host: &mut dyn ServerHost,
    player: PlayerRef,
    change: impl FnOnce(&mut Compound),
) -> bool {
    let place = ItemPlace::PlayerHand {
        player: player.uuid(),
        hand: Hand::Main,
    };
    host.transaction(|host, tx| {
        Ok::<bool, Infallible>(place.edit(host, tx, |stack| edit_data(stack, change)))
    })
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{BASE_COST, cost_per_block};

    fn upgrades(names: &[&str]) -> Vec<UpgradeEntry> {
        names.iter().map(|name| UpgradeEntry::new(name)).collect()
    }

    /// A key of a compound. `Compound` is a `BTreeMap<JavaString, Tag>` and the
    /// key type is not exported, so every read goes through `Into` (trap 13).
    fn at<'a>(data: &'a Compound, key: &str) -> Option<&'a Tag> {
        data.get(&key.into())
    }

    /// A nested key: `path[0]` in `data`, then each key of the `Tag` before it.
    fn nested<'a>(data: &'a Compound, path: &[&str]) -> Option<&'a Tag> {
        let (first, rest) = path.split_first()?;
        let mut tag = at(data, first)?;
        for key in rest {
            tag = tag.get(key)?;
        }
        Some(tag)
    }

    fn energy_data() -> Compound {
        // A gadget that already carries its 0.1.0 keys.
        let mut data = Compound::new();
        write_upgrades(&mut data, &upgrades(&["silk", "range_2"]));
        write_range(&mut data, 3);
        apply_energy(&mut data, 9_800);
        data
    }

    #[test]
    fn only_the_jars_three_gadgets_are_gadgets() {
        for id in GADGETS {
            assert!(is_gadget(id), "{id} is a gadget");
        }
        assert!(!is_gadget("mininggadgets:upgrade_silk"));
        assert!(!is_gadget("minecraft:stick"));
    }

    #[test]
    fn the_upgrades_round_trip_through_the_component_0_1_0_wrote() {
        let mut data = Compound::new();
        let list = vec![
            UpgradeEntry::new("silk"),
            UpgradeEntry {
                name: "fortune_1".to_owned(),
                active: false,
            },
        ];
        write_upgrades(&mut data, &list);
        assert_eq!(read_upgrades(&data), list);
        // The 0.1.0 shape, so a gadget made before the migration keeps its list.
        let Some(Tag::List(entries)) = at(&data, UPGRADES) else {
            panic!("the upgrade list is a list under `{UPGRADES}`");
        };
        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].get(NAME),
            Some(&Tag::String("silk".into())),
            "each entry is a compound with `name`"
        );
        assert_eq!(
            entries[0].get(ACTIVE),
            Some(&Tag::Byte(1)),
            "and `active` as a byte"
        );
        assert_eq!(entries[1].get(ACTIVE), Some(&Tag::Byte(0)));
    }

    #[test]
    fn an_empty_upgrade_list_writes_no_key() {
        let mut data = energy_data();
        write_upgrades(&mut data, &[]);
        assert!(
            at(&data, UPGRADES).is_none(),
            "an empty list is removed, not written empty"
        );
        assert_eq!(read_upgrades(&data), Vec::new());
    }

    #[test]
    fn the_range_is_one_until_it_is_written_and_one_removes_it() {
        let mut data = Compound::new();
        assert_eq!(read_range(&data), 1, "1 is the default");
        write_range(&mut data, 5);
        assert_eq!(read_range(&data), 5);
        assert_eq!(at(&data, RANGE), Some(&Tag::Int(5)));
        write_range(&mut data, 1);
        assert!(
            at(&data, RANGE).is_none(),
            "the default is stored as no key at all"
        );
        assert_eq!(read_range(&data), 1);
    }

    #[test]
    fn the_charge_is_the_zero_point_one_key_and_zero_removes_it() {
        let mut data = Compound::new();
        assert_eq!(energy_of(&data), 0, "an uncharged gadget reads empty");
        apply_energy(&mut data, 9_800);
        assert_eq!(energy_of(&data), 9_800);
        assert_eq!(
            at(&data, ENERGY),
            Some(&Tag::Int(9_800)),
            "the retired bundle's `minecraft:custom_data.horizon_energy` int"
        );
        apply_energy(&mut data, 0);
        assert!(
            at(&data, ENERGY).is_none(),
            "a flat gadget carries nothing, so it is the data of a new one"
        );
    }

    #[test]
    fn the_charge_is_clamped_to_the_buffer() {
        let mut data = Compound::new();
        apply_energy(&mut data, MAX_ENERGY + 5_000);
        assert_eq!(energy_of(&data), MAX_ENERGY);
        apply_energy(&mut data, -5_000);
        assert_eq!(energy_of(&data), 0, "a negative charge is no charge");
    }

    #[test]
    fn charging_keeps_the_upgrades_and_the_range_beside_it() {
        // The three keys share one `minecraft:custom_data` compound: writing the
        // charge must not drop its siblings (the retired bundle's bug class).
        let mut data = energy_data();
        apply_energy(&mut data, 8_200);
        assert_eq!(read_upgrades(&data), upgrades(&["silk", "range_2"]));
        assert_eq!(read_range(&data), 3);
        assert_eq!(energy_of(&data), 8_200);
        assert_eq!(
            data.len(),
            3,
            "exactly the three keys the port keeps, nothing else"
        );
    }

    #[test]
    fn the_tool_carries_silk_touch_and_fortune_as_26_3_enchantments() {
        assert_eq!(
            tool_enchantments(&upgrades(&["size_1"])),
            None,
            "no enchantment upgrade, no patch"
        );
        let silk = tool_enchantments(&upgrades(&["silk"])).expect("silk patches the tool");
        assert_eq!(
            nested(
                &silk,
                &["minecraft:enchantments", "levels", "minecraft:silk_touch"]
            ),
            Some(&Tag::Int(1)),
            "26.3 keeps enchantments as `{{levels: {{id: level}}}}`"
        );
        let fortune =
            tool_enchantments(&upgrades(&["silk", "fortune_3"])).expect("fortune patches too");
        let levels =
            nested(&fortune, &["minecraft:enchantments", "levels"]).expect("the levels compound");
        assert_eq!(levels.get("minecraft:silk_touch"), Some(&Tag::Int(1)));
        assert_eq!(levels.get("minecraft:fortune"), Some(&Tag::Int(3)));
        let off = vec![UpgradeEntry {
            name: "fortune_3".to_owned(),
            active: false,
        }];
        assert_eq!(
            tool_enchantments(&off),
            None,
            "an inactive upgrade enchants nothing"
        );
    }

    #[test]
    fn the_summary_names_an_inactive_upgrade() {
        assert_eq!(upgrades_summary(&[]), "empty");
        assert_eq!(upgrades_summary(&upgrades(&["silk"])), "silk");
        assert_eq!(
            upgrades_summary(&upgrades(&["silk", "fortune_1", "size_2"])),
            "silk, fortune_1, size_2"
        );
        let mixed = vec![
            UpgradeEntry::new("silk"),
            UpgradeEntry {
                name: "fortune_1".to_owned(),
                active: false,
            },
        ];
        assert_eq!(upgrades_summary(&mixed), "silk, fortune_1 (off)");
    }

    #[test]
    fn the_info_line_is_the_one_the_e2e_asserts() {
        let sight = Sight {
            energy: 10_000,
            cost: cost_per_block(&[]),
            range: 1,
            max_range: 1,
            upgrades: Vec::new(),
            time: 168,
        };
        assert_eq!(
            sight.line(),
            "Mining Gadget: 10000/1000000 FE, per block 200, range 1 of 1, upgrades empty, t=168"
        );
        assert_eq!(sight.cost, BASE_COST, "the base price of one block");
        let upgraded = Sight {
            energy: 8_200,
            cost: cost_per_block(&upgrades(&["silk", "fortune_2"])),
            range: 3,
            max_range: 3,
            upgrades: upgrades(&["silk", "fortune_2", "size_1"]),
            time: 1032,
        };
        assert_eq!(
            upgraded.line(),
            "Mining Gadget: 8200/1000000 FE, per block 360, range 3 of 3, upgrades silk, \
             fortune_2, size_1, t=1032"
        );
        assert_eq!(
            tier(&upgrades(&["fortune_2"]), "fortune_1"),
            2,
            "the tier the summary prints is the runtime's"
        );
        assert!(active(&upgrades(&["silk"]), "silk"));
    }

    #[test]
    fn a_gadgets_sight_is_read_from_its_own_component_data() {
        // The readout `/mininggadgets info` prints comes from the same three keys
        // the codec writes; the harness reads the line, not the stack.
        let data = energy_data();
        let sight = Sight::of_data(&data, 527);
        // The plane is the *size* upgrade's, so a range upgrade alone (the stored
        // range is 3) still mines one block, and silk adds its 100 FE a block.
        assert_eq!(
            sight.line(),
            "Mining Gadget: 9800/1000000 FE, per block 300, range 1 of 1, upgrades silk, range_2, \
             t=527"
        );
        let mut sized = energy_data();
        write_upgrades(&mut sized, &upgrades(&["silk", "range_2", "size_2"]));
        let sight = Sight::of_data(&sized, 1032);
        assert_eq!(
            sight.line(),
            "Mining Gadget: 9800/1000000 FE, per block 300, range 3 of 5, upgrades silk, range_2, \
             size_2, t=1032"
        );
        assert_eq!(sight.energy, energy_of(&sized));
        assert_eq!(sight.cost, cost_per_block(&read_upgrades(&sized)));
    }
}
