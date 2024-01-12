#[cfg(feature = "buddy-alloc")]
mod alloc;
mod wasm4;
mod ecs;
use ecs::{Entity, GenerationalIndexAllocator, EntityMap, GenerationalIndexArray};
use wasm4::*;

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
    trace("begin");
    let game_state: &mut GameState;
    unsafe {
        match GAME_STATE {
            None => {
                trace("Game state is none");
                for _ in 0..MAX_N_ENTITIES
                GAME_STATE = Some(GameState{
                    entity_allocator: GenerationalIndexAllocator{
                        entries: Vec::new(Alloc),
                        free: vec![0, 1, 2, 3, 4],
                        generation_counter: 0
                    },
                    position_components: GenerationalIndexArray{
                        0: Vec::new()
                    },
                    players: Vec::new()
                });

                if let Some(gs) = &mut GAME_STATE {
                    match gs.entity_allocator.allocate() {
                        Ok(index) => {
                            gs.players.push(index);
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

    trace("got game state");
    // game_state.position_components.get(&game_state.players[0]);

    fn draw_players_system(game_state: &GameState) {
        for player in &game_state.players {
            trace("trying player");
            if let Some(pos) = game_state.position_components.get(&player) {
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
