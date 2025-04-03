/*!********************************************************************************************
*
*   Physac v1.1 - 2D Physics library for videogames
*
*   DESCRIPTION:
*
*   Physac is a small 2D physics library written in pure C. The engine uses a fixed time-step thread loop
*   to simluate physics. A physics step contains the following phases: get collision information,
*   apply dynamics, collision solving and position correction. It uses a very simple struct for physic
*   bodies with a position vector to be used in any 3D rendering API.
*
*   CONFIGURATION:
*
*   #define PHYSAC_IMPLEMENTATION
*       Generates the implementation of the library into the included file.
*       If not defined, the library is in header only mode and can be included in other headers
*       or source files without problems. But only ONE file should hold the implementation.
*
*   #define PHYSAC_STATIC (defined by default)
*       The generated implementation will stay private inside implementation file and all
*       internal symbols and functions will only be visible inside that file.
*
*   #define PHYSAC_NO_THREADS
*       The generated implementation won't include pthread library and user must create a secondary thread to call PhysicsThread().
*       It is so important that the thread where PhysicsThread() is called must not have v-sync or any other CPU limitation.
*
*   #define PHYSAC_STANDALONE
*       Avoid raylib.h header inclusion in this file. Data types defined on raylib are defined
*       internally in the library and input management and drawing functions must be provided by
*       the user (check library implementation for further details).
*
*   #define PHYSAC_DEBUG
*       Traces log messages when creating and destroying physics bodies and detects errors in physics
*       calculations and reference exceptions; it is useful for debug purposes
*
*   #define PHYSAC_MALLOC()
*   #define PHYSAC_FREE()
*       You can define your own malloc/free implementation replacing stdlib.h malloc()/free() functions.
*       Otherwise it will include stdlib.h and use the C standard library malloc()/free() function.
*
*
*   NOTE 1: Physac requires multi-threading, when InitPhysics() a second thread is created to manage physics calculations.
*   NOTE 2: Physac requires static C library linkage to avoid dependency on MinGW DLL (-static -lpthread)
*
*   Use the following code to compile:
*   gcc -o $(NAME_PART).exe $(FILE_NAME) -s -static -lraylib -lpthread -lopengl32 -lgdi32 -lwinmm -std=c99
*
*   VERY THANKS TO:
*       - raysan5: helped with library design
*       - ficoos: added support for Linux
*       - R8D8: added support for Linux
*       - jubalh: fixed implementation of time calculations
*       - a3f: fixed implementation of time calculations
*       - define-private-public: added support for OSX
*       - pamarcos: fixed implementation of physics steps
*       - noshbar: fixed some memory leaks
*
*
*   LICENSE: zlib/libpng
*
*   Copyright (c) 2016-2025 Victor Fisac (github: @victorfisac)
*
*   This software is provided "as-is", without any express or implied warranty. In no event
*   will the authors be held liable for any damages arising from the use of this software.
*
*   Permission is granted to anyone to use this software for any purpose, including commercial
*   applications, and to alter it and redistribute it freely, subject to the following restrictions:
*
*     1. The origin of this software must not be misrepresented; you must not claim that you
*     wrote the original software. If you use this software in a product, an acknowledgment
*     in the product documentation would be appreciated but is not required.
*
*     2. Altered source versions must be plainly marked as such, and must not be misrepresented
*     as being the original software.
*
*     3. This notice may not be removed or altered from any source distribution.
*
**********************************************************************************************/

use crate::prelude::Vector2;

//----------------------------------------------------------------------------------
// Defines and Macros
//----------------------------------------------------------------------------------
pub const PHYSAC_MAX_BODIES:      u32 = 64;
pub const PHYSAC_MAX_MANIFOLDS:   u32 = 4096;
pub const PHYSAC_MAX_VERTICES:    u32 = 24;
pub const PHYSAC_CIRCLE_VERTICES: u32 = 24;

pub const PHYSAC_FIXED_TIME:             f32 = 1.0/60.0;
pub const PHYSAC_COLLISION_ITERATIONS:   u32 = 20;
pub const PHYSAC_PENETRATION_ALLOWANCE:  f32 = 0.05;
pub const PHYSAC_PENETRATION_CORRECTION: f32 = 0.4;

//----------------------------------------------------------------------------------
// Types and Structures Definition
//----------------------------------------------------------------------------------
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PhysicsShapeType {
    #[default]
    Circle,
    Polygon,
}
use std::f32::consts::PI;
use raylib_sys::DEG2RAD;
pub use PhysicsShapeType::{
    Circle as PHYSICS_CIRCLE,
    Polygon as PHYSICS_POLYGON,
};

