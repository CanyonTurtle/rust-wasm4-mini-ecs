#[cfg(feature = "linked_list_allocator")]
mod alloc;

mod wasm4;
mod ecs;
mod rng;
use ecs::{Entity, GenerationalIndexAllocator, EntityMap};
use rng::Rng;
use wasm4::*;

use crate::ecs::{AllocatorEntry, IndexType};

// tune-able constant: how many entities we have.
pub const MAX_N_ENTITIES: usize = 100;


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
    other: Option<Entity>,
    countdown_msec: u32,
}

// List your components in this struct. Each entity has one of each (each entry is optional).
struct EntityComponents {
    kinematics: EntityMap<Kinematics>,
    physics: EntityMap<PhysicsComponent>,
    raining_smiley: EntityMap<RainingSmileyComponent>,
}

// All other state that doesn't fit into a component goes here.
struct GameResources {
    // hello_msg: String,
    rng: Rng,
    gravity_overall_mult: f32,
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
                let x = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 10.0 + 75.0;
                let y = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 10.0 + 75.0;
                let vx = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 0.5 - 0.25;
                let vy = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 0.5 - 0.25; // 5.0 - 2.5;
                let collision_elasticity = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 0.35 + 0.4;
                let gravity_mult = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * 0.002;
                let countdown_msec = 60*7 + (gs.resources.rng.next() % 60*7) as u32;

