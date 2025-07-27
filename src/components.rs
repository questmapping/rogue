use rltk::{RGB, FontCharType};
use specs::prelude::*;
use specs_derive::Component;


// COMPONENTS ----------------------------------------------------------------


// tag component per la taglia del personaggio
// implementiamo PartialEq e PartialOrd per permettere la comparazione tra i componenti
// in particolare PartialOrd permette di usare il < e >, perchè li ritiene ordinati dal più piccolo al più grande
#[allow(dead_code)]
#[derive(Component, Debug, PartialEq, PartialOrd, Clone, Copy)]
pub enum CharacterSize {
    Tiny,
    Small,
    Medium,
    Large,
    Huge,
}
#[derive(Component)]
pub struct Position { pub x: i32, pub y: i32 }

#[derive(Component)]
pub struct Renderable {
    pub fg: RGB,
    pub bg: RGB,
    pub glyph: FontCharType,
}

#[derive(Component)]
pub struct LeftMover{}

#[derive(Component)]
pub struct CanMove{}

#[derive(Component)]
pub struct Player {}

// componente per il Field of View
#[derive(Component)]
pub struct Viewshed {
    pub visible_tiles : Vec<rltk::Point>,
    pub range : i32,
    pub dirty : bool // Flag to indicate if the viewshed needs to be recalculated.
}