/// Previously defined to be used in PhysicsShape struct as circular dependencies
#[derive(Debug, Clone, Default)]
pub struct PhysicsBody(Weak<RwLock<PhysicsBodyData>>);
impl std::ops::Deref for PhysicsBody {
    type Target = Weak<RwLock<PhysicsBodyData>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for PhysicsBody {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl PhysicsBody {
    #[must_use]
    pub fn try_get<T, F: FnOnce(&PhysicsBodyData) -> T>(&self, f: F) -> Option<T> {
        self.0.upgrade().map(|body| f(&*body.read().unwrap()))
    }

    #[must_use]
    pub fn try_get_mut<T, F: FnOnce(&mut PhysicsBodyData) -> T>(&self, f: F) -> Option<T> {
        self.0.upgrade().map(|body| f(&mut *body.write().unwrap()))
    }

    #[must_use]
    pub fn position(&self) -> Option<Vector2> {
        self.try_get(|body| body.position)
    }

    #[must_use]
    pub fn set_position(&self, value: Vector2) -> Option<()> {
        self.try_get_mut(|body| body.position = value)
    }
}

// Mat2 type (used for polygon shape rotation matrix)
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Mat2 {
    pub m00: f32,
    pub m01: f32,
    pub m10: f32,
    pub m11: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct PolygonData {
    /// Current used vertex and normals count
    pub vertex_count: u32,

    /// Polygon vertex positions vectors
    pub positions: [Vector2; PHYSAC_MAX_VERTICES as usize],

    /// Polygon vertex normals vectors
    pub normals: [Vector2; PHYSAC_MAX_VERTICES as usize],
}

#[derive(Debug, Default)]
pub struct PhysicsShape {
    /// Physics shape type (circle or polygon)
    pub kind: PhysicsShapeType,

    /// Shape physics body reference
    pub body: PhysicsBody,

    /// Circle shape radius (used for circle shapes)
    pub radius: f32,

    /// Vertices transform matrix 2x2
    pub transform: Mat2,

    /// Polygon shape vertices position and normals data (just used for polygon shapes)
    pub vertex_data: PolygonData,
}

#[derive(Debug, Default)]
pub struct PhysicsBodyData {
    /// Reference unique identifier
    pub id: u32,

    /// Enabled dynamics state (collisions are calculated anyway)
    pub enabled: bool,

    /// Physics body shape pivot
    pub position: Vector2,

    /// Current linear velocity applied to position
    pub velocity: Vector2,

    /// Current linear force (reset to 0 every step)
    pub force: Vector2,

    /// Current angular velocity applied to orient
    pub angular_velocity: f32,

    /// Current angular force (reset to 0 every step)
    pub torque: f32,

    /// Rotation in radians
    pub orient: f32,

    /// Moment of inertia
    pub inertia: f32,

    /// Inverse value of inertia
    pub inverse_inertia: f32,

    /// Physics body mass
    pub mass: f32,

    /// Inverse value of mass
    pub inverse_mass: f32,

    /// Friction when the body has not movement (0 to 1)
    pub static_friction: f32,

    /// Friction when the body has movement (0 to 1)
    pub dynamic_friction: f32,

    /// Restitution coefficient of the body (0 to 1)
    pub restitution: f32,

    /// Apply gravity force to dynamics
    pub use_gravity: bool,

    /// Physics grounded on other body state
    pub is_grounded: bool,

    /// Physics rotation constraint
    pub freeze_orient: bool,

    /// Physics body shape information (type, radius, vertices, normals)
    pub shape: PhysicsShape,
}

#[derive(Debug, Default)]
pub struct PhysicsManifoldData {
    /// Reference unique identifier
    pub id: u32,

    /// Manifold first physics body reference
    pub body_a: PhysicsBody,

    /// Manifold second physics body reference
    pub body_b: PhysicsBody,

    /// Depth of penetration from collision
    pub penetration: f32,

    /// Normal direction vector from 'a' to 'b'
    pub normal: Vector2,

    /// Points of contact during collision
    pub contacts: [Vector2; 2],

    /// Current collision number of contacts
    pub contacts_count: u32,

    /// Mixed restitution during collision
    pub restitution: f32,

    /// Mixed dynamic friction during collision
    pub dynamic_friction: f32,

    /// Mixed static friction during collision
    pub static_friction: f32,
}

#[derive(Debug, Clone, Default)]
pub struct PhysicsManifold(Weak<RwLock<PhysicsManifoldData>>);
impl std::ops::Deref for PhysicsManifold {
    type Target = Weak<RwLock<PhysicsManifoldData>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl std::ops::DerefMut for PhysicsManifold {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

/***********************************************************************************
*
*   PHYSAC IMPLEMENTATION
*
************************************************************************************/

use std::{
    sync::{
        atomic::{
            AtomicBool, AtomicU32, Ordering::Relaxed
        }, Arc, LazyLock, OnceLock, RwLock, Weak
    },
    time::{Duration, Instant},
};
// #[cfg(not(feature = "physac_no_threads"))]
// use std::thread;

//----------------------------------------------------------------------------------
// Defines and Macros
//----------------------------------------------------------------------------------
pub const PHYSAC_K: f32 = 1.0/3.0;

//----------------------------------------------------------------------------------
// Global Variables Definition
//----------------------------------------------------------------------------------
// #[cfg(not(feature = "physac_no_threads"))]
// /// Physics thread id
// static pthread_t physicsThreadId;

/// Physics thread enabled state
static PHYSICS_THREAD_ENABLED: AtomicBool = AtomicBool::new(false);
/// Offset time for MONOTONIC clock
static BASE_TIME: OnceLock<Instant> = OnceLock::new();
/// Start time in milliseconds
static mut START_TIME: f64 = 0.0;
/// Delta time used for physics steps, in milliseconds
static mut DELTA_TIME: f64 = 1.0/60.0/10.0 * 1000.0;
/// Current time in milliseconds
static mut CURRENT_TIME: f64 = 0.0;

/// Physics time step delta time accumulator
static mut ACCUMULATOR: f64 = 0.0;
/// Total physics steps processed
static STEPS_COUNT: AtomicU32 = AtomicU32::new(0);
/// Physics world gravity force
static mut GRAVITY_FORCE: Vector2 = Vector2::new(0.0, 9.81);
/// Physics bodies pointers array
static BODIES: LazyLock<Arc<RwLock<[Option<Arc<RwLock<PhysicsBodyData>>>; PHYSAC_MAX_BODIES as usize]>>>
    = LazyLock::new(|| Arc::new(RwLock::new([const { None }; PHYSAC_MAX_BODIES as usize])));
/// Physics world current bodies counter
static PHYSICS_BODIES_COUNT: AtomicU32 = AtomicU32::new(0);
/// Physics bodies pointers array
static CONTACTS: LazyLock<Arc<RwLock<[Option<Arc<RwLock<PhysicsManifoldData>>>; PHYSAC_MAX_MANIFOLDS as usize]>>>
    = LazyLock::new(|| Arc::new(RwLock::new([const { None }; PHYSAC_MAX_MANIFOLDS as usize])));
/// Physics world current manifolds counter
static PHYSICS_MANIFOLDS_COUNT: AtomicU32 = AtomicU32::new(0);

//----------------------------------------------------------------------------------
// Module Functions Definition
//----------------------------------------------------------------------------------
/// Initializes physics values, pointers and creates physics loop thread
pub fn init_physics() {
    // #[cfg(not(feature = "physac_no_threads"))]
    // // NOTE: if defined, user will need to create a thread for PhysicsThread function manually
    // // Create physics thread using POSIXS thread libraries
    // pthread_create(&physicsThreadId, NULL, &PhysicsLoop, NULL);

    // Initialize high resolution timer
    init_timer();

    if cfg!(feature = "physac_debug") {
        println!("[PHYSAC] physics module initialized successfully");
    }

    unsafe {
        ACCUMULATOR = 0.0;
    }
}

/// Returns true if physics thread is currently enabled
pub fn is_physics_enabled() -> bool {
    PHYSICS_THREAD_ENABLED.load(Relaxed)
}

/// Sets physics global gravity force
pub fn set_physics_gravity(x: f32, y: f32) {
    unsafe {
        GRAVITY_FORCE.x = x;
        GRAVITY_FORCE.y = y;
    }
}

impl PhysicsBody {
    /// Creates a new circle physics body with generic parameters
    pub fn create_circle(pos: Vector2, radius: f32, density: f32) -> PhysicsBody {
        let mut new_weak_body = PhysicsBody(Weak::new());
        let new_body = Arc::new(RwLock::new(PhysicsBodyData::default()));

        if let Some(new_id) = find_available_body_index() {
            new_weak_body = PhysicsBody(Arc::downgrade(&new_body));
            let mut new_body_data = new_body.write().unwrap();

            // Initialize new body with generic values
            new_body_data.id = new_id;
            new_body_data.enabled = true;
            new_body_data.position = pos;
            new_body_data.velocity = Vector2::zero();
            new_body_data.force = Vector2::zero();
            new_body_data.angular_velocity = 0.0;
            new_body_data.torque = 0.0;
            new_body_data.orient = 0.0;
            new_body_data.shape.kind = PHYSICS_CIRCLE;
            new_body_data.shape.body = new_weak_body.clone();
            new_body_data.shape.radius = radius;
            new_body_data.shape.transform = Mat2::radians(0.0);
            new_body_data.shape.vertex_data = PolygonData::default();

            new_body_data.mass = PI*radius*radius*density;
            new_body_data.inverse_mass = if new_body_data.mass != 0.0 { 1.0/new_body_data.mass } else { 0.0 };
            new_body_data.inertia = new_body_data.mass*radius*radius;
            new_body_data.inverse_inertia = if new_body_data.inertia != 0.0 { 1.0/new_body_data.inertia } else { 0.0 };
            new_body_data.static_friction = 0.4;
            new_body_data.dynamic_friction = 0.2;
            new_body_data.restitution = 0.0;
            new_body_data.use_gravity = true;
            new_body_data.is_grounded = false;
            new_body_data.freeze_orient = false;

            drop(new_body_data);

            // Add new body to bodies pointers array and update bodies count
            BODIES.write().unwrap()[PHYSICS_BODIES_COUNT.fetch_add(1, Relaxed) as usize] = Some(new_body);

            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] created polygon physics body id {}", new_body.id);
        } else {
            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] new physics body creation failed because there is any available id to use");
        }

        new_weak_body
    }
}

/// Creates a new rectangle physics body with generic parameters
pub fn create_physics_body_rectangle(pos: Vector2, width: f32, height: f32, density: f32) -> PhysicsBody {
    let mut new_weak_body = PhysicsBody(Weak::new());
    let new_body = Arc::new(RwLock::new(PhysicsBodyData::default()));

    if let Some(new_id) = find_available_body_index() {
        new_weak_body = PhysicsBody(Arc::downgrade(&new_body));
        let mut new_body_data = new_body.write().unwrap();

        // Initialize new body with generic values
        new_body_data.id = new_id;
        new_body_data.enabled = true;
        new_body_data.position = pos;
        new_body_data.velocity = Vector2::zero();
        new_body_data.force = Vector2::zero();
        new_body_data.angular_velocity = 0.0;
        new_body_data.torque = 0.0;
        new_body_data.orient = 0.0;
        new_body_data.shape.kind = PHYSICS_POLYGON;
        new_body_data.shape.body = new_weak_body.clone();
        new_body_data.shape.radius = 0.0;
        new_body_data.shape.transform = Mat2::radians(0.0);
        new_body_data.shape.vertex_data = create_rectangle_polygon(pos, Vector2 { x: width, y: height });

        // Calculate centroid and moment of inertia
        let mut center = Vector2 { x: 0.0, y: 0.0 };
        let mut area = 0.0;
        let mut inertia = 0.0;

        for i in 0..new_body_data.shape.vertex_data.vertex_count {
            // Triangle vertices, third vertex implied as (0, 0)
            let p1 = new_body_data.shape.vertex_data.positions[i as usize];
            let next_index = if (i + 1) < new_body_data.shape.vertex_data.vertex_count { i + 1 } else { 0 };
            let p2 = new_body_data.shape.vertex_data.positions[next_index as usize];

            #[allow(non_snake_case)]
            let D = math_cross_vector2(p1, p2);
            let triangle_area = D/2.0;

            area += triangle_area;

            // Use area to weight the centroid average, not just vertex position
            center.x += triangle_area*PHYSAC_K*(p1.x + p2.x);
            center.y += triangle_area*PHYSAC_K*(p1.y + p2.y);

            let intx2 = p1.x*p1.x + p2.x*p1.x + p2.x*p2.x;
            let inty2 = p1.y*p1.y + p2.y*p1.y + p2.y*p2.y;
            inertia += (0.25*PHYSAC_K*D)*(intx2 + inty2);
        }

        center.x *= 1.0/area;
        center.y *= 1.0/area;

        // Translate vertices to centroid (make the centroid (0, 0) for the polygon in model space)
        // Note: this is not really necessary
        for i in 0..new_body_data.shape.vertex_data.vertex_count {
            new_body_data.shape.vertex_data.positions[i as usize].x -= center.x;
            new_body_data.shape.vertex_data.positions[i as usize].y -= center.y;
        }

        new_body_data.mass = density*area;
        new_body_data.inverse_mass = if new_body_data.mass != 0.0 { 1.0/new_body_data.mass } else { 0.0 };
        new_body_data.inertia = density*inertia;
        new_body_data.inverse_inertia = if new_body_data.inertia != 0.0 { 1.0/new_body_data.inertia } else { 0.0 };
        new_body_data.static_friction = 0.4;
        new_body_data.dynamic_friction = 0.2;
        new_body_data.restitution = 0.0;
        new_body_data.use_gravity = true;
        new_body_data.is_grounded = false;
        new_body_data.freeze_orient = false;

        drop(new_body_data);

        // Add new body to bodies pointers array and update bodies count
        BODIES.write().unwrap()[PHYSICS_BODIES_COUNT.fetch_add(1, Relaxed) as usize] = Some(new_body);

        #[cfg(feature = "physac_debug")]
        println!("[PHYSAC] created polygon physics body id {}", new_body.id);
    } else {
        #[cfg(feature = "physac_debug")]
        println!("[PHYSAC] new physics body creation failed because there is any available id to use");
    }

    new_weak_body
}

/// Creates a new polygon physics body with generic parameters
pub fn create_physics_body_polygon(pos: Vector2, radius: f32, sides: u32, density: f32) -> PhysicsBody {
    let mut new_weak_body = PhysicsBody(Weak::new());
    let new_body = Arc::new(RwLock::new(PhysicsBodyData::default()));

    if let Some(new_id) = find_available_body_index() {
        new_weak_body = PhysicsBody(Arc::downgrade(&new_body));
        let mut new_body_data = new_body.write().unwrap();

        // Initialize new body with generic values
        new_body_data.id = new_id;
        new_body_data.enabled = true;
        new_body_data.position = pos;
        new_body_data.velocity = Vector2::zero();
        new_body_data.force = Vector2::zero();
        new_body_data.angular_velocity = 0.0;
        new_body_data.torque = 0.0;
        new_body_data.orient = 0.0;
        new_body_data.shape.kind = PHYSICS_POLYGON;
        new_body_data.shape.body = new_weak_body.clone();
        new_body_data.shape.transform = Mat2::radians(0.0);
        new_body_data.shape.vertex_data = create_random_polygon(radius, sides);

        // Calculate centroid and moment of inertia
        let mut center = Vector2 { x: 0.0, y: 0.0 };
        let mut area = 0.0;
        let mut inertia = 0.0;

        for i in 0..new_body_data.shape.vertex_data.vertex_count {
            // Triangle vertices, third vertex implied as (0, 0)
            let position1 = new_body_data.shape.vertex_data.positions[i as usize];
            let next_index = if (i + 1) < new_body_data.shape.vertex_data.vertex_count { i + 1 } else { 0 };
            let position2 = new_body_data.shape.vertex_data.positions[next_index as usize];

            let cross = math_cross_vector2(position1, position2);
            let triangle_area = cross/2.0;

            area += triangle_area;

            // Use area to weight the centroid average, not just vertex position
            center.x += triangle_area*PHYSAC_K*(position1.x + position2.x);
            center.y += triangle_area*PHYSAC_K*(position1.y + position2.y);

            let intx2 = position1.x*position1.x + position2.x*position1.x + position2.x*position2.x;
            let inty2 = position1.y*position1.y + position2.y*position1.y + position2.y*position2.y;
            inertia += (0.25*PHYSAC_K*cross)*(intx2 + inty2);
        }

        center.x *= 1.0/area;
        center.y *= 1.0/area;

        // Translate vertices to centroid (make the centroid (0, 0) for the polygon in model space)
        // Note: this is not really necessary
        for i in 0..new_body_data.shape.vertex_data.vertex_count {
            new_body_data.shape.vertex_data.positions[i as usize].x -= center.x;
            new_body_data.shape.vertex_data.positions[i as usize].y -= center.y;
        }

        new_body_data.mass = density*area;
        new_body_data.inverse_mass = if new_body_data.mass != 0.0 { 1.0/new_body_data.mass } else { 0.0 };
        new_body_data.inertia = density*inertia;
        new_body_data.inverse_inertia = if new_body_data.inertia != 0.0 { 1.0/new_body_data.inertia } else { 0.0 };
        new_body_data.static_friction = 0.4;
        new_body_data.dynamic_friction = 0.2;
        new_body_data.restitution = 0.0;
        new_body_data.use_gravity = true;
        new_body_data.is_grounded = false;
        new_body_data.freeze_orient = false;

        drop(new_body_data);

        // Add new body to bodies pointers array and update bodies count
        BODIES.write().unwrap()[PHYSICS_BODIES_COUNT.fetch_add(1, Relaxed) as usize] = Some(new_body);

        #[cfg(feature = "physac_debug")]
        println!("[PHYSAC] created polygon physics body id {}", new_body->id);
    } else {
        #[cfg(feature = "physac_debug")]
        println!("[PHYSAC] new physics body creation failed because there is any available id to use");
    }

    new_weak_body
}

impl PhysicsBody {
    /// Adds a force to a physics body
    pub fn add_force(&mut self, force: Vector2) {
        if let Some(body) = self.upgrade() {
            let mut body = body.write().unwrap();
            body.force = body.force + force;
        }
    }

    /// Adds an angular force to a physics body
    pub fn add_torque(&mut self, amount: f32) {
        if let Some(body) = self.upgrade() {
            let mut body = body.write().unwrap();
            body.torque += amount;
        }
    }
}

/// Shatters a polygon shape physics body to little physics bodies with explosion force
pub fn physics_shatter(body: &mut PhysicsBodyData, position: Vector2, force: f32) {
    todo!()
    // if (body != NULL)
    // {
    //     if (body->shape.type == PHYSICS_POLYGON)
    //     {
    //         PolygonData vertexData = body->shape.vertexData;
    //         bool collision = false;

    //         for (int i = 0; i < vertexData.vertexCount; i++)
    //         {
    //             Vector2 positionA = body->position;
    //             Vector2 positionB = Mat2MultiplyVector2(body->shape.transform, Vector2Add(body->position, vertexData.positions[i]));
    //             int nextIndex = (((i + 1) < vertexData.vertexCount) ? (i + 1) : 0);
    //             Vector2 positionC = Mat2MultiplyVector2(body->shape.transform, Vector2Add(body->position, vertexData.positions[nextIndex]));

    //             // Check collision between each triangle
    //             float alpha = ((positionB.y - positionC.y)*(position.x - positionC.x) + (positionC.x - positionB.x)*(position.y - positionC.y))/
    //                           ((positionB.y - positionC.y)*(positionA.x - positionC.x) + (positionC.x - positionB.x)*(positionA.y - positionC.y));

    //             float beta = ((positionC.y - positionA.y)*(position.x - positionC.x) + (positionA.x - positionC.x)*(position.y - positionC.y))/
    //                          ((positionB.y - positionC.y)*(positionA.x - positionC.x) + (positionC.x - positionB.x)*(positionA.y - positionC.y));

    //             float gamma = 1.0f - alpha - beta;

    //             if ((alpha > 0.0f) && (beta > 0.0f) && (gamma > 0.0f))
    //             {
    //                 collision = true;
    //                 break;
    //             }
    //         }

    //         if (collision)
    //         {
    //             int count = vertexData.vertexCount;
    //             Vector2 bodyPos = body->position;
    //             Vector2 *vertices = (Vector2*)PHYSAC_MALLOC(sizeof(Vector2) * count);
    //             Mat2 trans = body->shape.transform;

    //             for (int i = 0; i < count; i++)
    //                 vertices[i] = vertexData.positions[i];

    //             // Destroy shattered physics body
    //             DestroyPhysicsBody(body);

    //             for (int i = 0; i < count; i++)
    //             {
    //                 int nextIndex = (((i + 1) < count) ? (i + 1) : 0);
    //                 Vector2 center = TriangleBarycenter(vertices[i], vertices[nextIndex], PHYSAC_VECTOR_ZERO);
    //                 center = Vector2Add(bodyPos, center);
    //                 Vector2 offset = Vector2Subtract(center, bodyPos);

    //                 PhysicsBody newBody = CreatePhysicsBodyPolygon(center, 10, 3, 10);     // Create polygon physics body with relevant values

    //                 PolygonData newData = { 0 };
    //                 newData.vertexCount = 3;

    //                 newData.positions[0] = Vector2Subtract(vertices[i], offset);
    //                 newData.positions[1] = Vector2Subtract(vertices[nextIndex], offset);
    //                 newData.positions[2] = Vector2Subtract(position, center);

    //                 // Separate vertices to avoid unnecessary physics collisions
    //                 newData.positions[0].x *= 0.95f;
    //                 newData.positions[0].y *= 0.95f;
    //                 newData.positions[1].x *= 0.95f;
    //                 newData.positions[1].y *= 0.95f;
    //                 newData.positions[2].x *= 0.95f;
    //                 newData.positions[2].y *= 0.95f;

    //                 // Calculate polygon faces normals
    //                 for (int j = 0; j < newData.vertexCount; j++)
    //                 {
    //                     int nextVertex = (((j + 1) < newData.vertexCount) ? (j + 1) : 0);
    //                     Vector2 face = Vector2Subtract(newData.positions[nextVertex], newData.positions[j]);

    //                     newData.normals[j] = (Vector2){ face.y, -face.x };
    //                     MathNormalize(&newData.normals[j]);
    //                 }

    //                 // Apply computed vertex data to new physics body shape
    //                 newBody->shape.vertexData = newData;
    //                 newBody->shape.transform = trans;

    //                 // Calculate centroid and moment of inertia
    //                 center = PHYSAC_VECTOR_ZERO;
    //                 float area = 0.0f;
    //                 float inertia = 0.0f;

    //                 for (int j = 0; j < newBody->shape.vertexData.vertexCount; j++)
    //                 {
    //                     // Triangle vertices, third vertex implied as (0, 0)
    //                     Vector2 p1 = newBody->shape.vertexData.positions[j];
    //                     int nextVertex = (((j + 1) < newBody->shape.vertexData.vertexCount) ? (j + 1) : 0);
    //                     Vector2 p2 = newBody->shape.vertexData.positions[nextVertex];

    //                     float D = MathCrossVector2(p1, p2);
    //                     float triangleArea = D/2;

    //                     area += triangleArea;

    //                     // Use area to weight the centroid average, not just vertex position
    //                     center.x += triangleArea*PHYSAC_K*(p1.x + p2.x);
    //                     center.y += triangleArea*PHYSAC_K*(p1.y + p2.y);

    //                     float intx2 = p1.x*p1.x + p2.x*p1.x + p2.x*p2.x;
    //                     float inty2 = p1.y*p1.y + p2.y*p1.y + p2.y*p2.y;
    //                     inertia += (0.25f*PHYSAC_K*D)*(intx2 + inty2);
    //                 }

    //                 center.x *= 1.0f/area;
    //                 center.y *= 1.0f/area;

    //                 newBody->mass = area;
    //                 newBody->inverseMass = ((newBody->mass != 0.0f) ? 1.0f/newBody->mass : 0.0f);
    //                 newBody->inertia = inertia;
    //                 newBody->inverseInertia = ((newBody->inertia != 0.0f) ? 1.0f/newBody->inertia : 0.0f);

    //                 // Calculate explosion force direction
    //                 Vector2 pointA = newBody->position;
    //                 Vector2 pointB = Vector2Subtract(newData.positions[1], newData.positions[0]);
    //                 pointB.x /= 2.0f;
    //                 pointB.y /= 2.0f;
    //                 Vector2 forceDirection = Vector2Subtract(Vector2Add(pointA, Vector2Add(newData.positions[0], pointB)), newBody->position);
    //                 MathNormalize(&forceDirection);
    //                 forceDirection.x *= force;
    //                 forceDirection.y *= force;

    //                 // Apply force to new physics body
    //                 PhysicsAddForce(newBody, forceDirection);
    //             }

    //             PHYSAC_FREE(vertices);
    //         }
    //     }
    // }
    // #if defined(PHYSAC_DEBUG)
    //     else
    //         println!("[PHYSAC] error when trying to shatter a null reference physics body");
    // #endif
}

/// Returns the current amount of created physics bodies
pub fn get_physics_bodies_count() -> u32 {
    PHYSICS_BODIES_COUNT.load(Relaxed)
}

/// Returns a physics body of the bodies pool at a specific index
///
/// # Panics
///
/// May panic if the index is out of bounds.
pub fn get_physics_body(index: u32) -> Option<PhysicsBody> {
    let bodies = BODIES.read().unwrap();

    if index < PHYSICS_BODIES_COUNT.load(Relaxed) {
        if bodies[index as usize].is_none() {
            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] error when trying to get a null reference physics body");
        }
    } else {
        #[cfg(feature = "physac_debug")]
        println!("[PHYSAC] physics body index is out of bounds");
    }

    bodies[index as usize]
        .as_ref()
        .map(|body| PhysicsBody(Arc::downgrade(body)))
}

/// Returns the physics body shape type (PHYSICS_CIRCLE or PHYSICS_POLYGON)
pub fn get_physics_shape_type(index: u32) -> Option<PhysicsShapeType> {
    let mut result = None;
    let bodies = BODIES.read().unwrap();

    if index < PHYSICS_BODIES_COUNT.load(Relaxed) {
        if let Some(body) = bodies[index as usize].as_ref() {
            let body = body.read().unwrap();
            result = Some(body.shape.kind);
        } else {
            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] error when trying to get a null reference physics body");
        }
    } else {
        #[cfg(feature = "physac_debug")]
        println!("[PHYSAC] physics body index is out of bounds");
    }

    result
}

/// Returns the amount of vertices of a physics body shape
pub fn get_physics_shape_vertices_count(index: u32) -> u32 {
    let mut result = 0;
    let bodies = BODIES.read().unwrap();

    if index < PHYSICS_BODIES_COUNT.load(Relaxed) {
        if let Some(body) = bodies[index as usize].as_ref() {
            let body = body.read().unwrap();
            result = match body.shape.kind {
                PHYSICS_CIRCLE => PHYSAC_CIRCLE_VERTICES,
                PHYSICS_POLYGON => body.shape.vertex_data.vertex_count,
            };
        } else {
            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] error when trying to get a null reference physics body");
        }
    } else {
        #[cfg(feature = "physac_debug")]
        println!("[PHYSAC] physics body index is out of bounds");
    }

