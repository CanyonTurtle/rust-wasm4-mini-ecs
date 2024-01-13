#[cfg(feature = "buddy-alloc")]
mod alloc;
mod wasm4;
mod ecs;
mod rng;
use ecs::{Entity, GenerationalIndexAllocator, EntityMap};
use rng::Rng;
use wasm4::*;

use crate::ecs::{AllocatorEntry, IndexType};

// tune-able constant: how many entities we have.
pub const MAX_N_ENTITIES: usize = 250;


// Example ECS component
struct Kinematics{
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

// Another example component in the ECS
struct PhysicsComponent {
    gravity_mult: f32,
    w: f32,
    h: f32,
    collision_elasticity: f32
}

// An empty component just to tag something as being involved in a given system.
struct RainingSmileyComponent {
    countdown_msec: u64,
}

// List your components in this struct. Each entity has one of each (each entry is optional).
struct EntityComponents {
    kinematics: EntityMap<Kinematics>,
    physics: EntityMap<PhysicsComponent>,
    raining_smiley: EntityMap<RainingSmileyComponent>,
}

// All other state that doesn't fit into a component goes here.
struct GameResources {
    hello_msg: String,
    rng: Rng,
}

// Here's the global state of the game, in our ECS object!
struct ECS {
    entity_allocator: GenerationalIndexAllocator,
    entity_components: EntityComponents,
    resources: GameResources,
    entities: Vec<Entity>,
}

// The ECS is stored in static memory here.
static mut STATIC_ECS_DATA: Option<ECS> = None;

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

    // This isn't a "system" per-se, this is just a function that adds a ball entity.
    fn add_smiley_ball(gs: &mut ECS) {
        match gs.entity_allocator.allocate() {
            Ok(index) => {
                gs.entities.push(index);
                let x = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 120.0;
                let y = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 50.0;
                let vx = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 5.0 - 2.5;
                let vy = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 5.0 - 2.5;
                let collision_elasticity = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 0.25 + 0.6;
                let gravity_mult = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 0.02 + 0.18;
                let countdown_msec = 3 * 60 + ((gs.resources.rng.next() % (3 * 60)));

                if let Err(_) = gs.entity_components.kinematics.set(&gs.entities.last().unwrap(), Kinematics{x , y, vx, vy}) {
                    trace("Pos component set fail")

                }
                if let Err(_) = gs.entity_components.physics.set(&gs.entities.last().unwrap(), PhysicsComponent{gravity_mult, w: 8.0, h: 8.0, collision_elasticity}) {
                    trace("Phys component set fail")
                }
                if let Err(_) = gs.entity_components.raining_smiley.set(&gs.entities.last().unwrap(), RainingSmileyComponent{countdown_msec}) {
                    trace("Phys component set fail")
                }
            },
            Err(_) => {
                trace("allocate fail");
            },
        }
    }

    // trace("begin");
    let mut ecs: &mut ECS;
    unsafe {
        match STATIC_ECS_DATA {
            None => {

                // Initialize / allocate entities and components.
                let mut entries = Vec::new();
                let mut free = Vec::new();
                let mut pos_comp_items = Vec::new();
                let mut phys_comp_items = Vec::new();
                let mut raining_smiley_items = Vec::new();
                // The ECS has a max size limit. We allocate everything upfront.
                for i in 0..MAX_N_ENTITIES as IndexType {
                    entries.push(AllocatorEntry {
                        is_live: false,
                        generation: 0,
                    });
                    free.push(i);
                    pos_comp_items.push(None);
                    phys_comp_items.push(None);
                    raining_smiley_items.push(None);
                }

                // Initialization for the ECS happens here.
                STATIC_ECS_DATA = Some(ECS{
                    entity_allocator: GenerationalIndexAllocator{
                        entries,
                        free,
                        generation_counter: 0
                    },
                    entity_components: EntityComponents{
                        kinematics: EntityMap{0: pos_comp_items},
                        physics: EntityMap{0: phys_comp_items},
                        raining_smiley: EntityMap{0: raining_smiley_items},
                    },
                    entities: Vec::new(),
                    resources: GameResources{
                        hello_msg: "Hello from Rust!".to_string(),
                        rng: Rng::new()
                    }
                });

                // Example usage on startup: allocate an entity and give it a position.
                if let Some(gs) = &mut STATIC_ECS_DATA {
                    for _ in 0..MAX_N_ENTITIES {
                        add_smiley_ball(gs);
                    }
                }

            },
            _ => {}
        }

        // Once we've intiailized the ECS, a mut ref is available to it outside our unsafe block.
        match &mut STATIC_ECS_DATA {
            Some(gs) => {
                ecs = gs
            },
            _ => {
                trace("fail set game state");
                unreachable!();
            }
        }
    }

