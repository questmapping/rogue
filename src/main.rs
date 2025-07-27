#![allow(unused_imports)]
use rltk::{GameState, Rltk, VirtualKeyCode, RGB};
mod components;
mod map;
mod rect;
use map::{draw_map, dungeon_map, wilderness_map, Building, Forest, SnowyMountains, Tile, Volcano, xy_idx};
mod player;
use player::{PlayerAction, PlayerIntent};
use specs::prelude::*;
use crate::components::*;
use specs_derive::Component;
use std::cmp::{max, min};
mod visibility_system;
use visibility_system::VisibilitySystem;


// Lo State contiene il mondo ECS, poi lo implementiamo per i sistemi
struct State {
    ecs: World,
}




// PLAYER ACTIONS -----------------------------------------------------------
// La funzione di movimento non cambia. per altre azioni aggiungeremo in seguito le relative funzioni


fn try_move_player(delta_x: i32, delta_y: i32, ecs: &mut World) {
    // --- Phase 1: Read-only checks --- //
    let intent = {
        let players = ecs.read_storage::<Player>();
        let positions = ecs.read_storage::<Position>();
        let sizes = ecs.read_storage::<CharacterSize>();
        // We fetch the whole Map resource. Previously, this was incorrectly fetching `Vec<Tile>`,
        // which caused a panic because the resource did not exist.
        let map = ecs.fetch::<map::Map>();

        let mut intent = PlayerIntent::DoNothing;

        for (_player, pos, size) in (&players, &positions, &sizes).join() {
            let dest_x = pos.x + delta_x;
            let dest_y = pos.y + delta_y;

            // Boundary check
            if dest_x < 0 || dest_x > 79 || dest_y < 0 || dest_y > 49 {
                intent = PlayerIntent::DoNothing;
                break; // Don't try to move out of bounds
            }
            let dest_idx = xy_idx(dest_x, dest_y);

            // Access the `tiles` field of the `map` resource to check the door state.
            if let Some(door_state) = map.tiles[dest_idx].door_state {
                if door_state == map::DoorState::Closed || door_state == map::DoorState::Locked {
                    intent = PlayerIntent::OpenDoor(dest_idx);
                    break;
                }
            }

            let mut can_move_to_dest = true;
            if delta_x != 0 && delta_y != 0 && size >= &CharacterSize::Medium {
                let adjacent_x_idx = xy_idx(pos.x + delta_x, pos.y);
                let adjacent_y_idx = xy_idx(pos.x, pos.y + delta_y);
                if !map.tiles[adjacent_x_idx].walkable && !map.tiles[adjacent_y_idx].walkable {
                    can_move_to_dest = false;
                }
            }

            if map.tiles[dest_idx].walkable && can_move_to_dest {
                intent = PlayerIntent::Move;
            }
        }
        intent
    };
    // All read-only borrows are dropped here.

    // --- Phase 2: Write actions --- //
    match intent {
        PlayerIntent::OpenDoor(idx) => {
            try_open_door(idx, ecs);
        }
        PlayerIntent::Move => {
            let mut positions = ecs.write_storage::<Position>();
            let mut players = ecs.write_storage::<Player>();
            let mut viewsheds = ecs.write_storage::<Viewshed>();
            for (_player, pos, viewshed) in (&mut players, &mut positions, &mut viewsheds).join() {
                pos.x = min(79, max(0, pos.x + delta_x));
                pos.y = min(49, max(0, pos.y + delta_y));

                // When the player moves, we mark their viewshed as 'dirty' to trigger a recalculation.
                viewshed.dirty = true;
            }
        }
        PlayerIntent::DoNothing => {}
    }
}

/// Opens a door and updates its properties on the map.
fn try_open_door(idx: usize, ecs: &mut World) {
    // Fetch the whole Map resource to modify its tiles.
    let mut map = ecs.write_resource::<map::Map>();
    if let Some(door_state) = map.tiles[idx].door_state {
        match door_state {
            map::DoorState::Closed => {
                // Change the tile's properties to represent an open door.
                map.tiles[idx].door_state = Some(map::DoorState::Open);
                map.tiles[idx].glyph = rltk::to_cp437('/'); // Open door glyph
                map.tiles[idx].walkable = true;
                map.tiles[idx].transparent = true;
                map.tiles[idx].provides_cover = false;
            }
            map::DoorState::Locked => {
                // For now, you can't open locked doors.
                // We could add a message to the player here later.
            }
            map::DoorState::Open => {}
        }
    }
}

// KEYMAPPING ---------------------------------------------------------------
// modifichiamo la funzione di input per fare un match con le azioni del player
// in base al tasto passato al ctx.key (contesto key di Rltk)
// se trova l'azione restituisce Some altrimenti None
// Some() e None sono varianti di Option
fn player_input(ctx: &mut Rltk) -> Option<PlayerAction> {
    // Player movement
    match ctx.key {
        None => None, // Nothing happened
        Some(key) => match key {
            VirtualKeyCode::A | VirtualKeyCode::Left | VirtualKeyCode::Numpad4=> Some(PlayerAction::Move { dx: -1, dy: 0 }),
            VirtualKeyCode::D | VirtualKeyCode::Right | VirtualKeyCode::Numpad6 => Some(PlayerAction::Move { dx: 1, dy: 0 }),
            VirtualKeyCode::W | VirtualKeyCode::Up | VirtualKeyCode::Numpad8 => Some(PlayerAction::Move { dx: 0, dy: -1 }),
            VirtualKeyCode::X | VirtualKeyCode::Down | VirtualKeyCode::Numpad2 => Some(PlayerAction::Move { dx: 0, dy: 1 }),
            VirtualKeyCode::Q | VirtualKeyCode::Numpad7 => Some(PlayerAction::Move { dx: -1, dy: -1 }),
            VirtualKeyCode::E | VirtualKeyCode::Numpad9 => Some(PlayerAction::Move { dx: 1, dy: -1 }),
            VirtualKeyCode::Z | VirtualKeyCode::Numpad1 => Some(PlayerAction::Move { dx: -1, dy: 1 }),
            VirtualKeyCode::C | VirtualKeyCode::Numpad3 => Some(PlayerAction::Move { dx: 1, dy: 1 }),
            _ => None, // se non trova nulla restituisce None
        },
    }
}

