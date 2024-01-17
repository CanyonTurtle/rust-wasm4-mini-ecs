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
pub const MAX_N_ENTITIES: usize = 30;

pub const BALL_WIDTH: f32 = 8.0;
pub const BALL_HEIGHT: f32 = 8.0;

// Example ECS component
struct Kinematics{
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

// Another example component in the ECS
struct PhysicsComponent {
    collision_elasticity: f32
}

enum BallLink {
    ReadyToLink,
    CurrentlyLinked(Entity)
}

// Another example component. Each ball can have a link to another ball (or be ready to link).
struct SmileyBallComponent {
    link: BallLink,
    // countdown_msec: u32,
}

// List your components in this struct. Each entity has one of each (each entry is optional).
struct EntityComponents {
    kinematics: EntityMap<Kinematics>,
    physics: EntityMap<PhysicsComponent>,
    raining_smiley: EntityMap<SmileyBallComponent>,
}

// All other state that doesn't fit into a component goes here.
struct GameResources {
    // hello_msg: String,
    rng: Rng,
    gravity_overall_mult: f32,
    current_wind: (f32, f32),
}

// Here's the global state of the game, in our ECS object!
struct ECS {
    entity_allocator: GenerationalIndexAllocator,
    components: EntityComponents,
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

    /// Adds a ball to the ECS. This isn't a "system" per-se, this is just a function that adds a ball entity.
    /// (this is analogous to a "Command" in Bevy in that it adds an entity.)
    fn add_smiley_ball(gs: &mut ECS) {
        match gs.entity_allocator.allocate() {
            Ok(index) => {
                const SPEED_VARIATION: f32 = 2.0;
                const POS_VARIATION: f32 = 2.0;
                const ELASTICITY_VARIATION: f32 = 0.0;
                let x = ((gs.resources.rng.next() % 1000) as f32 / 1000.0 - 0.5) * POS_VARIATION + 10.0;
                let y = ((gs.resources.rng.next() % 1000) as f32 / 1000.0 - 0.5) * POS_VARIATION + 10.0;
                let vx = ((gs.resources.rng.next() % 1000) as f32 / 1000.0 - 0.5) * SPEED_VARIATION;
                let vy = ((gs.resources.rng.next() % 1000) as f32 / 1000.0 - 0.5) * SPEED_VARIATION; // 5.0 - 2.5;
                let collision_elasticity = ((gs.resources.rng.next() % 1000) as f32 / 1000.0) * ELASTICITY_VARIATION + 1.0;

                // We push this generational index in, then we can reliably set the components (gs.entities will have something in it)
                gs.entities.push(index);
                if let Err(_) = gs.components.kinematics.set(&gs.entities.last().unwrap(), Kinematics{x , y, vx, vy}) {
                    trace("Pos component set fail")

                }
                if let Err(_) = gs.components.physics.set(&gs.entities.last().unwrap(), PhysicsComponent{collision_elasticity}) {
                    trace("Phys component set fail")
                }
                if let Err(_) = gs.components.raining_smiley.set(&gs.entities.last().unwrap(), SmileyBallComponent{link: BallLink::ReadyToLink}) {
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
                    entries.push(AllocatorEntry::new());
                    free.push(i);
                    pos_comp_items.push(None);
                    phys_comp_items.push(None);
                    raining_smiley_items.push(None);
                }

                // Initialization for the ECS happens here.
                STATIC_ECS_DATA = Some(ECS{
                    entity_allocator: GenerationalIndexAllocator::new(entries, free),
                    components: EntityComponents{
                        kinematics: EntityMap{0: pos_comp_items},
                        physics: EntityMap{0: phys_comp_items},
                        raining_smiley: EntityMap{0: raining_smiley_items},
                    },
                    entities,
                    resources: GameResources{
                        // hello_msg: "Hello from Rust!".to_string(),
                        rng: Rng::new(),
                        gravity_overall_mult: 0.015,
                        current_wind: (0.0, 0.0)
                    }
                });

                // Example usage on startup: allocate an entity and give it a position.
                // #[allow(static_mut_ref)]
                if let Some(gs) = &mut STATIC_ECS_DATA {
                    for _ in 0..MAX_N_ENTITIES {
                        add_smiley_ball(gs);
                    }
                }

            },
            _ => {}
        }

        // Once we've intiailized the ECS, a mut ref is available to it outside our unsafe block.
        // #[allow(static_mut_ref)]
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

    /// Example immutable-reference system: take in the ECS and compute something from it (e.g. rendering)
    fn draw_smileys_system(ecs: &ECS) {
        for player in &ecs.entities {
            if let Some(p1) = ecs.components.kinematics.get(&player, &ecs.entity_allocator) {
                if let Some(sm) = ecs.components.raining_smiley.get(&player, &ecs.entity_allocator) {
                    unsafe { *DRAW_COLORS = 0x0002 }
                    if let BallLink::CurrentlyLinked(id2) = sm.link {
                        if let Some(p2) = ecs.components.kinematics.get(&id2, &ecs.entity_allocator) {
                            unsafe { *DRAW_COLORS = 0x0003 }
                            line(p1.x as i32 + 4, p1.y as i32 + 4, p2.x as i32 + 4, p2.y as i32 + 4);
                        } 
                    }
                    blit(&SMILEY, p1.x as i32, p1.y as i32, 8, 8, BLIT_1BPP);
                }
            }
        }
    }

    /// Example mutable-reference system: move all entities that have kinematics.
    fn update_kinematics_system(ecs: &mut ECS) {
        for e in &mut ecs.entities {
            if let Some(pos) = ecs.components.kinematics.get_mut(&e, &ecs.entity_allocator) {
                pos.x += pos.vx;
                pos.y += pos.vy;
            }
        }
    }

    /// Example mutable-reference system. Adds springlike effect to linked smiley balls.
    fn update_smileys_system(ecs: &mut ECS) {
        let mut to_rm = vec![];
        for (i, e) in &mut ecs.entities.iter_mut().enumerate() {
            let mut k2p = None;

            // Check if there's an active linked ball (get its position if so).
            if let Some(sm) = ecs.components.raining_smiley.get(&e, &ecs.entity_allocator) {
                if let BallLink::CurrentlyLinked(o) = sm.link {
                    if let Some(k2) = ecs.components.kinematics.get(&o, &ecs.entity_allocator) {
                        k2p = Some((k2.x, k2.y, o));
                    }
                }
            }
            
            // Update the kinematics of this ball.
            if let Some(pos) = ecs.components.kinematics.get_mut(&e, &ecs.entity_allocator) {
                if let Some(phys) = ecs.components.physics.get(&e, &ecs.entity_allocator) {

                    // apply wind
                    const WIND_SCALER: f32 = 0.03;
                    pos.vx += ecs.resources.current_wind.0 * WIND_SCALER;
                    pos.vy += ecs.resources.current_wind.1 * WIND_SCALER;

                    match k2p {
                        Some(k2p) => {
                            // if it's a linked ball, apply a tension force to its link.
                            let del_x = k2p.0 - pos.x;
                            let del_y = k2p.1 - pos.y;
                            let denom = (del_x.powi(2) + del_y.powi(2)).sqrt();
                            if denom > 0.0 {
                                pos.vy += del_y / denom * ecs.resources.gravity_overall_mult;
                                pos.vx += del_x / denom * ecs.resources.gravity_overall_mult;
                            }

                            // if it's a linked ball, remove it when it hits the screen bounds.
                            if pos.x < 0.0 || pos.x + BALL_WIDTH >= 160.0 || pos.y < 0.0 || pos.y + BALL_HEIGHT >= 160.0 {
                                if let Ok(()) = ecs.entity_allocator.deallocate(&e) {
                                    to_rm.push((i, k2p.2));
                                }
                            }
                        }
                        // if it's an unlinked ball, let it bounce on the edges
                        None => {
                            if pos.x + BALL_WIDTH >= 160.0 {
                                pos.vx *= -phys.collision_elasticity;
                                pos.x = 160.0 - BALL_WIDTH;
                            } else if pos.x < 0.0 {
                                pos.vx *= -phys.collision_elasticity;
                                pos.x = 0.0;
                            }
                            if pos.y + BALL_HEIGHT >= 160.0 {
                                pos.vy = pos.vy.abs() * -phys.collision_elasticity;
                                pos.y = 160.0 - BALL_HEIGHT;
                            } else if pos.y < 0.0 {
                                pos.y = 0.0;
                                pos.vy *= -phys.collision_elasticity;
                            }
                        },
                    }

                    

                    
                }
            }
        }
        // remove ball entities when they've been deallocated successfully (and replace them with new ones!)
        // Also, make sure the other ball that was paired changes state to "ready to link".
        for (i, other_ball) in to_rm.into_iter().rev() {
            ecs.entities.remove(i);
            if let Some(sm) = ecs.components.raining_smiley.get_mut(&other_ball, &ecs.entity_allocator) {
                sm.link = BallLink::ReadyToLink;
            }
            add_smiley_ball(ecs);
        }
    }

    /// Example mutable system: If balls are touching, link them if both have no other link.
    fn link_smileys_system(ecs: &mut ECS) {
        let mut links = vec![];
        let mut linked_entities_this_pass = vec![];
        for i in 0..ecs.entities.len() {
            let e1 = &ecs.entities[i];
            for j in (i+1)..ecs.entities.len() {
                let e2 = &ecs.entities[j];
                if let Some(rs1) = ecs.components.raining_smiley.get(e1, &ecs.entity_allocator) {
                    if let Some(rs2) = ecs.components.raining_smiley.get(e2, &ecs.entity_allocator) {
                        if let Some(k1) = ecs.components.kinematics.get(e1, &ecs.entity_allocator) {
                            if let Some(k2) = ecs.components.kinematics.get(e2, &ecs.entity_allocator) {
                                if (k1.x - k2.x).powi(2) + (k1.y - k2.y).powi(2) < (8.0f32).powi(2) {
                                    if let BallLink::ReadyToLink = rs1.link {
                                        if let BallLink::ReadyToLink = rs2.link {
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
            if let Some(rsm1) = ecs.components.raining_smiley.get_mut(e1, &ecs.entity_allocator) {
                rsm1.link = BallLink::CurrentlyLinked(*e2);
            }
            if let Some(rsm2) = ecs.components.raining_smiley.get_mut(e2, &ecs.entity_allocator) {
                rsm2.link = BallLink::CurrentlyLinked(*e1);
            }
        }
        
    }

    unsafe { *DRAW_COLORS = 2 }

    // text(&ecs.resources.hello_msg, 10, 10);

    let gamepad = unsafe { *GAMEPAD1 };
    ecs.resources.gravity_overall_mult = match gamepad != 0 {
        true => 0.1,
        false => 0.015
    };
    
    // Example input mutable system: this stores game input for other systems to use later (via the resources struct in the ecs struct).
    fn update_input_system(ecs: &mut ECS) {
        unsafe {
            if *GAMEPAD1 & BUTTON_LEFT != 0 {
                ecs.resources.current_wind = (-1.0, 0.0);
            } else if *GAMEPAD1 & BUTTON_RIGHT != 0 {
                ecs.resources.current_wind = (1.0, 0.0);
            } else if *GAMEPAD1 & BUTTON_UP != 0 {
                ecs.resources.current_wind = (0.0, -1.0);
            } else if *GAMEPAD1 & BUTTON_DOWN != 0 {
                ecs.resources.current_wind = (0.0, 1.0);
            } else {
                ecs.resources.current_wind = (0.0, 0.0);
            }
        }
    }


    // Running the game is just playing forward all the systems!!

    // mutable systems
    update_input_system(&mut ecs);
    update_smileys_system(&mut ecs);
    update_kinematics_system(&mut ecs);
    link_smileys_system(&mut ecs);

    // immutable systems
    draw_smileys_system(&ecs);

    unsafe { *DRAW_COLORS = 0x0004 }
    text("rust-wasm4-mini-ecs", 3, 150);
}