    result
}

impl PhysicsBody {
    /// Returns transformed position of a body shape (body position + vertex transformed position)
    pub fn get_shape_vertex(&self, vertex: i32) -> Vector2 {
        let mut position = Vector2 { x: 0.0, y: 0.0 };

        if let Some(body) = self.upgrade() {
            let body = body.read().unwrap();
            match body.shape.kind {
                PHYSICS_CIRCLE => {
                    position.x = body.position.x + (360.0/PHYSAC_CIRCLE_VERTICES as f32*vertex as f32*DEG2RAD as f32).cos()*body.shape.radius;
                    position.y = body.position.y + (360.0/PHYSAC_CIRCLE_VERTICES as f32*vertex as f32*DEG2RAD as f32).sin()*body.shape.radius;
                }
                PHYSICS_POLYGON => {
                    let vertex_data = body.shape.vertex_data;
                    position = body.position + body.shape.transform.multiply_vector2(vertex_data.positions[vertex as usize]);
                }
            }
        } else {
            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] error when trying to get a null reference physics body");
        }

        position
    }

    /// Sets physics body shape transform based on radians parameter
    pub fn set_rotation(&mut self, radians: f32) {
        if let Some(body) = self.upgrade() {
            let mut body = body.write().unwrap();

            body.orient = radians;

            if body.shape.kind == PHYSICS_POLYGON {
                body.shape.transform = Mat2::radians(radians);
            }
        }
    }
}