    // Example immutable-reference system: take in the ECS and compute something from it (e.g. rendering)
    fn draw_entities_system(game_state: &ECS) {
        for player in &game_state.entities {
            // trace("trying player");
            if let Some(pos) = game_state.entity_components.kinematics.get(&player) {
                // trace("got comp");
                blit(&SMILEY, pos.x as i32, pos.y as i32, 8, 8, BLIT_1BPP);
            }
        }
    }

    fn update_kinematics_system(ecs: &mut ECS) {
        for e in &mut ecs.entities {
            // trace("trying player");
            if let Some(pos) = ecs.entity_components.kinematics.get_mut(&e) {
                pos.x += pos.vx;
                pos.y += pos.vy;
            }
        }
    }

    // Example mutable-reference system: take in the ECS and compute something from it (e.g. rendering)
    fn update_physics_system(ecs: &mut ECS) {
        for e in &mut ecs.entities {
            // trace("trying player");
            if let Some(pos) = ecs.entity_components.kinematics.get_mut(&e) {
                // trace("got comp");
                if let Some(phys) = ecs.entity_components.physics.get(&e) {
                    // trace("got comp");
                    pos.vy += phys.gravity_mult;
                    
                    if pos.x + phys.w >= 160.0 {
                        pos.vx *= -phys.collision_elasticity;
                        pos.x = 160.0 - phys.w;
                    } else if pos.x + pos.vx < 0.0 {
                        pos.vx *= -phys.collision_elasticity;
                        pos.x = 0.0;
                    }
                    if pos.y + phys.h >= 160.0 {
                        pos.vy = pos.vy.abs() * -phys.collision_elasticity;
                        pos.y = 160.0 - phys.h;
                    } else if pos.y < 0.0 {
                        pos.y = 0.0;
                        pos.vy *= -phys.collision_elasticity;
                    }
                }
            }
        }
    }

    fn rain_smiley_system(ecs: &mut ECS) {
        for i in 0..ecs.entities.len() {
            let e = &mut ecs.entities[i];
            // trace("trying player");
            if let Some(s) = ecs.entity_components.raining_smiley.get_mut(e) {
                // trace("got comp");
                s.countdown_msec -= 1;
                if s.countdown_msec <= 0 {
                    if let Err(_) = ecs.entity_allocator.deallocate(e) {
                        // trace(format!["Deallocate err: {:?}", e])
                    } else {
                        ecs.entities.remove(i);
                        // trace("Deallocated");

                        add_smiley_ball(ecs);
                    }
                    
                }   
                
            }
           
        }
    }

    

    unsafe { *DRAW_COLORS = 2 }

    text(&ecs.resources.hello_msg, 10, 10);

    let gamepad = unsafe { *GAMEPAD1 };
    if gamepad & BUTTON_1 != 0 {
        unsafe { *DRAW_COLORS = 4 }
    }
    
    text("Press X to blink", 16, 90);

    // Running the game is just playing forward all the systems!!
    update_physics_system(&mut ecs);
    update_kinematics_system(&mut ecs);
    rain_smiley_system(&mut ecs);

    draw_entities_system(&ecs);
}
