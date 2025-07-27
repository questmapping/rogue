use rltk::{RandomNumberGenerator, RGB, to_cp437, Algorithm2D, BaseMap, Point, Rltk};
use specs::prelude::*;
use std::cmp::{max, min};

use crate::rect::Rect;

// Struttura che ci serve per la memoria della mappa (per il campo visivo: cosa ho già visto e cosa non ho ancora visto?)
#[derive(Default)]
pub struct Map {
    pub tiles : Vec<Tile>,
    pub rooms : Vec<Rect>,
    pub width : i32,
    pub height : i32,
    pub revealed_tiles : Vec<bool>
}

// RLTK traits per il bridge con le mappe costruite alla nostra maniera
impl Algorithm2D for Map {
    fn dimensions(&self) -> Point {
        Point::new(self.width, self.height)
    }
}
// RLTK traits per il bridge con le mappe costruite alla nostra maniera
impl BaseMap for Map {
    fn is_opaque(&self, idx:usize) -> bool {
        // abbiamo già la proprietà transparent in Tile, quindi non dobbiamo fare altro che restituirla invertita per opaque
        !self.tiles[idx as usize].transparent
    }
}

// --- Core Tile and Biome Structures ---
// This section defines the fundamental building blocks of our world generation.
// The goal is to separate a tile's properties (like behavior and appearance)
// from the map generation algorithm. This allows us to create diverse environments
// without rewriting the core logic for movement, field of view, etc.

/// Represents a single tile on the map.
/// Instead of a simple enum, this is a struct containing all the information
/// needed to interact with and display the tile.
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct Tile {
    // --- Visuals (Large Types) ---
    pub fg: RGB,                   // 12 bytes
    pub bg: RGB,                   // 12 bytes

    // --- Tile-specific State (Medium Types) ---
    pub trap_dc: Option<i32>,      // 8 bytes

    // --- Gameplay Effects (Medium Types) ---
    pub direct_damage: i32,        // 4 bytes
    pub slipperiness: i32,         // 4 bytes

    // --- Visuals (Small Types) ---
    pub glyph: rltk::FontCharType, // 2 bytes

    // --- State and Properties (Smallest Types) ---
    pub door_state: Option<DoorState>, // 1 byte (with niche optimization)
    pub status_effect: Option<StatusEffect>, // 1 byte (with niche optimization)
    pub walkable: bool,                // 1 byte
    pub transparent: bool,             // 1 byte
    pub provides_cover: bool,          // 1 byte
}

/// Enum for status effects that a tile can apply.
/// This can be expanded with more effects like Poisoned, Slowed, etc.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StatusEffect {
    Burning,
    Entangled,
}

/// Enum for the state of a door. This allows us to handle doors that can be
/// opened, closed, or even locked, requiring different interactions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum DoorState {
    Open,
    Closed,
    Locked,
}

/// The Biome Trait: A contract for all biomes.
/// Any struct that implements this trait can be used by the map generator.
/// This enforces consistency, ensuring that every biome provides the essential tiles.
#[allow(dead_code)]
pub trait Biome {
    // --- Required Tiles ---
    // These methods *must* be implemented for a biome to be valid.
    fn get_floor(&self) -> Tile;
    fn get_wall(&self) -> Tile;

    // --- Optional Tiles ---
    // These use `Option` to indicate that a biome might not have this tile type.
    // The map generator can then decide how to handle its absence.
    fn get_water(&self) -> Option<Tile> { None }
    fn get_trap(&self) -> Option<Tile> { None }
    fn get_stairs(&self) -> Option<Tile> { None }
    fn get_door(&self) -> Option<Tile> { None }
    fn get_locked_door(&self) -> Option<Tile> { None }
}


// --- Biome Implementations ---
// Here we define the specific biomes for our game.

// 1. The Building Biome
// A classic indoor environment with simple walls and floors.
pub struct Building;
impl Biome for Building {
    fn get_floor(&self) -> Tile {
        Tile {
            walkable: true, transparent: true, provides_cover: false,
            glyph: to_cp437('.'), // Ensuring this is a period for less noise
            fg: RGB::named(rltk::DARK_GRAY),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 0, door_state: None, trap_dc: None,
        }
    }