impl PhysicsBody {
    /// Unitializes and destroys a physics body
    pub fn destroy(self) {
        if let Some(body) = self.upgrade() {
            let id = body.read().unwrap().id;
            let mut index = None;

            let mut bodies = BODIES.write().unwrap();

            for i in 0..PHYSICS_BODIES_COUNT.load(Relaxed) {
                let body = bodies[i as usize].as_ref().unwrap();
                if body.read().unwrap().id == id {
                    index = Some(i);
                    break;
                }
            }

            if index.is_none() {
                #[cfg(feature = "physac_debug")]
                println!("[PHYSAC] Not possible to find body id {} in pointers array", id);
                return;
            }
            let index = index.unwrap();

            // Free body allocated memory
            drop(body);
            bodies[index as usize] = None;

            // Reorder physics bodies pointers array and its catched index
            for i in index..PHYSICS_BODIES_COUNT.load(Relaxed) {
                if let ([.., curr], [next, ..]) = bodies.split_at_mut(i as usize) {
                    std::mem::swap(curr, next);
                }
            }

            // Update physics bodies count
            PHYSICS_BODIES_COUNT.store(PHYSICS_BODIES_COUNT.load(Relaxed) - 1, Relaxed);

            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] destroyed physics body id {}", id);
        } else {
            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] error trying to destroy a null referenced body");
        }
    }
}

/// Unitializes physics pointers and exits physics loop thread
pub fn close_physics() {
    // Exit physics loop thread
    PHYSICS_THREAD_ENABLED.store(false, Relaxed);

    // #[cfg(not(feature = "physac_no_threads"))]
    // pthread_join(physicsThreadId, NULL);

    let contacts = CONTACTS.write().unwrap();
    let bodies = BODIES.write().unwrap();

    // Unitialize physics manifolds dynamic memory allocations
    for i in (0..PHYSICS_MANIFOLDS_COUNT.load(Relaxed) - 1).rev() {
        PhysicsManifold(Arc::downgrade(contacts[i as usize].as_ref().unwrap())).destroy();
    }

    // Unitialize physics bodies dynamic memory allocations
    for i in (0..PHYSICS_BODIES_COUNT.load(Relaxed) - 1).rev() {
        PhysicsBody(Arc::downgrade(bodies[i as usize].as_ref().unwrap())).destroy();
    }

    #[cfg(feature = "physac_debug")]
    println!("[PHYSAC] physics module closed successfully");
}

//----------------------------------------------------------------------------------
// Module Internal Functions Definition
//----------------------------------------------------------------------------------
/// Finds a valid index for a new physics body initialization
fn find_available_body_index() -> Option<u32> {
    let mut index = None;
    let bodies = BODIES.read().unwrap();
    for i in 0..PHYSAC_MAX_BODIES {
        let mut current_id = i;

        // Check if current id already exist in other physics body
        for k in 0..PHYSICS_BODIES_COUNT.load(Relaxed) {
            let body = bodies[k as usize].as_ref().unwrap(); // every body index < PHYSICS_BODIES_COUNT should be Some
            let body = body.read().unwrap();
            if body.id == current_id {
                current_id += 1;
                break;
            }
        }

        // If it is not used, use it as new physics body id
        if current_id == i {
            index = Some(i);
            break;
        }
    }

    index
}

/// Creates a random polygon shape with max vertex distance from polygon pivot
fn create_random_polygon(radius: f32, sides: u32) -> PolygonData {
    let mut data = PolygonData::default();
    data.vertex_count = sides;

    // Calculate polygon vertices positions
    for i in 0..data.vertex_count {
        data.positions[i as usize].x = (360.0/sides as f32*i as f32*DEG2RAD as f32).cos()*radius;
        data.positions[i as usize].y = (360.0/sides as f32*i as f32*DEG2RAD as f32).sin()*radius;
    }

    // Calculate polygon faces normals
    for i in 0..data.vertex_count {
        let next_index = if (i + 1) < sides { i + 1 } else { 0 };
        let face = data.positions[next_index as usize] - data.positions[i as usize];

        data.normals[i as usize] = Vector2 { x: face.y, y: -face.x };
        math_normalize(&mut data.normals[i as usize]);
    }

    data
}

/// Creates a rectangle polygon shape based on a min and max positions
fn create_rectangle_polygon(pos: Vector2, size: Vector2) -> PolygonData {
    let mut data = PolygonData::default();
    data.vertex_count = 4;

    // Calculate polygon vertices positions
    data.positions[0] = Vector2 { x: pos.x + size.x/2.0, y: pos.y - size.y/2.0 };
    data.positions[1] = Vector2 { x: pos.x + size.x/2.0, y: pos.y + size.y/2.0 };
    data.positions[2] = Vector2 { x: pos.x - size.x/2.0, y: pos.y + size.y/2.0 };
    data.positions[3] = Vector2 { x: pos.x - size.x/2.0, y: pos.y - size.y/2.0 };

    // Calculate polygon faces normals
    for i in 0..data.vertex_count {
        let next_index = if (i + 1) < data.vertex_count { i + 1 } else { 0 };
        let face = data.positions[next_index as usize] - data.positions[i as usize];

        data.normals[i as usize] = Vector2 { x: face.y, y: -face.x };
        math_normalize(&mut data.normals[i as usize]);
    }

    data
}