                if let Err(_) = gs.entity_components.kinematics.set(&gs.entities.last().unwrap(), Kinematics{x , y, vx, vy}) {
                    trace("Pos component set fail")

                }
                if let Err(_) = gs.entity_components.physics.set(&gs.entities.last().unwrap(), PhysicsComponent{gravity_mult, w: 8.0, h: 8.0, collision_elasticity}) {
                    trace("Phys component set fail")
                }
                if let Err(_) = gs.entity_components.raining_smiley.set(&gs.entities.last().unwrap(), RainingSmileyComponent{other: None, countdown_msec}) {
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

                #[cfg(feature = "linked_list_allocator")]
                alloc::init_heap();

                // Initialize / allocate entities and components.
                // ORDER MATTERS. Reserve memory in order from largest to smallest components, so the layout is fit optimally.
                let mut pos_comp_items = Vec::with_capacity(MAX_N_ENTITIES);
                let mut phys_comp_items = Vec::with_capacity(MAX_N_ENTITIES);
                let mut raining_smiley_items = Vec::with_capacity(MAX_N_ENTITIES);

                let entities = Vec::with_capacity(MAX_N_ENTITIES);

                let mut entries = Vec::with_capacity(MAX_N_ENTITIES);
                let mut free = Vec::with_capacity(MAX_N_ENTITIES);

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
                    entities,
                    resources: GameResources{
                        // hello_msg: "Hello from Rust!".to_string(),
                        rng: Rng::new(),
                        gravity_overall_mult: 1.0,
                    }
                });

                // Example usage on startup: allocate an entity and give it a position.
                #[allow(static_mut_ref)]
                if let Some(gs) = &mut STATIC_ECS_DATA {
                    for _ in 0..MAX_N_ENTITIES {
                        add_smiley_ball(gs);
                    }
                }

            },
            _ => {}
        }

        // Once we've intiailized the ECS, a mut ref is available to it outside our unsafe block.
        #[allow(static_mut_ref)]
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
            if let Some(p1) = game_state.entity_components.kinematics.get(&player) {
                // trace("got comp");
                if let Some(sm) = game_state.entity_components.raining_smiley.get(&player) {
                    unsafe { *DRAW_COLORS = 0x0002 }
                    if let Some(id2) = sm.other {
                        if let Some(p2) = game_state.entity_components.kinematics.get(&id2) {
                            unsafe { *DRAW_COLORS = 0x0003 }
                            line(p1.x as i32 + 4, p1.y as i32 + 4, p2.x as i32 + 4, p2.y as i32 + 4);
                        } 
                    }
                    blit(&SMILEY, p1.x as i32, p1.y as i32, 8, 8, BLIT_1BPP);
                }
                
                
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
        let mut to_rm = vec![];
        for (i, e) in &mut ecs.entities.iter_mut().enumerate() {
            // trace("trying player");
            let mut k2p = None;

            if let Some(sm) = ecs.entity_components.raining_smiley.get(&e) {
                if let Some(o) = sm.other {
                    if let Some(k2) = ecs.entity_components.kinematics.get(&o) {
                        k2p = Some((k2.x, k2.y));
                    }
                }
            }
            if let Some(pos) = ecs.entity_components.kinematics.get_mut(&e) {
                // trace("got comp");
                if let Some(phys) = ecs.entity_components.physics.get(&e) {
                    // trace("got comp");
                    // pos.vy += phys.gravity_mult*ecs.resources.gravity_overall_mult;
                    
                    match k2p {
                        Some(k2p) => {
                            let del_x = k2p.0 - pos.x;
                            let del_y = k2p.1 - pos.y;
                            let denom = (del_x.powi(2) + del_y.powi(2)).sqrt();
                            if denom > 0.0 {
                                pos.vy += del_y / denom * 0.01;
                                pos.vx += del_x / denom * 0.01;
                            }
                            if ((pos.x - 80.0).powi(2) + (pos.y - 80.0).powi(2)) >= 80f32.powi(2) {
                                match ecs.entity_allocator.deallocate(&e) {
                                    Ok(_) => {
                                        to_rm.push(i);
                                    },
                                    Err(_) => {}
                                }
                            }
                        }
                        None => {
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
                        },
                    }

                    

                    
                }
            }
        }
        for i in to_rm {
            ecs.entities.remove(i);
        }
    }

    fn rain_smiley_system(ecs: &mut ECS) {
        let mut break_links = vec![];
        for i in 0..ecs.entities.len() {
            let e = &mut ecs.entities[i];
            // trace("trying player");
            if let Some(s) = ecs.entity_components.raining_smiley.get_mut(e) {
                // trace("got comp");
                // s.countdown_msec -= 1;
                // if s.countdown_msec <= 0 {
                //     // break link
                //     if let Some(oe) = s.other {
                //         break_links.push(oe);
                //     }
                //     if let Err(_) = ecs.entity_allocator.deallocate(e) {
                //         // trace(format!["Deallocate err: {:?}", e])
                //     } else {
                        
                //         ecs.entities.remove(i);
                        
                //         // trace("Deallocated");

                //         add_smiley_ball(ecs);
                //     }
                    
                // }   
                
            }
            
        }
        for break_link in break_links {
            if let Some(sm) = ecs.entity_components.raining_smiley.get_mut(&break_link) {
                sm.other = None;
            }
        }
    }

    fn link_smiley_system(ecs: &mut ECS) {
        let mut links = vec![];
        let mut linked_entities_this_pass = vec![];
        for i in 0..ecs.entities.len() {
            let e1 = &ecs.entities[i];
            for j in (i+1)..ecs.entities.len() {
                let e2 = &ecs.entities[j];
                if let Some(rs1) = ecs.entity_components.raining_smiley.get(e1) {
                    if let Some(rs2) = ecs.entity_components.raining_smiley.get(e2) {
                        if let Some(k1) = ecs.entity_components.kinematics.get(e1) {
                            if let Some(k2) = ecs.entity_components.kinematics.get(e2) {
                                if (k1.x - k2.x).powi(2) + (k1.y - k2.y).powi(2) < (8.0f32).powi(2) {
                                    if let None = rs1.other {
                                        if let None = rs2.other {
                                            if !linked_entities_this_pass.contains(e1) && !linked_entities_this_pass.contains(e2) {
                                                linked_entities_this_pass.push(*e1);
                                                linked_entities_this_pass.push(*e2);
                                                links.push((e1, e2));
                                            }
                                            
                                        }
                                    }
                                }
                            } 
                        }
                    } 
                }   
            }      
        }

        for (e1, e2) in links {
            if let Some(rsm1) = ecs.entity_components.raining_smiley.get_mut(e1) {
                rsm1.other = Some(*e2);
            }
            if let Some(rsm2) = ecs.entity_components.raining_smiley.get_mut(e2) {
                rsm2.other = Some(*e1);
            }
        }
        
    }

    unsafe { *DRAW_COLORS = 2 }

    // text(&ecs.resources.hello_msg, 10, 10);

    let gamepad = unsafe { *GAMEPAD1 };
    ecs.resources.gravity_overall_mult = match gamepad != 0 {
        true => 10.0,
        false => 0.5
    };
    

    // Running the game is just playing forward all the systems!!
    update_physics_system(&mut ecs);
    update_kinematics_system(&mut ecs);
    rain_smiley_system(&mut ecs);

    draw_entities_system(&ecs);
    link_smiley_system(&mut ecs);

    unsafe { *DRAW_COLORS = 0x0004 }
    text("rust-wasm4-mini-ecs", 3, 150);
}