    fn get_wall(&self) -> Tile {
        Tile {
            walkable: false, transparent: false, provides_cover: true,
            glyph: to_cp437('#'),
            fg: RGB::named(rltk::WHITE),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 0, door_state: None, trap_dc: None,
        }
    }

    fn get_door(&self) -> Option<Tile> {
        Some(Tile {
            walkable: false, transparent: false, // A closed door blocks movement and sight.
            provides_cover: true,
            glyph: to_cp437('+'), // Closed door glyph
            fg: RGB::named(rltk::CHOCOLATE),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 0,
            door_state: Some(DoorState::Closed), trap_dc: None,
        })
    }

    fn get_locked_door(&self) -> Option<Tile> {
        Some(Tile {
            walkable: false, transparent: false, provides_cover: true,
            glyph: to_cp437('+'), // Same glyph, but maybe a different color later
            fg: RGB::named(rltk::RED), // Locked doors are red
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 0,
            door_state: Some(DoorState::Locked), trap_dc: None,
        })
    }
}

// 2. The Forest Biome
// An outdoor environment with trees, grass, and water.
pub struct Forest;
impl Biome for Forest {
    fn get_floor(&self) -> Tile { // Grass
        Tile {
            walkable: true, transparent: true, provides_cover: false,
            glyph: to_cp437('.'), // Changed from a quote to a period for less noise
            fg: RGB::named(rltk::GREEN),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 0, door_state: None, trap_dc: None,
        }
    }

    fn get_wall(&self) -> Tile { // Trees
        Tile {
            walkable: false, transparent: false, provides_cover: true,
            glyph: to_cp437('♣'),
            fg: RGB::named(rltk::BROWN1),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 0, door_state: None, trap_dc: None,
            // Later, we could add a component to trees to make them climbable.
        }
    }

    fn get_water(&self) -> Option<Tile> { // River/Lake
        Some(Tile {
            walkable: false, transparent: true, provides_cover: false,
            glyph: to_cp437('~'),
            fg: RGB::named(rltk::BLUE),
            bg: RGB::named(rltk::DARK_BLUE),
            direct_damage: 5, // Drowning damage
            status_effect: None, slipperiness: 0, door_state: None, trap_dc: None,
        })
    }

    fn get_trap(&self) -> Option<Tile> { // Entangling Vines
        Some(Tile {
            walkable: true, transparent: true, provides_cover: false,
            glyph: to_cp437(';'),
            fg: RGB::named(rltk::DARK_GREEN),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 1, // Spike damage
            status_effect: Some(StatusEffect::Entangled), slipperiness: 0, door_state: None, trap_dc: Some(15), // DC 15 to spot this trap
        })
    }
}

// 3. The Volcano Biome
// A dangerous environment with lava and obsidian.
pub struct Volcano;
impl Biome for Volcano {
    fn get_floor(&self) -> Tile { // Ash-covered ground
        Tile {
            walkable: true, transparent: true, provides_cover: false,
            glyph: to_cp437('▒'),
            fg: RGB::named(rltk::DARK_GRAY),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 0, door_state: None, trap_dc: None,
        }
    }

    fn get_wall(&self) -> Tile { // Obsidian shards
        Tile {
            walkable: false, transparent: false, provides_cover: true,
            glyph: to_cp437('▲'),
            fg: RGB::named(rltk::PURPLE),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 0, door_state: None, trap_dc: None,
        }
    }

    fn get_water(&self) -> Option<Tile> { // Lava
        Some(Tile {
            walkable: false, transparent: true, provides_cover: false,
            glyph: to_cp437('~'),
            fg: RGB::named(rltk::ORANGE),
            bg: RGB::named(rltk::RED),
            direct_damage: 10,
            status_effect: Some(StatusEffect::Burning), slipperiness: 0, door_state: None, trap_dc: None,
        })
    }
}

// 4. The Snowy Mountains Biome
// A cold, slippery environment.
pub struct SnowyMountains;
impl Biome for SnowyMountains {
    fn get_floor(&self) -> Tile { // Snow
        Tile {
            walkable: true, transparent: true, provides_cover: false,
            glyph: to_cp437(' '),
            fg: RGB::named(rltk::WHITE),
            bg: RGB::named(rltk::LIGHT_GRAY),
            direct_damage: 0, status_effect: None, slipperiness: 2, door_state: None, trap_dc: None,
        }
    }