/// Physics loop thread function
fn physics_loop() {
    #[cfg(feature = "physac_debug")]
    println!("[PHYSAC] physics thread created successfully");

    // Initialize physics loop thread values
    PHYSICS_THREAD_ENABLED.store(true, Relaxed);

    // Physics update loop
    while PHYSICS_THREAD_ENABLED.load(Relaxed) {
        run_physics_step();

        let req = Duration::from_nanos((PHYSAC_FIXED_TIME*1000.0*1000.0) as u64);

        std::thread::sleep(req);
    }
}

/// Physics steps calculations (dynamics, collisions and position corrections)
fn physics_step() {
    todo!()
    // // Update current steps count
    // stepsCount++;

    // // Clear previous generated collisions information
    // for (int i = physicsManifoldsCount - 1; i >= 0; i--)
    // {
    //     PhysicsManifold manifold = contacts[i];

    //     if (manifold != NULL)
    //         DestroyPhysicsManifold(manifold);
    // }

    // // Reset physics bodies grounded state
    // for (int i = 0; i < physicsBodiesCount; i++)
    // {
    //     PhysicsBody body = bodies[i];
    //     body->isGrounded = false;
    // }

    // // Generate new collision information
    // for (int i = 0; i < physicsBodiesCount; i++)
    // {
    //     PhysicsBody bodyA = bodies[i];

    //     if (bodyA != NULL)
    //     {
    //         for (int j = i + 1; j < physicsBodiesCount; j++)
    //         {
    //             PhysicsBody bodyB = bodies[j];

    //             if (bodyB != NULL)
    //             {
    //                 if ((bodyA->inverseMass == 0) && (bodyB->inverseMass == 0))
    //                     continue;

    //                 PhysicsManifold manifold = CreatePhysicsManifold(bodyA, bodyB);
    //                 SolvePhysicsManifold(manifold);

    //                 if (manifold->contactsCount > 0)
    //                 {
    //                     // Create a new manifold with same information as previously solved manifold and add it to the manifolds pool last slot
    //                     PhysicsManifold newManifold = CreatePhysicsManifold(bodyA, bodyB);
    //                     newManifold->penetration = manifold->penetration;
    //                     newManifold->normal = manifold->normal;
    //                     newManifold->contacts[0] = manifold->contacts[0];
    //                     newManifold->contacts[1] = manifold->contacts[1];
    //                     newManifold->contactsCount = manifold->contactsCount;
    //                     newManifold->restitution = manifold->restitution;
    //                     newManifold->dynamicFriction = manifold->dynamicFriction;
    //                     newManifold->staticFriction = manifold->staticFriction;
    //                 }
    //             }
    //         }
    //     }
    // }

    // // Integrate forces to physics bodies
    // for (int i = 0; i < physicsBodiesCount; i++)
    // {
    //     PhysicsBody body = bodies[i];

    //     if (body != NULL)
    //         IntegratePhysicsForces(body);
    // }

    // // Initialize physics manifolds to solve collisions
    // for (int i = 0; i < physicsManifoldsCount; i++)
    // {
    //     PhysicsManifold manifold = contacts[i];

    //     if (manifold != NULL)
    //         InitializePhysicsManifolds(manifold);
    // }

    // // Integrate physics collisions impulses to solve collisions
    // for (int i = 0; i < PHYSAC_COLLISION_ITERATIONS; i++)
    // {
    //     for (int j = 0; j < physicsManifoldsCount; j++)
    //     {
    //         PhysicsManifold manifold = contacts[j];

    //         if (manifold != NULL)
    //             IntegratePhysicsImpulses(manifold);
    //     }
    // }

    // // Integrate velocity to physics bodies
    // for (int i = 0; i < physicsBodiesCount; i++)
    // {
    //     PhysicsBody body = bodies[i];

    //     if (body != NULL)
    //         IntegratePhysicsVelocity(body);
    // }

    // // Correct physics bodies positions based on manifolds collision information
    // for (int i = 0; i < physicsManifoldsCount; i++)
    // {
    //     PhysicsManifold manifold = contacts[i];

    //     if (manifold != NULL)
    //         CorrectPhysicsPositions(manifold);
    // }

    // // Clear physics bodies forces
    // for (int i = 0; i < physicsBodiesCount; i++)
    // {
    //     PhysicsBody body = bodies[i];

    //     if (body != NULL)
    //     {
    //         body->force = PHYSAC_VECTOR_ZERO;
    //         body->torque = 0.0f;
    //     }
    // }
}

/// Wrapper to ensure PhysicsStep is run with at a fixed time step
pub fn run_physics_step() {
    todo!()
    // // Calculate current time
    // currentTime = GetCurrTime();

    // // Calculate current delta time
    // double delta = currentTime - startTime;

    // // Store the time elapsed since the last frame began
    // accumulator += delta;

    // // Fixed time stepping loop
    // while (accumulator >= deltaTime)
    // {
    //     PhysicsStep();
    //     accumulator -= deltaTime;
    // }

    // // Record the starting of this frame
    // startTime = currentTime;
}

pub fn set_physics_time_step(delta: f64) {
    unsafe {
        DELTA_TIME = delta;
    }
}

/// Finds a valid index for a new manifold initialization
fn find_available_manifold_index() -> Option<u32> {
    let id = PHYSICS_MANIFOLDS_COUNT.load(Relaxed) + 1;

    if id >= PHYSAC_MAX_MANIFOLDS {
        return None;
    }

    Some(id)
}

impl PhysicsManifold {
    /// Creates a new physics manifold to solve collision
    fn create(a: &PhysicsBody, b: &PhysicsBody) -> PhysicsManifold {
        let mut new_weak_manifold = PhysicsManifold(Weak::new());
        let new_manifold = Arc::new(RwLock::new(PhysicsManifoldData::default()));

        if let Some(new_id) = find_available_manifold_index() {
            new_weak_manifold = PhysicsManifold(Arc::downgrade(&new_manifold));
            // unwraps are safe here because there is no way something else has a reference to the arc we JUST created locally
            let mut new_manifold_data = new_manifold.write().unwrap();

            // Initialize new manifold with generic values
            new_manifold_data.id = new_id;
            new_manifold_data.body_a = a.clone();
            new_manifold_data.body_b = b.clone();
            new_manifold_data.penetration = 0.0;
            new_manifold_data.normal = Vector2::zero();
            new_manifold_data.contacts[0] = Vector2::zero();
            new_manifold_data.contacts[1] = Vector2::zero();
            new_manifold_data.contacts_count = 0;
            new_manifold_data.restitution = 0.0;
            new_manifold_data.dynamic_friction = 0.0;
            new_manifold_data.static_friction = 0.0;

            drop(new_manifold_data);

            // Add new body to bodies pointers array and update bodies count
            CONTACTS.write().unwrap()[PHYSICS_MANIFOLDS_COUNT.fetch_add(1, Relaxed) as usize] = Some(new_manifold);
        } else {
            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] new physics manifold creation failed because there is any available id to use");
        }

        new_weak_manifold
    }

    /// Unitializes and destroys a physics manifold
    fn destroy(self) {
        if let Some(manifold) = self.upgrade() {
            let mut contacts = CONTACTS.write().unwrap();

            let id = manifold.read().unwrap().id;
            let mut index = None;

            for i in 0..PHYSICS_MANIFOLDS_COUNT.load(Relaxed) {
                let contact = contacts[i as usize].as_ref().unwrap();
                if contact.read().unwrap().id == id {
                    index = Some(i);
                    break;
                }
            }

            if index.is_none() {
                #[cfg(feature = "physac_debug")]
                println!("[PHYSAC] Not possible to manifold id {} in pointers array", id);
                return;
            }
            let index = index.unwrap();

            // Free manifold allocated memory
            drop(manifold);
            contacts[index as usize] = None;

            // Reorder physics manifolds pointers array and its catched index
            for i in index..PHYSICS_MANIFOLDS_COUNT.load(Relaxed) {
                if let ([.., curr], [next, ..]) = contacts.split_at_mut(i as usize) {
                    std::mem::swap(curr, next);
                }
            }

            // Update physics manifolds count
            PHYSICS_MANIFOLDS_COUNT.store(PHYSICS_MANIFOLDS_COUNT.load(Relaxed) - 1, Relaxed);
        } else {
            #[cfg(feature = "physac_debug")]
            println!("[PHYSAC] error trying to destroy a null referenced manifold");
        }
    }
}

impl PhysicsManifoldData {
    /// Solves a created physics manifold between two physics bodies
    fn solve_physics_manifold(&mut self) {
        let body_a = self.body_a.upgrade().unwrap();
        let body_b = self.body_b.upgrade().unwrap();

        let     body_a = body_a.read ().unwrap();
        let mut body_b = body_b.write().unwrap();

        match body_a.shape.kind {
            PHYSICS_CIRCLE => {
                match body_b.shape.kind {
                    PHYSICS_CIRCLE => self.solve_circle_to_circle(),
                    PHYSICS_POLYGON => self.solve_circle_to_polygon(),
                }
            }
            PHYSICS_POLYGON => {
                match body_b.shape.kind {
                    PHYSICS_CIRCLE => self.solve_polygon_to_circle(),
                    PHYSICS_POLYGON => self.solve_polygon_to_polygon(),
                }
            }
        }

        // Update physics body grounded state if normal direction is down and grounded state is not set yet in previous manifolds
        if !body_b.is_grounded {
            body_b.is_grounded = self.normal.y < 0.0;
        }
    }

