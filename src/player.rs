use specs::prelude::*;
use specs_derive::Component;
use crate::components::Player;



// these are the actions that the player can take
#[derive(PartialEq, Copy, Clone)]
pub enum PlayerAction {
    Move { dx: i32, dy: i32 },
}

// these are the implied actions that the player wants to take when moving against an object
// will be useful also for auto attack when the player moves against an enemy
#[derive(PartialEq, Copy, Clone)]
pub enum PlayerIntent {
    Move,
    OpenDoor(usize),
    DoNothing,
}