    fn get_wall(&self) -> Tile { // Icy rock
        Tile {
            walkable: false, transparent: false, provides_cover: true,
            glyph: to_cp437('▲'),
            fg: RGB::named(rltk::LIGHT_CYAN),
            bg: RGB::named(rltk::BLACK),
            direct_damage: 0, status_effect: None, slipperiness: 1, door_state: None, trap_dc: None,
        }
    }
}


// --- Map Generation ---

/// Calculates the array index from a 2D coordinate.
pub fn xy_idx(x: i32, y: i32) -> usize {
    (y as usize * 80) + x as usize
}

/// Creates a map for a given biome.
/// This function is now generic and works with any `&dyn Biome`.
/// It doesn't know what a "Forest" or "Volcano" is; it just asks the biome
/// for the appropriate tiles.
// MAP BUILDER - Wilderness
#[allow(dead_code)]
pub fn wilderness_map(biome: &dyn Biome) -> (Map, Vec<Rect>) {
    let mut map = Map{
        tiles : vec![biome.get_floor(); 80*50],
        rooms : Vec::new(),
        width : 80,
        height: 50,
        revealed_tiles : vec![false; 80*50] // inizializza tutti i valori a false (non visti) quando crea la mappa
    };
    
    let wall_tile = biome.get_wall();

    // Make the boundaries walls
    for x in 0..80 {
        map.tiles[xy_idx(x, 0)] = wall_tile;
        map.tiles[xy_idx(x, 49)] = wall_tile;
    }
    for y in 0..50 {
        map.tiles[xy_idx(0, y)] = wall_tile;
        map.tiles[xy_idx(79, y)] = wall_tile;
    }

    // Randomly place some walls
    let mut rng = RandomNumberGenerator::new();
    for _i in 0..400 {
        let x = rng.roll_dice(1, 79);
        let y = rng.roll_dice(1, 49);
        let idx = xy_idx(x, y);
        if idx != xy_idx(40, 25) { // Don't block the player's starting position
            // 20% chance of placing a door, if the biome supports it.
            let roll = rng.roll_dice(1, 100);
            if roll > 80 {
                if roll > 90 {
                    if let Some(locked_door) = biome.get_locked_door() {
                        map.tiles[idx] = locked_door;
                    } else {
                        map.tiles[idx] = wall_tile.clone();
                    }
                } else {
                    if let Some(door) = biome.get_door() {
                        map.tiles[idx] = door;
                    } else {
                        map.tiles[idx] = wall_tile.clone();
                    }
                }
            } else {
                map.tiles[idx] = wall_tile.clone();
            }
        }
    }

    // Optionally, place some biome-specific features like water or traps
    if let Some(water_tile) = biome.get_water() {
        for _i in 0..20 {
            let x = rng.roll_dice(1, 79);
            let y = rng.roll_dice(1, 49);
            let idx = xy_idx(x, y);
            map.tiles[idx] = water_tile;
        }
    }

    if let Some(trap_tile) = biome.get_trap() {
        for _i in 0..10 {
            let x = rng.roll_dice(1, 79);
            let y = rng.roll_dice(1, 49);
            let idx = xy_idx(x, y);
            map.tiles[idx] = trap_tile;
        }
    }

    (map, Vec::new()) // Return map and an empty list of rooms
}