    /// Solves collision between two circle shape physics bodies
    fn solve_circle_to_circle(&mut self) {
        let body_a = self.body_a.upgrade();
        let body_b = self.body_b.upgrade();

        if let (Some(body_a), Some(body_b)) = (body_a, body_b) {
            let mut body_a = body_a.write().unwrap();
            let     body_b = body_b.read ().unwrap();

            // Calculate translational vector, which is normal
            let normal = body_b.position - body_a.position;

            let dist_sqr = normal.length_sqr();
            let radius = body_a.shape.radius + body_b.shape.radius;

            // Check if circles are not in contact
            if dist_sqr >= radius*radius {
                self.contacts_count = 0;
                return;
            }

            let distance = dist_sqr.sqrt();
            self.contacts_count = 1;

            if distance == 0.0 {
                self.penetration = body_a.shape.radius;
                self.normal = Vector2 { x: 1.0, y: 0.0 };
                self.contacts[0] = body_a.position;
            } else {
                self.penetration = radius - distance;
                self.normal = Vector2 { x: normal.x/distance, y: normal.y/distance }; // Faster than using MathNormalize() due to sqrt is already performed
                self.contacts[0] = Vector2 {
                    x: self.normal.x*body_a.shape.radius + body_a.position.x,
                    y: self.normal.y*body_a.shape.radius + body_a.position.y,
                };
            }

            // Update physics body grounded state if normal direction is down
            if !body_a.is_grounded {
                body_a.is_grounded = self.normal.y < 0.0;
            }
        }
    }

    /// Solves collision between a circle to a polygon shape physics bodies
    fn solve_circle_to_polygon(&mut self) {
        let body_a = self.body_a.upgrade();
        let body_b = self.body_b.upgrade();

        if let (Some(body_a), Some(body_b)) = (body_a, body_b) {
            let mut body_a = body_a.write().unwrap();
            let mut body_b = body_b.write().unwrap();

            self.solve_different_shapes(&mut *body_a, &mut *body_b);
        }
    }

    /// Solves collision between a circle to a polygon shape physics bodies
    fn solve_polygon_to_circle(&mut self) {
        let body_a = self.body_a.upgrade();
        let body_b = self.body_b.upgrade();

        if let (Some(body_a), Some(body_b)) = (body_a, body_b) {
            let mut body_a = body_a.write().unwrap();
            let mut body_b = body_b.write().unwrap();

            self.solve_different_shapes(&mut *body_b, &mut *body_a);

            self.normal.x *= -1.0;
            self.normal.y *= -1.0;
        }
    }

    /// Solve collision between two different types of shapes
    fn solve_different_shapes(&mut self, body_a: &mut PhysicsBodyData, body_b: &mut PhysicsBodyData) {
        todo!()
        // manifold->contactsCount = 0;

        // // Transform circle center to polygon transform space
        // Vector2 center = bodyA->position;
        // center = Mat2MultiplyVector2(Mat2Transpose(bodyB->shape.transform), Vector2Subtract(center, bodyB->position));

        // // Find edge with minimum penetration
        // // It is the same concept as using support points in SolvePolygonToPolygon
        // float separation = -PHYSAC_FLT_MAX;
        // int faceNormal = 0;
        // PolygonData vertexData = bodyB->shape.vertexData;

        // for (int i = 0; i < vertexData.vertexCount; i++)
        // {
        //     float currentSeparation = MathDot(vertexData.normals[i], Vector2Subtract(center, vertexData.positions[i]));

        //     if (currentSeparation > bodyA->shape.radius)
        //         return;

        //     if (currentSeparation > separation)
        //     {
        //         separation = currentSeparation;
        //         faceNormal = i;
        //     }
        // }

        // // Grab face's vertices
        // Vector2 v1 = vertexData.positions[faceNormal];
        // int nextIndex = (((faceNormal + 1) < vertexData.vertexCount) ? (faceNormal + 1) : 0);
        // Vector2 v2 = vertexData.positions[nextIndex];

        // // Check to see if center is within polygon
        // if (separation < PHYSAC_EPSILON)
        // {
        //     manifold->contactsCount = 1;
        //     Vector2 normal = Mat2MultiplyVector2(bodyB->shape.transform, vertexData.normals[faceNormal]);
        //     manifold->normal = (Vector2){ -normal.x, -normal.y };
        //     manifold->contacts[0] = (Vector2){ manifold->normal.x*bodyA->shape.radius + bodyA->position.x, manifold->normal.y*bodyA->shape.radius + bodyA->position.y };
        //     manifold->penetration = bodyA->shape.radius;
        //     return;
        // }

        // // Determine which voronoi region of the edge center of circle lies within
        // float dot1 = MathDot(Vector2Subtract(center, v1), Vector2Subtract(v2, v1));
        // float dot2 = MathDot(Vector2Subtract(center, v2), Vector2Subtract(v1, v2));
        // manifold->penetration = bodyA->shape.radius - separation;

        // if (dot1 <= 0.0f) // Closest to v1
        // {
        //     if (DistSqr(center, v1) > bodyA->shape.radius*bodyA->shape.radius)
        //         return;

        //     manifold->contactsCount = 1;
        //     Vector2 normal = Vector2Subtract(v1, center);
        //     normal = Mat2MultiplyVector2(bodyB->shape.transform, normal);
        //     MathNormalize(&normal);
        //     manifold->normal = normal;
        //     v1 = Mat2MultiplyVector2(bodyB->shape.transform, v1);
        //     v1 = Vector2Add(v1, bodyB->position);
        //     manifold->contacts[0] = v1;
        // }
        // else if (dot2 <= 0.0f) // Closest to v2
        // {
        //     if (DistSqr(center, v2) > bodyA->shape.radius*bodyA->shape.radius)
        //         return;

        //     manifold->contactsCount = 1;
        //     Vector2 normal = Vector2Subtract(v2, center);
        //     v2 = Mat2MultiplyVector2(bodyB->shape.transform, v2);
        //     v2 = Vector2Add(v2, bodyB->position);
        //     manifold->contacts[0] = v2;
        //     normal = Mat2MultiplyVector2(bodyB->shape.transform, normal);
        //     MathNormalize(&normal);
        //     manifold->normal = normal;
        // }
        // else // Closest to face
        // {
        //     Vector2 normal = vertexData.normals[faceNormal];

        //     if (MathDot(Vector2Subtract(center, v1), normal) > bodyA->shape.radius)
        //         return;

        //     normal = Mat2MultiplyVector2(bodyB->shape.transform, normal);
        //     manifold->normal = (Vector2){ -normal.x, -normal.y };
        //     manifold->contacts[0] = (Vector2){ manifold->normal.x*bodyA->shape.radius + bodyA->position.x, manifold->normal.y*bodyA->shape.radius + bodyA->position.y };
        //     manifold->contactsCount = 1;
        // }
    }

    /// Solves collision between two polygons shape physics bodies
    fn solve_polygon_to_polygon(&mut self) {
        todo!()
        // if ((manifold->bodyA == NULL) || (manifold->bodyB == NULL))
        //     return;

        // PhysicsShape bodyA = manifold->bodyA->shape;
        // PhysicsShape bodyB = manifold->bodyB->shape;
        // manifold->contactsCount = 0;

        // // Check for separating axis with A shape's face planes
        // int faceA = 0;
        // float penetrationA = FindAxisLeastPenetration(&faceA, bodyA, bodyB);

        // if (penetrationA >= 0.0f)
        //     return;

        // // Check for separating axis with B shape's face planes
        // int faceB = 0;
        // float penetrationB = FindAxisLeastPenetration(&faceB, bodyB, bodyA);

        // if (penetrationB >= 0.0f)
        //     return;

        // int referenceIndex = 0;
        // bool flip = false;  // Always point from A shape to B shape

        // PhysicsShape refPoly; // Reference
        // PhysicsShape incPoly; // Incident

        // // Determine which shape contains reference face
        // if (BiasGreaterThan(penetrationA, penetrationB))
        // {
        //     refPoly = bodyA;
        //     incPoly = bodyB;
        //     referenceIndex = faceA;
        // }
        // else
        // {
        //     refPoly = bodyB;
        //     incPoly = bodyA;
        //     referenceIndex = faceB;
        //     flip = true;
        // }

        // // World space incident face
        // Vector2 incidentFace[2];
        // FindIncidentFace(&incidentFace[0], &incidentFace[1], refPoly, incPoly, referenceIndex);

        // // Setup reference face vertices
        // PolygonData refData = refPoly.vertexData;
        // Vector2 v1 = refData.positions[referenceIndex];
        // referenceIndex = (((referenceIndex + 1) < refData.vertexCount) ? (referenceIndex + 1) : 0);
        // Vector2 v2 = refData.positions[referenceIndex];

        // // Transform vertices to world space
        // v1 = Mat2MultiplyVector2(refPoly.transform, v1);
        // v1 = Vector2Add(v1, refPoly.body->position);
        // v2 = Mat2MultiplyVector2(refPoly.transform, v2);
        // v2 = Vector2Add(v2, refPoly.body->position);

        // // Calculate reference face side normal in world space
        // Vector2 sidePlaneNormal = Vector2Subtract(v2, v1);
        // MathNormalize(&sidePlaneNormal);

        // // Orthogonalize
        // Vector2 refFaceNormal = { sidePlaneNormal.y, -sidePlaneNormal.x };
        // float refC = MathDot(refFaceNormal, v1);
        // float negSide = MathDot(sidePlaneNormal, v1)*-1;
        // float posSide = MathDot(sidePlaneNormal, v2);

        // // Clip incident face to reference face side planes (due to floating point error, possible to not have required points
        // if (Clip((Vector2){ -sidePlaneNormal.x, -sidePlaneNormal.y }, negSide, &incidentFace[0], &incidentFace[1]) < 2)
        //     return;

        // if (Clip(sidePlaneNormal, posSide, &incidentFace[0], &incidentFace[1]) < 2)
        //     return;

        // // Flip normal if required
        // manifold->normal = (flip ? (Vector2){ -refFaceNormal.x, -refFaceNormal.y } : refFaceNormal);

        // // Keep points behind reference face
        // int currentPoint = 0; // Clipped points behind reference face
        // float separation = MathDot(refFaceNormal, incidentFace[0]) - refC;

        // if (separation <= 0.0f)
        // {
        //     manifold->contacts[currentPoint] = incidentFace[0];
        //     manifold->penetration = -separation;
        //     currentPoint++;
        // }
        // else
        //     manifold->penetration = 0.0f;

        // separation = MathDot(refFaceNormal, incidentFace[1]) - refC;

        // if (separation <= 0.0f)
        // {
        //     manifold->contacts[currentPoint] = incidentFace[1];
        //     manifold->penetration += -separation;
        //     currentPoint++;

        //     // Calculate total penetration average
        //     manifold->penetration /= currentPoint;
        // }

        // manifold->contactsCount = currentPoint;
    }
}

