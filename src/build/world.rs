use graphite_server::{world::{WorldService, World, TickPhase}, player::{proto_player::ProtoPlayer, player_vec::PlayerVec}, entity::position::{Position, Coordinate, Rotation}, gamemode::GameMode, universe::Universe, WorldTicker};
use graphite_server::ticker::WorldTicker;

use crate::BuildUniverse;

use super::player::BuildPlayer;

#[derive(WorldTicker)]
pub struct BuildWorld {
    pub players: PlayerVec<BuildPlayer>,
}

impl WorldService for BuildWorld {
    type UniverseServiceType = BuildUniverse;
    type ParentWorldServiceType = Self;

    const CHUNK_VIEW_DISTANCE: u8 = 8;
    const ENTITY_VIEW_DISTANCE: u8 = 8;
    const SHOW_DEFAULT_WORLD_BORDER: bool = true;

    fn handle_player_join(
        world: &mut World<Self>,
        mut proto_player: ProtoPlayer<Self::UniverseServiceType>,
    ) {
        proto_player.abilities.gamemode = GameMode::Creative;
        proto_player.hardcore = true;

        // make player from proto_player
        world
            .service
            .players
            .add(
                proto_player,
                BuildPlayer {},
                Position {
                    coord: Coordinate {
                        x: 8.0,
                        y: 224.0,
                        z: 8.0,
                    },
                    rot: Rotation {
                        yaw: 0.0,
                        pitch: 0.0,
                    },
                },
            )
            .unwrap();
    }
}