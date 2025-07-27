use specs::prelude::*;
use super::{Viewshed, Position, Player};
use crate::map::{Map, xy_idx};
use rltk::{field_of_view, Point};

pub struct VisibilitySystem {}

impl<'a> System<'a> for VisibilitySystem {
    type SystemData = ( WriteExpect<'a, Map>,
                        Entities<'a>,
                        WriteStorage<'a, Viewshed>,
                        WriteStorage<'a, Position>,
                        ReadStorage<'a, Player>);

    fn run(&mut self, data : Self::SystemData) {
        let (mut map, entities, mut viewshed, pos, player) = data;

        for (ent,viewshed,pos) in (&entities, &mut viewshed, &pos).join() {
            // We only recalculate the field of view if the 'dirty' flag is set.
            // This is an optimization to avoid recalculating every frame.
            if viewshed.dirty {
                viewshed.visible_tiles.clear();
                viewshed.visible_tiles = field_of_view(Point::new(pos.x, pos.y), viewshed.range, &*map);
                viewshed.visible_tiles.retain(|p| p.x >= 0 && p.x < map.width && p.y >= 0 && p.y < map.height );

                // Once the viewshed is recalculated, we set the dirty flag to false.
                viewshed.dirty = false;

                // If this is the player, reveal what they can see
                let p : Option<&Player> = player.get(ent);
                if let Some(_p) = p {
                    for vis in viewshed.visible_tiles.iter() {
                        let idx = xy_idx(vis.x, vis.y);
                        map.revealed_tiles[idx] = true;
                    }
                }
            }
        }
    }
}