impl PhysicsBodyData {
    /// Integrates physics forces into velocity
    fn integrate_physics_forces(&mut self) {
        if (self.inverse_mass == 0.0) || !self.enabled {
            return;
        }

        self.velocity.x += ((self.force.x*self.inverse_mass) as f64*(unsafe { DELTA_TIME }/2.0)) as f32;
        self.velocity.y += ((self.force.y*self.inverse_mass) as f64*(unsafe { DELTA_TIME }/2.0)) as f32;

        if self.use_gravity {
            self.velocity.x += (unsafe { GRAVITY_FORCE.x as f64 }*(unsafe { DELTA_TIME }/1000.0/2.0)) as f32;
            self.velocity.y += (unsafe { GRAVITY_FORCE.y as f64 }*(unsafe { DELTA_TIME }/1000.0/2.0)) as f32;
        }

        if !self.freeze_orient {
            self.angular_velocity += (self.torque as f64*self.inverse_inertia as f64*(unsafe { DELTA_TIME }/2.0)) as f32;
        }
    }
}

impl PhysicsManifoldData {
    /// Initializes physics manifolds to solve collisions
    fn initialize_physics_manifolds(&mut self) {
        let body_a = self.body_a.upgrade();
        let body_b = self.body_b.upgrade();

        if let (Some(body_a), Some(body_b)) = (body_a, body_b) {
            let body_a = body_a.read().unwrap();
            let body_b = body_b.read().unwrap();

            // Calculate average restitution, static and dynamic friction
            self.restitution = (body_a.restitution*body_b.restitution).sqrt();
            self.static_friction = (body_a.static_friction*body_b.static_friction).sqrt();
            self.dynamic_friction = (body_a.dynamic_friction*body_b.dynamic_friction).sqrt();

            for i in 0..self.contacts_count
            {
                // Caculate radius from center of mass to contact
                let radius_a = self.contacts[i as usize] - body_a.position;
                let radius_b = self.contacts[i as usize] - body_b.position;

                let cross_a = math_cross(body_a.angular_velocity, radius_a);
                let cross_b = math_cross(body_b.angular_velocity, radius_b);

                let mut radius_v = Vector2 { x: 0.0, y: 0.0 };
                radius_v.x = body_b.velocity.x + cross_b.x - body_a.velocity.x - cross_a.x;
                radius_v.y = body_b.velocity.y + cross_b.y - body_a.velocity.y - cross_a.y;

                // Determine if we should perform a resting collision or not;
                // The idea is if the only thing moving this object is gravity, then the collision should be performed without any restitution
                if radius_v.length_sqr() < ((Vector2 {
                    x: unsafe { GRAVITY_FORCE.x }*unsafe { DELTA_TIME } as f32/1000.0,
                    y: unsafe { GRAVITY_FORCE.y }*unsafe { DELTA_TIME } as f32/1000.0,
                }).length_sqr() + f32::EPSILON) {
                    self.restitution = 0.0;
                }
            }
        }
    }

    /// Integrates physics collisions impulses to solve collisions
    fn integrate_physics_impulses(&mut self) {
        todo!()
        // PhysicsBody bodyA = manifold->bodyA;
        // PhysicsBody bodyB = manifold->bodyB;

        // if ((bodyA == NULL) || (bodyB == NULL))
        //     return;

        // // Early out and positional correct if both objects have infinite mass
        // if (fabs(bodyA->inverseMass + bodyB->inverseMass) <= PHYSAC_EPSILON)
        // {
        //     bodyA->velocity = PHYSAC_VECTOR_ZERO;
        //     bodyB->velocity = PHYSAC_VECTOR_ZERO;
        //     return;
        // }

        // for (int i = 0; i < manifold->contactsCount; i++)
        // {
        //     // Calculate radius from center of mass to contact
        //     Vector2 radiusA = Vector2Subtract(manifold->contacts[i], bodyA->position);
        //     Vector2 radiusB = Vector2Subtract(manifold->contacts[i], bodyB->position);

        //     // Calculate relative velocity
        //     Vector2 radiusV = { 0.0f, 0.0f };
        //     radiusV.x = bodyB->velocity.x + MathCross(bodyB->angularVelocity, radiusB).x - bodyA->velocity.x - MathCross(bodyA->angularVelocity, radiusA).x;
        //     radiusV.y = bodyB->velocity.y + MathCross(bodyB->angularVelocity, radiusB).y - bodyA->velocity.y - MathCross(bodyA->angularVelocity, radiusA).y;

        //     // Relative velocity along the normal
        //     float contactVelocity = MathDot(radiusV, manifold->normal);

        //     // Do not resolve if velocities are separating
        //     if (contactVelocity > 0.0f)
        //         return;

        //     float raCrossN = MathCrossVector2(radiusA, manifold->normal);
        //     float rbCrossN = MathCrossVector2(radiusB, manifold->normal);

        //     float inverseMassSum = bodyA->inverseMass + bodyB->inverseMass + (raCrossN*raCrossN)*bodyA->inverseInertia + (rbCrossN*rbCrossN)*bodyB->inverseInertia;

        //     // Calculate impulse scalar value
        //     float impulse = -(1.0f + manifold->restitution)*contactVelocity;
        //     impulse /= inverseMassSum;
        //     impulse /= (float)manifold->contactsCount;

        //     // Apply impulse to each physics body
        //     Vector2 impulseV = { manifold->normal.x*impulse, manifold->normal.y*impulse };

        //     if (bodyA->enabled)
        //     {
        //         bodyA->velocity.x += bodyA->inverseMass*(-impulseV.x);
        //         bodyA->velocity.y += bodyA->inverseMass*(-impulseV.y);

        //         if (!bodyA->freezeOrient)
        //             bodyA->angularVelocity += bodyA->inverseInertia*MathCrossVector2(radiusA, (Vector2){ -impulseV.x, -impulseV.y });
        //     }

        //     if (bodyB->enabled)
        //     {
        //         bodyB->velocity.x += bodyB->inverseMass*(impulseV.x);
        //         bodyB->velocity.y += bodyB->inverseMass*(impulseV.y);

        //         if (!bodyB->freezeOrient)
        //             bodyB->angularVelocity += bodyB->inverseInertia*MathCrossVector2(radiusB, impulseV);
        //     }

        //     // Apply friction impulse to each physics body
        //     radiusV.x = bodyB->velocity.x + MathCross(bodyB->angularVelocity, radiusB).x - bodyA->velocity.x - MathCross(bodyA->angularVelocity, radiusA).x;
        //     radiusV.y = bodyB->velocity.y + MathCross(bodyB->angularVelocity, radiusB).y - bodyA->velocity.y - MathCross(bodyA->angularVelocity, radiusA).y;

        //     Vector2 tangent = { radiusV.x - (manifold->normal.x*MathDot(radiusV, manifold->normal)), radiusV.y - (manifold->normal.y*MathDot(radiusV, manifold->normal)) };
        //     MathNormalize(&tangent);

        //     // Calculate impulse tangent magnitude
        //     float impulseTangent = -MathDot(radiusV, tangent);
        //     impulseTangent /= inverseMassSum;
        //     impulseTangent /= (float)manifold->contactsCount;

        //     float absImpulseTangent = fabs(impulseTangent);

        //     // Don't apply tiny friction impulses
        //     if (absImpulseTangent <= PHYSAC_EPSILON)
        //         return;

        //     // Apply coulumb's law
        //     Vector2 tangentImpulse = { 0.0f, 0.0f };
        //     if (absImpulseTangent < impulse*manifold->staticFriction)
        //         tangentImpulse = (Vector2){ tangent.x*impulseTangent, tangent.y*impulseTangent };
        //     else
        //         tangentImpulse = (Vector2){ tangent.x*-impulse*manifold->dynamicFriction, tangent.y*-impulse*manifold->dynamicFriction };

        //     // Apply friction impulse
        //     if (bodyA->enabled)
        //     {
        //         bodyA->velocity.x += bodyA->inverseMass*(-tangentImpulse.x);
        //         bodyA->velocity.y += bodyA->inverseMass*(-tangentImpulse.y);

        //         if (!bodyA->freezeOrient)
        //             bodyA->angularVelocity += bodyA->inverseInertia*MathCrossVector2(radiusA, (Vector2){ -tangentImpulse.x, -tangentImpulse.y });
        //     }

        //     if (bodyB->enabled)
        //     {
        //         bodyB->velocity.x += bodyB->inverseMass*(tangentImpulse.x);
        //         bodyB->velocity.y += bodyB->inverseMass*(tangentImpulse.y);

        //         if (!bodyB->freezeOrient)
        //             bodyB->angularVelocity += bodyB->inverseInertia*MathCrossVector2(radiusB, tangentImpulse);
        //     }
        // }
    }
}