impl GameState for State {
    // WHAT WILL BE DONE at each frame of the GAMELOOP
    fn tick(&mut self, ctx : &mut Rltk) {
        // ora con il movimento ha senso pulire il buffer della console
        ctx.cls();
        // disegniamo la mappa in un blocco separato per rilasciare il borrow di ecs
        // che avviene a causa di self.ecs.fetch() (ovvero durante l'accesso al world come risorsa)
        // in questo caso non c'è bisogno di usare il borrow perché non si modifica il mondo
        // ma solo si accede ad esso
        {
            draw_map(&self.ecs, ctx);
        }

        // INPUTS -------------------------------------------
        let player_action = player_input(ctx);
        if let Some(action) = player_action {
            match action {
                // se trova l'azione Move esegue try_move_player
                PlayerAction::Move { dx, dy } => {
                    // passiamo gli spostamenti assegnati al tasto e il mondo
                    try_move_player(dx, dy, &mut self.ecs);
                }
                // aggiungeremo altre azioni qui in futuro
            }
        } // se trova None non fa nulla

        // run ECS systems
        self.run_systems();
        // ECS Entities rendering pipeline
        let positions = self.ecs.read_storage::<Position>();
        let renderables = self.ecs.read_storage::<Renderable>();

        for (pos, render) in (&positions, &renderables).join() {
            ctx.set(pos.x, pos.y, render.fg, render.bg, render.glyph);
        }
    }
}

// ECS Systems pipeline
struct LeftWalker {}
impl<'a> System<'a> for LeftWalker {
    type SystemData = (ReadStorage<'a, LeftMover>, 
                        WriteStorage<'a, Position>);
// lefty è l'alias di riferimento alla readstorage su LeftMover
// pos è l'alias di riferimento alla writestorage (per questo è mut) su Position
    fn run(&mut self, (lefty, mut pos) : Self::SystemData) {
        //join per verificare che l'entità abbia entrambi i componenti
        for (_lefty,pos) in (&lefty, &mut pos).join() {
            // purtroppo il % in rust non è smart come in python e non permette overflow
            // per questo motivo dobbiamo usare la formula (pos.x - 1 + 80) % 80
            // per evitare overflow verso sinistra
            pos.x = (pos.x - 1 + 80) % 80;
        }
    }
}
// ECS Systems execution pipeline
impl State {
    fn run_systems(&mut self) {
        let mut vis = VisibilitySystem{};
        vis.run_now(&self.ecs);
        let mut lw = LeftWalker{};
        lw.run_now(&self.ecs);
        self.ecs.maintain();
    }
}

fn main() -> rltk::BError {
    // STARTUP ----------------------------------------------
    use rltk::RltkBuilder;
    let context = RltkBuilder::simple80x50()
        .with_title("Roguelike Tutorial")
        .build()?;
    let mut gs = State {
        ecs: World::new()
    };
    // ECS Components registration
    gs.ecs.register::<Position>();
    gs.ecs.register::<Renderable>();
    gs.ecs.register::<LeftMover>(); // tag component è comunque da registrare
    gs.ecs.register::<Player>();
    gs.ecs.register::<CanMove>();
    gs.ecs.register::<CharacterSize>();
    gs.ecs.register::<Viewshed>();
    
    // inseriamo la mappa come risorsa, quindi globalmente accessibile nel mondo ecs
        // --- MAP CREATION ---
    // Here, we decide which biome to generate.
    // We can easily switch `Forest` to `Volcano`, `Building`, or `SnowyMountains`
    // to completely change the generated world.
    let biome = Building{};
    // we can choose between wilderness_map and dungeon_map creators
    let (map, rooms) = wilderness_map(&biome);
    gs.ecs.insert(map);
    let (player_x, player_y) = if rooms.is_empty() {
        (40, 25) // Default position for wilderness maps
    } else {
        rooms[0].center() // Position for dungeon maps
    };

    // ECS Entities creation pipeline
    gs.ecs
        .create_entity()
        .with(Position { x: player_x, y: player_y })
        .with(Renderable {
        glyph: rltk::to_cp437('@'),
        fg: RGB::named(rltk::YELLOW),
        bg: RGB::named(rltk::BLACK),
    })
    .with(Player{}) //identifica il player
    .with(CanMove{}) // permette al player di muovere
    .with(CharacterSize::Medium) // definisce la taglia del player
    // The player's viewshed is initially dirty so it's calculated on the first turn.
    .with(Viewshed { visible_tiles: Vec::new(), range: 8, dirty: true }) // definisce il campo visivo del player
    .build();

    // Togliendo la creazione dei nemici, il sistema LeftWalker non ha più nulla da fare, 
    // quindi non fa nulla, anche senza cancellarlo
    // for i in 0..10 {
    //     gs.ecs
    //     .create_entity()
    //     .with(Position { x: i * 7, y: 20 })
    //     .with(Renderable {
    //         glyph: rltk::to_cp437('☺'),
    //         fg: RGB::named(rltk::RED),
    //         bg: RGB::named(rltk::BLACK),
    //     })
    //     .with(LeftMover{}) // tag component si aggiunge con un semplice .with()
    //     .build();
    // }

    // GAMELOOP ---------------------------------------------
    rltk::main_loop(context, gs)
}