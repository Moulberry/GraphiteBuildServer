use graphite_server::{player::PlayerService, inventory::inventory_handler::VanillaPlayerInventory};

use crate::BuildUniverse;

use super::world::BuildWorld;

pub struct BuildPlayer {}

impl PlayerService for BuildPlayer {
    const FAST_PACKET_RESPONSE: bool = true;

    type UniverseServiceType = BuildUniverse;
    type WorldServiceType = BuildWorld;
    type InventoryHandlerType = VanillaPlayerInventory;
}