impl PhysicsBodyData {
    /// Integrates physics velocity into position and forces
    fn integrate_physics_velocity(&mut self) {
        if !self.enabled {
            return;
        }

        self.position.x += (self.velocity.x as f64*unsafe { DELTA_TIME }) as f32;
        self.position.y += (self.velocity.y as f64*unsafe { DELTA_TIME }) as f32;

        if !self.freeze_orient {
            self.orient += (self.angular_velocity as f64*unsafe { DELTA_TIME }) as f32;
        }

        let orient = self.orient;
        self.shape.transform.set(orient);

        self.integrate_physics_forces();
    }
}

/// Corrects physics bodies positions based on manifolds collision information
fn correct_physics_positions(manifold: &mut PhysicsManifoldData) {
    let body_a = manifold.body_a.upgrade();
    let body_b = manifold.body_b.upgrade();

    if let (Some(body_a), Some(body_b)) = (body_a, body_b) {
        let mut body_a = body_a.write().unwrap();
        let mut body_b = body_b.write().unwrap();

        let correction = Vector2 {
            x: ((manifold.penetration - PHYSAC_PENETRATION_ALLOWANCE).max(0.0)/(body_a.inverse_mass + body_b.inverse_mass))*manifold.normal.x*PHYSAC_PENETRATION_CORRECTION,
            y: ((manifold.penetration - PHYSAC_PENETRATION_ALLOWANCE).max(0.0)/(body_a.inverse_mass + body_b.inverse_mass))*manifold.normal.y*PHYSAC_PENETRATION_CORRECTION,
        };

        if body_a.enabled {
            body_a.position.x -= correction.x*body_a.inverse_mass;
            body_a.position.y -= correction.y*body_a.inverse_mass;
        }

        if body_b.enabled {
            body_b.position.x += correction.x*body_b.inverse_mass;
            body_b.position.y += correction.y*body_b.inverse_mass;
        }
    }
}

/// Returns the extreme point along a direction within a polygon
fn get_support(shape: &PhysicsShape, dir: Vector2) -> Vector2 {
    let mut best_projection = -f32::MIN_POSITIVE;
    let mut best_vertex = Vector2 { x: 0.0, y: 0.0 };
    let data = &shape.vertex_data;

    for i in 0..data.vertex_count {
        let vertex = data.positions[i as usize];
        let projection = vertex.dot(dir);

        if projection > best_projection {
            best_vertex = vertex;
            best_projection = projection;
        }
    }

    best_vertex
}

/// Finds polygon shapes axis least penetration
fn find_axis_least_penetration(face_index: &mut u32, shape_a: &PhysicsShape, shape_b: &PhysicsShape) -> f32 {
    let mut best_distance = f32::MIN;
    let mut best_index = 0;

    let data_a = &shape_a.vertex_data;

    for i in 0..data_a.vertex_count {
        // Retrieve a face normal from A shape
        let mut normal = data_a.normals[i as usize];
        let trans_normal = shape_a.transform.multiply_vector2(normal);

        // Transform face normal into B shape's model space
        let bu_t = shape_b.transform.transpose();
        normal = bu_t.multiply_vector2(trans_normal);

        // Retrieve support point from B shape along -n
        let support = get_support(shape_b, Vector2 { x: -normal.x, y: -normal.y });

        // Retrieve vertex on face from A shape, transform into B shape's model space
        let mut vertex = data_a.positions[i as usize];
        vertex = shape_a.transform.multiply_vector2(vertex);
        vertex = vertex + shape_a.body.position().unwrap();
        vertex = vertex - shape_b.body.position().unwrap();
        vertex = bu_t.multiply_vector2(vertex);

        // Compute penetration distance in B shape's model space
        let distance = normal.dot(support - vertex);

        // Store greatest distance
        if distance > best_distance {
            best_distance = distance;
            best_index = i;
        }
    }

    *face_index = best_index;
    best_distance
}

/// Finds two polygon shapes incident face
fn find_incident_face(v0: &mut Vector2, v1: &mut Vector2, ref_shape: &PhysicsShape, inc_shape: &PhysicsShape, index: u32) {
    let ref_data = &ref_shape.vertex_data;
    let inc_data = &inc_shape.vertex_data;

    let mut reference_normal = ref_data.normals[index as usize];

    // Calculate normal in incident's frame of reference
    reference_normal = ref_shape.transform.multiply_vector2(reference_normal); // To world space
    reference_normal = inc_shape.transform.transpose().multiply_vector2(reference_normal); // To incident's model space

    // Find most anti-normal face on polygon
    let mut incident_face = 0;
    let mut min_dot = f32::MAX;

    for i in 0..inc_data.vertex_count {
        let dot = reference_normal.dot(inc_data.normals[i as usize]);

        if dot < min_dot {
            min_dot = dot;
            incident_face = i;
        }
    }

    // Assign face vertices for incident face
    *v0 = inc_shape.transform.multiply_vector2(inc_data.positions[incident_face as usize]);
    *v0 = *v0 + inc_shape.body.position().unwrap();
    incident_face = if (incident_face + 1) < inc_data.vertex_count { incident_face + 1 } else { 0 };
    *v1 = inc_shape.transform.multiply_vector2(inc_data.positions[incident_face as usize]);
    *v1 = *v1 + inc_shape.body.position().unwrap();
}

/// Calculates clipping based on a normal and two faces
fn clip(normal: Vector2, clip: f32, face_a: &mut Vector2, face_b: &mut Vector2) -> u32 {
    let mut sp = 0;
    let mut out = [*face_a, *face_b];

    // Retrieve distances from each endpoint to the line
    let distance_a = normal.dot(*face_a) - clip;
    let distance_b = normal.dot(*face_b) - clip;

    // If negative (behind plane)
    if distance_a <= 0.0 {
        out[sp as usize] = *face_a;
        sp += 1;
    }

    if distance_b <= 0.0 {
        out[sp as usize] = *face_b;
        sp += 1;
    }

    // If the points are on different sides of the plane
    if (distance_a*distance_b) < 0.0 {
        // Push intersection point
        let alpha = distance_a/(distance_a - distance_b);
        out[sp as usize] = *face_a;
        let mut delta = *face_b - *face_a;
        delta.x *= alpha;
        delta.y *= alpha;
        out[sp as usize] = out[sp as usize] + delta;
        sp += 1;
    }

    // Assign the new converted values
    *face_a = out[0];
    *face_b = out[1];

    sp
}

/// Check if values are between bias range
fn bias_greater_than(value_a: f32, value_b: f32) -> bool {
    value_a >= (value_b*0.95 + value_a*0.01)
}

/// Returns the barycenter of a triangle given by 3 points
fn triangle_barycenter(v1: Vector2, v2: Vector2, v3: Vector2) -> Vector2 {
    Vector2 {
        x: (v1.x + v2.x + v3.x)/3.0,
        y: (v1.y + v2.y + v3.y)/3.0,
    }
}

/// Initializes hi-resolution MONOTONIC timer
fn init_timer() {
    BASE_TIME.set(Instant::now()).expect("tried to initialize timer twice"); // Get MONOTONIC clock time offset
    unsafe { START_TIME = get_curr_time() }; // Get current time
}

/// Get hi-res MONOTONIC time measure in seconds
fn get_time_count() -> u64 {
    BASE_TIME.get().expect("BASE_TIME should be initialized before checking time").elapsed().as_secs()
}

/// Get current time in milliseconds
fn get_curr_time() -> f64 {
    let duration = BASE_TIME.get().expect("BASE_TIME should be initialized before checking time").elapsed();
    duration.as_secs_f64() * 1_000.0
}

// Returns the cross product of a vector and a value
#[inline(always)]
fn math_cross(value: f32, vector: Vector2) -> Vector2 {
    Vector2 { x: -value*vector.y, y: value*vector.x }
}

// Returns the cross product of two vectors
#[inline(always)]
fn math_cross_vector2(v1: Vector2, v2: Vector2) -> f32 {
    v1.x*v2.y - v1.y*v2.x
}

// Returns the square root of distance between two vectors
#[inline(always)]
fn dist_sqr(v1: Vector2, v2: Vector2) -> f32 {
    let dir = v1 - v2;
    dir.dot(dir)
}

/// Returns the normalized values of a vector
fn math_normalize(vector: &mut Vector2) {
    let (mut length, ilength): (f32, f32);

    let aux = *vector;
    length = (aux.x*aux.x + aux.y*aux.y).sqrt();

    if length == 0.0 {
        length = 1.0;
    }

    ilength = 1.0/length;

    vector.x *= ilength;
    vector.y *= ilength;
}

impl Mat2 {
    /// Creates a matrix 2x2 from a given radians value
    fn radians(radians: f32) -> Mat2 {
        let (s, c) = radians.sin_cos();

        Mat2 {
            m00: c,
            m01: -s,
            m10: s,
            m11: c,
        }
    }

    /// Set values from radians to a created matrix 2x2
    fn set(&mut self, radians: f32) {
        let (sin, cos) = radians.sin_cos();

        self.m00 = cos;
        self.m01 = -sin;
        self.m10 = sin;
        self.m11 = cos;
    }

    // Returns the transpose of a given matrix 2x2
    #[inline(always)]
    fn transpose(&self) -> Mat2 {
        Mat2 {
            m00: self.m00,
            m01: self.m10,
            m10: self.m01,
            m11: self.m11,
        }
    }

    // Multiplies a vector by a matrix 2x2
    #[inline(always)]
    fn multiply_vector2(&self, vector: Vector2) -> Vector2 {
        Vector2 {
            x: self.m00*vector.x + self.m01*vector.y,
            y: self.m10*vector.x + self.m11*vector.y,
        }
    }
}
