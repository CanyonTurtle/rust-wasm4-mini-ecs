#[cfg(feature = "buddy-alloc")]
mod alloc;
mod wasm4;
mod ecs;
use ecs::{Entity, GenerationalIndexAllocator, EntityMap, GenerationalIndexArray};
use wasm4::*;

use crate::ecs::{ArrayEntry, AllocatorEntry, MAX_N_ENTITIES};

// Example usage of ECS
struct PositionComponent{
    x: f32,
    y: f32
}

struct GameState {

    entity_allocator: GenerationalIndexAllocator,

    position_components: EntityMap<PositionComponent>,

    players: Vec<Entity>
}

static mut GAME_STATE: Option<GameState> = None;

#[rustfmt::skip]
const SMILEY: [u8; 8] = [
    0b11000011,
    0b10000001,
    0b00100100,
    0b00100100,
    0b00000000,
    0b00100100,
    0b10011001,
    0b11000011,
];

#[no_mangle]
fn update() {
    // trace("begin");
    let game_state: &mut GameState;
    unsafe {
        match GAME_STATE {
            None => {
                trace("Game state is none");
                let mut entries = Vec::new();
                let mut free = Vec::new();
                let mut pos_comp_items = Vec::new();
                for i in 0..MAX_N_ENTITIES {
                    entries.push(AllocatorEntry {
                        is_live: false,
                        generation: 0,
                    });
                    free.push(i);
                    pos_comp_items.push(None);
                }
                GAME_STATE = Some(GameState{
                    entity_allocator: GenerationalIndexAllocator{
                        entries,
                        free,
                        generation_counter: 0
                    },
                    position_components: GenerationalIndexArray{
                        0: pos_comp_items
                    },
                    players: Vec::new()
                });

                if let Some(gs) = &mut GAME_STATE {
                    match gs.entity_allocator.allocate() {
                        Ok(index) => {
                            gs.players.push(index);
                            match gs.position_components.set(&gs.players[0], PositionComponent{x: 10.0, y: 10.0}) {
                                Err(_) => {
                                    trace("Pos component set fail")
                                },
                                _ => {}
                            }
                        },
                        Err(_) => {
                            trace("allocate fail");
                        },
                    }
                }

            },
            _ => {}
        }
        match &mut GAME_STATE {
            Some(gs) => {
                game_state = gs
            },
            _ => {
                trace("fail set game state");
                unreachable!();
            }
        }
    }

    // trace("got game state");
    // game_state.position_components.get(&game_state.players[0]);

    fn draw_players_system(game_state: &GameState) {
        for player in &game_state.players {
            // trace("trying player");
            if let Some(pos) = game_state.position_components.get(&player) {
                trace("got comp");
                blit(&SMILEY, pos.x as i32, pos.y as i32, 8, 8, BLIT_1BPP);
            }
        }
    }

    draw_players_system(&game_state);

    unsafe { *DRAW_COLORS = 2 }
    text("Hello from Rust!", 10, 10);

    let gamepad = unsafe { *GAMEPAD1 };
    if gamepad & BUTTON_1 != 0 {
        unsafe { *DRAW_COLORS = 4 }
    }

    
    text("Press X to blink", 16, 90);
}