// procedura di rendering della mappa
/// This function is now much simpler. It doesn't need to know anything about
/// different tile types. It just iterates through the map and uses the properties
/// (glyph, color) stored in each Tile struct.
pub fn draw_map(ecs: &World, ctx: &mut Rltk) {
    let map = ecs.fetch::<Map>();
    let mut rng = rltk::RandomNumberGenerator::new();

    let mut y = 0;
    let mut x = 0;
    for (idx, tile) in map.tiles.iter().enumerate() {
        // We only draw tiles that have been revealed
        if map.revealed_tiles[idx] {
            let glyph;
            let fg;

            // Check if the tile is currently visible
            if map.revealed_tiles[idx] {
                fg = tile.fg;
                // Check for hidden traps only if visible (THIS IS FOOD FOR THE VISIBILITY SYSTEM)
                if let Some(dc) = tile.trap_dc {
                    if rng.roll_dice(1, 20) < dc {
                        glyph = rltk::to_cp437('.'); // Failed to spot
                    } else {
                        glyph = tile.glyph; // Spotted
                    }
                } else {
                    glyph = tile.glyph; // No trap
                }
            } else {
                // If not visible but revealed, draw in grayscale
                fg = tile.fg.to_greyscale();
                glyph = tile.glyph;
            }
            ctx.set(x, y, fg, tile.bg, glyph);
        }

        // Move the coordinates
        //  The "move coordinates" block is responsible for converting the 1D index of the tile in the map.tiles vector into the correct 2D 
        // (x, y)
        // position on the screen for rendering.
        x += 1;
        if x >= map.width {
            x = 0;
            y += 1;
        }
    }
}

// MAP BUILDER - Dungeon
#[allow(dead_code)]
fn apply_room_to_map(room: &Rect, map: &mut [Tile], biome: &dyn Biome) {
    let floor = biome.get_floor();
    for y in room.y1 + 1..=room.y2 {
        for x in room.x1 + 1..=room.x2 {
            let idx = xy_idx(x, y);
            map[idx] = floor.clone();
        }
    }
}

#[allow(dead_code)]
fn apply_horizontal_tunnel(map: &mut [Tile], x1: i32, x2: i32, y: i32, biome: &dyn Biome) {
    let floor = biome.get_floor();
    for x in min(x1, x2)..=max(x1, x2) {
        let idx = xy_idx(x, y);
        if idx > 0 && idx < (80 * 50) {
            map[idx] = floor.clone();
        }
    }
}

#[allow(dead_code)]
fn apply_vertical_tunnel(map: &mut [Tile], y1: i32, y2: i32, x: i32, biome: &dyn Biome) {
    let floor = biome.get_floor();
    for y in min(y1, y2)..=max(y1, y2) {
        let idx = xy_idx(x, y);
        if idx > 0 && idx < (80*50) {
            map[idx] = floor.clone();
        }
    }
}

#[allow(dead_code)]
/// Calculates the coordinate for a tunnel to start or end, just outside a room's door.
/// This ensures that tunnels connect to the tile adjacent to the door,
/// rather than starting on the door tile itself, which would overwrite it.
#[allow(dead_code)]
fn get_exit_point(p: (i32, i32), room: &Rect) -> (i32, i32) {
    if p.0 == room.x1 { // West wall
        (p.0 - 1, p.1)
    } else if p.0 == room.x2 { // East wall
        (p.0 + 1, p.1)
    } else if p.1 == room.y1 { // North wall
        (p.0, p.1 - 1)
    } else { // South wall
        (p.0, p.1 + 1)
    }
}

/// Finds the best candidate tile on a room's perimeter to place a door.
/// The "best" candidate is the wall tile on the room's edge that is closest to
/// the line of the future corridor, which is estimated from the room's center.
/// It prioritizes cardinal directions (North, South, East, West) over corners.
#[allow(dead_code)]
fn find_door_candidate(center_x: i32, center_y: i32, room: &Rect) -> Option<(i32, i32)> {
    let mut candidates = Vec::new();
    // Check walls, preferring cardinal directions
    if center_x > room.x1 && center_x < room.x2 {
        candidates.push((center_x, room.y1)); // North
        candidates.push((center_x, room.y2)); // South
    }
    if center_y > room.y1 && center_y < room.y2 {
        candidates.push((room.x1, center_y)); // West
        candidates.push((room.x2, center_y)); // East
    }

    if candidates.is_empty() {
        // Fallback for corners or small rooms
        candidates.push((room.x1, room.y1));
        candidates.push((room.x1, room.y2));
        candidates.push((room.x2, room.y1));
        candidates.push((room.x2, room.y2));
    }

    // Find the candidate closest to the room center
    candidates.sort_by(|a, b| {
        let dist_a = (a.0 - center_x).pow(2) + (a.1 - center_y).pow(2);
        let dist_b = (b.0 - center_x).pow(2) + (b.1 - center_y).pow(2);
        dist_a.cmp(&dist_b)
    });

    candidates.first().cloned()
}

