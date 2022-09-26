use std::env;
use std::fs::File;
use std::io::Write;
use std::sync::mpsc::Sender;

use build::world::BuildWorld;
use graphite_command::brigadier;
use graphite_command::types::CommandResult;
use graphite_concierge::Concierge;
use graphite_concierge::ConciergeService;
use graphite_net::network_handler::UninitializedConnection;
use graphite_mc_protocol::types::GameProfile;
use graphite_server::{UniverseTicker, WorldTicker};
use graphite_server::entity::position::Coordinate;
use graphite_server::entity::position::Position;
use graphite_server::entity::position::Rotation;
use graphite_server::gamemode::GameMode;
use graphite_server::inventory::inventory_handler::VanillaPlayerInventory;
use graphite_server::player::player_connection::ConnectionReference;
use graphite_server::player::player_vec::PlayerVec;
use graphite_server::player::proto_player::ProtoPlayer;
use graphite_server::player::Player;
use graphite_server::player::PlayerService;
use graphite_server::universe::Universe;
use graphite_server::universe::UniverseService;
use graphite_server::world::TickPhase;
use graphite_server::world::World;
use graphite_server::ticker::{WorldTicker};
use graphite_server::world::world_map::WorldMap;
use graphite_server::world::WorldService;
use graphite_sticky::Unsticky;

use crate::build::player::BuildPlayer;

mod build;

// todo: world ticker derive
// todo: brigadier strings
// todo: graphite chat
// todo: graphite tablist options: UniverseShared, WorldShared, PlayerView
// todo: world save/load from local file
// todo: world autosave
// todo: world github sync

struct MyConciergeImpl {
    universe_sender: Sender<(UninitializedConnection, GameProfile)>,
}

impl ConciergeService for MyConciergeImpl {
    fn get_serverlist_response(&mut self) -> String {
        "{\
            \"version\": {
                \"name\": \"1.19.1\",
                \"protocol\": 760
            },
            \"players\": {
                \"max\": 0,
                \"online\": 0,
                \"sample\": []
            },
            \"description\": {
                \"text\": \"Hello world\"
            },
            \"favicon\": \"data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAIAAAAlC+aJAAABGklEQVRo3u2aUQ7EIAhEbcNReiPP6Y16F/djk/1bozJASYffJu08BRxMj957yRxnSR4EIMDbAQTylrvWwdOrNTuAY6+NjhV7YiwDbEg3xVgDUKq3wIgp4rtW1FqYAEwuMAQDk0L/FE/q02TUqVR/tTb4vGkDBaTQjL4xIU/i91gJVNeDV8gZ+HnIorAGCJAAwKIBAACAhixyIvsyKL3Qg0bKqzXnbZlNoXmH/NwitvBkeuC1Ira2lk5daBvDAn6/iH9qAi+Fyva9EDDvlYTxVkJZx/RCBMgHgO1L3IEXAmANn+SV7r0DRk5b0im2BfAfaCRcn/JYkBIXwXejDzmPJZ1iVwCHAfrgD08EIAABCEAAAhCAAAQgwG58AEFWdXlZzlUbAAAAAElFTkSuQmCC\"
        }".into()
    }

    fn accept_player(
        &mut self,
        player_connection: UninitializedConnection,
        mut concierge_connection: graphite_concierge::ConciergeConnection<Self>,
    ) {
        let join_data = (
            player_connection,
            concierge_connection.game_profile.take().unwrap(),
        );

        self.universe_sender.send(join_data).unwrap();
    }
}

fn main() {
    #[brigadier("save")]
    fn save(player: &mut Player<LobbyPlayer>) -> CommandResult {
        let world = player.get_world();
        let chunks = world.get_chunks();
        let output = graphite_magma::to_magma(chunks, 0);

        let dest_path = env::current_dir().unwrap().join("world.magma");
        let mut f = File::create(&dest_path).unwrap();
        f.write_all(output.unwrap().as_slice()).unwrap();

        Ok(())
    }

    #[brigadier("expand", {}, {}, {})]
    fn expand(player: &mut Player<BuildPlayer>, size_x: isize, size_y: isize, size_z: isize) -> CommandResult {
        let world = player.get_world_mut();
        world.expand(size_x, size_y, size_z);
        Ok(())
    }

    // todo: add string support to arguments for brigadier

    #[brigadier("load", {})]
    #[brigadier_players(LobbyPlayer, BuildPlayer)]
    fn load<P: PlayerService<UniverseServiceType = BuildUniverse>>(player: &mut Player<P>, name: u8) -> CommandResult {
        player.transfer(Box::from(move |world: &mut World<P::WorldServiceType>, _old_service, proto_player| {
            println!("load");
            let name = name.to_string();

            world.get_universe().service.maps.get_or_default(name, || {
                World::new_with_default_chunks(BuildWorld {
                    players: PlayerVec::new(),
                }, 1, 24, 1)
            }).handle_player_join(proto_player);
        }));
        Ok(())
    }

    save.merge(load).unwrap();
    save.merge(expand).unwrap();

    let (dispatcher, packet) =
        graphite_command::minecraft::create_dispatcher_and_brigadier_packet(save);

    let universe_sender = Universe::create_and_start(
        || BuildUniverse {
            lobby: {
                World::new_with_default_chunks(LobbyWorld {
                    players: PlayerVec::new(),
                }, 6, 24, 6)
            },
            maps: WorldMap::new()
        },
        Some((dispatcher, packet)),
    );

    Concierge::bind("127.0.0.1:25565", MyConciergeImpl { universe_sender }).unwrap();
}

// universe

#[derive(UniverseTicker)]
pub struct BuildUniverse {
    lobby: World<LobbyWorld>,
    maps: WorldMap<String, BuildWorld>
}

impl UniverseService for BuildUniverse {
    type ConnectionReferenceType = ConnectionReference<Self>;

    fn handle_player_join(universe: &mut Universe<Self>, proto_player: ProtoPlayer<Self>) {
        universe.service.lobby.handle_player_join(proto_player);
    }
}

// world

#[derive(WorldTicker)]
struct LobbyWorld {
    players: PlayerVec<LobbyPlayer>,
}

impl WorldService for LobbyWorld {
    type UniverseServiceType = BuildUniverse;
    type ParentWorldServiceType = Self;

    const CHUNK_VIEW_DISTANCE: u8 = 3;
    const ENTITY_VIEW_DISTANCE: u8 = 3;
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
                LobbyPlayer {},
                Position {
                    coord: Coordinate {
                        x: 32.0,
                        y: 224.0,
                        z: 32.0,
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

// player

struct LobbyPlayer {}

impl PlayerService for LobbyPlayer {
    const FAST_PACKET_RESPONSE: bool = true;

    type UniverseServiceType = BuildUniverse;
    type WorldServiceType = LobbyWorld;
    type InventoryHandlerType = VanillaPlayerInventory;
}
