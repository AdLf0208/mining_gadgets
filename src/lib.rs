//! Mining Gadgets for Horizon — slice 1, the laser and its FE.
//!
//! One flat SDK package (mod API 0.2): the three gadget tiers and the 24 upgrade
//! cards are the carried `assets/` and `data/` here, and the behaviour is the
//! one `impl Hooks` below. The rules themselves live in [`runtime`],
//! byte-identical to the retired `.hnmod` bundle's `runtime.rs`; [`gadget`] is
//! the glue.
//!
//! What the retired bundle expressed as seven injected helpers is here three
//! shapes: a held gadget's right click and its air use are typed hooks and an
//! **item Part** (`Gadget`), its FE buffer is a declared `ITEM_ENERGY`
//! capability ([`gadget::GadgetEnergy`]), and `/mininggadgets` is the typed
//! `register_commands`/`run_mod_command` pair. Its `embedded-content` helper has
//! no successor: one `.hmod` carries the content and the code, which is what
//! that helper existed to work around.
mod gadget;

pub mod runtime;

use mod_api::{
    self, ClientHost, Command, CommandArgumentValue, CommandSource, ComponentTypesEvent, Flow,
    Hooks, HostSide, InteractionResult, ItemPart, RegisterCommandsEvent, Registrar,
    RegistrationContext, RightClickBlockEvent, RunModCommandEvent, ServerHost, Text, TooltipEvent,
    UseItemEvent,
};

use crate::runtime::MAX_ENERGY;

#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the gadget: the three tiers' item Part (the air use and the
    /// tooltip) and their item energy capability.
    ///
    /// The retired bundle answered `item_energy_buffer` out of a shared output for
    /// every item the game asked about; a declaration is keyed by the item, so a
    /// foreign item cannot be given a buffer by mistake.
    fn register(&self, registrar: &mut dyn Registrar, _context: &RegistrationContext) {
        for id in gadget::GADGETS {
            // Bind the key first: `item_part(registrar.item_key(..), ..)` is two
            // mutable borrows of the registrar (trap 15).
            let key = registrar.item_key(id);
            registrar.item_part(key, Gadget);
        }
        for id in gadget::GADGETS {
            registrar.item_capability(&mod_api::ITEM_ENERGY, id, |_, place| {
                Some(Box::new(gadget::GadgetEnergy::new(place.clone())))
            });
        }
    }

    /// The component ids the gadget's own data model names, registered before
    /// content loads, as the retired `component_types` hook did.
    fn component_types(&self, _context: &RegistrationContext, event: &mut ComponentTypesEvent<'_>) {
        event.types.push(gadget::UPGRADES.to_owned());
        event.types.push(gadget::RANGE.to_owned());
    }

    /// A right click with a held gadget: the laser fires at the aimed face and the
    /// click is consumed before the block's own use.
    fn right_click_block(&self, host: HostSide<'_>, event: &mut RightClickBlockEvent<'_>) -> Flow {
        gadget::right_click(host, event)
    }

    /// `/mininggadgets info|energy|range|upgrade`: the harness controls the in-game
    /// check uses (there is no modification table or settings screen in slice 1).
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        event.commands.push(Command::new("mininggadgets info"));
        event
            .commands
            .push(Command::op("mininggadgets energy <amount:integer>", 2));
        event
            .commands
            .push(Command::op("mininggadgets range <size:integer>", 2));
        event
            .commands
            .push(Command::op("mininggadgets upgrade <name:word>", 2));
    }

    /// The command bodies, in the acting player's own state. The arguments are
    /// parsed and resolved by the server's dispatcher, so this only reads the held
    /// gadget and answers — with the retired bundle's wording, which the in-game
    /// check matches.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
        let player = match event.source {
            CommandSource::Player { player, .. } => Some(*player),
            _ => None,
        };
        let integer = |name: &str| match event.command.argument(name) {
            Some(CommandArgumentValue::Integer(amount)) => Some(*amount),
            _ => None,
        };
        let word = |name: &str| match event.command.argument(name) {
            Some(CommandArgumentValue::String(word)) => Some(word.clone()),
            _ => None,
        };
        let line = match (literals.as_slice(), player) {
            (["mininggadgets", "info"], Some(player)) => gadget::info_line(host, player)
                .unwrap_or_else(|| "Hold a Mining Gadget first.".to_owned()),
            (["mininggadgets", "energy"], Some(player)) => match integer("amount") {
                Some(amount) => match gadget::set_energy(host, player, amount) {
                    Some(energy) => {
                        format!("Mining Gadget energy set to {energy}/{MAX_ENERGY} FE.")
                    }
                    None => "Hold a Mining Gadget first.".to_owned(),
                },
                None => "Usage: /mininggadgets energy <amount>".to_owned(),
            },
            (["mininggadgets", "range"], Some(player)) => match integer("size") {
                Some(size) => match gadget::set_range(host, player, size) {
                    Some((range, allowed)) => {
                        format!("Mining Gadget range set to {range} of {allowed}.")
                    }
                    None => "Hold a Mining Gadget first.".to_owned(),
                },
                None => "Usage: /mininggadgets range <size>".to_owned(),
            },
            (["mininggadgets", "upgrade"], Some(player)) => match word("name") {
                Some(name) => {
                    gadget::add_upgrade(host, player, &name).unwrap_or_else(|refusal| refusal)
                }
                None => "Usage: /mininggadgets upgrade <name>".to_owned(),
            },
            (["mininggadgets", _], None) => "Hold a Mining Gadget first.".to_owned(),
            _ => return Flow::Continue,
        };
        let _ = host.commands().feedback(Text::literal(line));
        Flow::Cancel
    }
}

/// The three gadget tiers' Part: the air use and the tooltip. A right click on a
/// *block* is the global hook, because it must be able to claim the click before
/// the block's own use; these two are per item, so the game calls them only for
/// the gadget and the port never has to ask which item it holds.
struct Gadget;

#[mod_api::horizon_part]
impl ItemPart for Gadget {
    /// `Item.use`: a right click in the air ray-casts to the beam range and fires.
    fn use_item(
        &self,
        host: HostSide<'_>,
        event: &mut UseItemEvent<'_>,
    ) -> Option<InteractionResult> {
        gadget::use_in_air(host, event)
    }

    /// The gadget's tooltip: its charge, its per-block price and its range.
    fn tooltip(&self, host: &dyn ClientHost, event: &mut TooltipEvent<'_>) {
        gadget::tooltip(host, event);
    }
}