/// Creates a dungeon-style map with rooms and corridors.
/// The algorithm is as follows:
/// 1. Fill the entire map with solid wall tiles.
/// 2. Generate a list of random, non-overlapping rooms (Rects).
/// 3. "Carve" out each room by replacing wall tiles with floor tiles.
/// 4. For each pair of consecutive rooms:
///    a. Find the best candidate point on the edge of each room for a door.
///    b. Calculate the "exit point" for the tunnel, which is the tile just outside the door.
///    c. Carve L-shaped tunnels between the two exit points.
/// 5. Place door tiles at all the candidate points stored in step 4a.
#[allow(dead_code)]
pub fn dungeon_map(biome: &dyn Biome) -> (Map, Vec<Rect>) {
    let wall = biome.get_wall();
    let mut map = Map{
        tiles : vec![wall; 80*50],
        rooms : Vec::new(),
        width : 80,
        height: 50,
        revealed_tiles : vec![false; 80*50] // inizializza tutti i valori a false (non visti) quando crea la mappa
    };

    let mut rooms: Vec<Rect> = Vec::new();
    const MAX_ROOMS: i32 = 30;
    const MIN_SIZE: i32 = 6;
    const MAX_SIZE: i32 = 10;

    let mut rng = RandomNumberGenerator::new();

    for _ in 0..MAX_ROOMS {
        let w = rng.range(MIN_SIZE, MAX_SIZE);
        let h = rng.range(MIN_SIZE, MAX_SIZE);
        let x = rng.roll_dice(1, 80 - w - 1) - 1;
        let y = rng.roll_dice(1, 50 - h - 1) - 1;
        let new_room = Rect::new(x, y, w, h);

        // To prevent rooms from spilling over the edge of the map or overlapping, we perform checks.
        let mut ok = true;
        // Check for intersections with existing rooms.
        for other_room in rooms.iter() {
            if new_room.intersect(other_room) { ok = false }
        }
        // Check if the room is within the map boundaries.
        if new_room.x1 < 1 || new_room.x2 > 78 || new_room.y1 < 1 || new_room.y2 > 48 { 
            ok = false;
        }

        if ok {
            rooms.push(new_room);
        }
    }

    for room in rooms.iter() {
        apply_room_to_map(room, &mut map.tiles, biome);
    }

    let mut doors = Vec::new();
    // Iterate through the rooms to create corridors connecting them.
    for i in 1..rooms.len() {
        // Get the center points of the current and previous rooms.
        let (new_x, new_y) = rooms[i].center();
        let (prev_x, prev_y) = rooms[i-1].center();

        // Find the best points on the room edges to place doors.
        let p1_door_candidate = find_door_candidate(prev_x, prev_y, &rooms[i-1]);
        let p2_door_candidate = find_door_candidate(new_x, new_y, &rooms[i]);

        if let (Some(p1), Some(p2)) = (p1_door_candidate, p2_door_candidate) {
            // Store the door locations for later placement.
            doors.push(p1);
            doors.push(p2);

            // Get the tunnel exit points, which are adjacent to the doors.
            let c1 = get_exit_point(p1, &rooms[i-1]);
            let c2 = get_exit_point(p2, &rooms[i]);

            // Randomly decide whether to carve the horizontal or vertical tunnel first.
            if rng.range(0,2) == 1 {
                apply_horizontal_tunnel(&mut map.tiles, c1.0, c2.0, c1.1, biome);
                apply_vertical_tunnel(&mut map.tiles, c1.1, c2.1, c2.0, biome);
            } else {
                apply_vertical_tunnel(&mut map.tiles, c1.1, c2.1, c1.0, biome);
                apply_horizontal_tunnel(&mut map.tiles, c1.0, c2.0, c2.1, biome);
            }
        }
    }

    // Finally, place the doors at all the candidate locations we stored.
    if let Some(door_tile) = biome.get_door() {
        for door_pos in doors {
            let idx = xy_idx(door_pos.0, door_pos.1);
            map.tiles[idx] = door_tile.clone();
        }
    }

    (map, rooms)
}
