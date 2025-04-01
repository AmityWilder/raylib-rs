#![allow(non_snake_case)]
#![allow(unused_parens)]

pub use raylib::prelude::*;

pub mod example;

type SampleOut = Box<dyn for<'a> FnMut(&'a mut RaylibHandle, &'a RaylibThread) -> ()>;
type Sample = fn(&mut RaylibHandle, &RaylibThread) -> SampleOut;

use std::cell::RefCell;
thread_local! (static APP: RefCell<Option<Box<dyn FnMut() -> bool>>> = RefCell::new(None));

pub const EXIT_KEY: raylib::consts::KeyboardKey = raylib::consts::KeyboardKey::KEY_ESCAPE;

fn main() {
    let title = "Showcase";
    let screen_width = 800;
    let screen_height = 640;
    let (mut rl, thread) = raylib::init(screen_width, screen_height, title)
        .resizable()
        .vsync()
        .msaa_4x()
        .build()
        .unwrap();

    rl.set_exit_key(None);

    let samples: Vec<(&str, Sample)> = vec![
        (
            "raygui - controls test suite",
            example::controls_test_suite::controls_test_suite::run,
        ),
        (
            "raygui - image exporter",
            example::image_exporter::image_exporter::run,
        ),
        (
            "raygui - portable window",
            example::portable_window::portable_window::run,
        ),
        (
            "raygui - GuiScrollPanel()",
            example::scroll_panel::gui_scroll_panel::run,
        ),
        (
            "raylib [audio] example - music playing (streaming)",
            example::audio::audio_music_stream::run,
        ),
        (
            "raylib [audio] example - module playing (streaming)",
            example::audio::audio_module_playing::run,
        ),
        (
            "raylib [audio] example - Multichannel sound playing",
            example::audio::audio_multichannel_sound::run,
        ),
        (
            "raylib [audio] example - raw audio streaming",
            example::audio::audio_raw_stream::run,
        ),
        (
            "raylib [audio] example - sound loading and playing",
            example::audio::audio_sound_loading::run,
        ),
        (
            "raylib [core] example - Camera",
            example::core::core_2d_camera::run,
        ),
        (
            "raylib [core] example - Camera Platformer",
            example::core::core_2d_camera_platformer::run,
        ),
        (
            "raylib [core] example - 3d camera first person",
            example::core::core_3d_camera_first_person::run,
        ),
        (
            "raylib [core] example - 3d camera free",
            example::core::core_3d_camera_free::run,
        ),
        (
            "raylib [core] example - 3d camera mode",
            example::core::core_3d_camera_mode::run,
        ),
        (
            "raylib [core] example - 3d picking",
            example::core::core_3d_picking::run,
        ),
        (
            "raylib [core] example - basic window",
            example::core::core_basic_window::run,
        ),
        #[cfg(target_os = "windows")]
        (
            "raylib [core] example - custom logging",
            example::core::core_custom_logging::run,
        ),
        (
            "raylib [core] example - drop files",
            example::core::core_drop_files::run,
        ),
        (
            "raylib [core] example - gamepad input",
            example::core::core_input_gamepad::run,
        ),
        (
            "raylib [core] example - input gestures",
            example::core::core_input_gestures::run,
        ),
        (
            "raylib [core] example - keyboard input",
            example::core::core_input_keys::run,
        ),
        (
            "raylib [core] example - input mouse wheel",
            example::core::core_input_mouse_wheel::run,
        ),
        (
            "raylib [core] example - mouse input",
            example::core::core_input_mouse::run,
        ),
        (
            "raylib [core] example - input multitouch",
            example::core::core_input_multitouch::run,
        ),
        (
            "raylib [core] example - generate random values",
            example::core::core_random_values::run,
        ),
        (
            "raylib [core] example - window scale letterbox",
            example::core::core_window_letterbox::run,
        ),
        (
            "raylib [core] example - core world screen",
            example::core::core_world_screen::run,
        ),
        (
            "raylib [core] example - scissor test",
            example::core::core_scissor_test::run,
        ),
        // VR is Buggy AF. Take a look at it
        // (
        //     "raylib [core] example - vr simulator",
        //     example::core::core_vr_simulator::run,
        // ),
        (
            "raylib [models] example - cubesmap loading and drawing",
            example::models::models_cubicmap::run,
        ),
        // (
        //     "raylib [models] example - pbr material",
        //     example::models::models_material_pbr::run,
        // ),
        (
            "raylib [models] example - drawing billboards",
            example::models::models_billboard::run,
        ),
        (
            "raylib [models] example - box collisions",
            example::models::models_box_collisions::run,
        ),
        (
            "raylib [models] example - cubesmap loading and drawing",
            example::models::models_cubicmap::run,
        ),
        (
            "raylib [models] example - model animation",
            example::models::models_animation::run,
        ),
        (
            "raylib [models] example - first person maze",
            example::models::models_first_person_maze::run,
        ),
        (
            "raylib [models] example - geometric shapes",
            example::models::models_geometric_shapes::run,
        ),
        (
            "raylib [models] example - heightmap loading and drawing",
            example::models::models_heightmap::run,
        ),
        (
            "raylib [models] example - models loading",
            example::models::models_loading::run,
        ),
        (
            "raylib [models] example - mesh generation",
            example::models::models_mesh_generation::run,
        ),
        (
            "raylib [models] example - mesh picking",
            example::models::models_mesh_picking::run,
        ),
        (
            "raylib [models] example - orthographic projection",
            example::models::models_orthographic_projection::run,
        ),
        (
            "raylib [models] example - rlgl module usage with push/pop matrix transformations",
            example::models::models_rlgl_solar_system::run,
        ),
        // (
        //     "raylib [models] example - skybox loading and drawing",
        //     example::models::models_skybox::run,
        // ),
        (
            "raylib [models] example - waving cubes",
            example::models::models_waving_cubes::run,
        ),
        (
            "raylib [models] example - plane rotations (yaw, pitch, roll)",
            example::models::models_yaw_pitch_roll::run,
        ),
        (
            "raylib [textures] example - bunnymark",
            example::textures::textures_bunnymark::run,
        ),
        (
            "raylib [shaders] example - basic lighting",
            example::shaders::shaders_basic_lighting::run,
        ),
        (
            "raylib [shaders] example - custom uniform variable",
            example::shaders::shaders_custom_uniform::run,
        ),
        (
            "raylib [shaders] example - Sieve of Eratosthenes",
            example::shaders::shaders_eratosthenes::run,
        ),
        (
            "raylib [shaders] example - fog",
            example::shaders::shaders_fog::run,
        ),
        (
            "raylib [shaders] example - julia sets",
            example::shaders::shaders_julia_set::run,
        ),
        (
            "raylib [shaders] example - postprocessing shader",
            example::shaders::shaders_postprocessing::run,
        ),
        (
            "raylib [texture] example - texture rectangle",
            example::textures::textures_rectangle::run,
        ),
        (
            "raylib [textures] example - mouse painting",
            example::textures::textures_mouse_painting::run,
        ),
        (
            "rlgl standalone",
            example::others::rlgl_standalone::run,
        ),
    ];
    let mut sample = None;
    let mut list_view_active = -1;
    let mut list_view_focus = -1;
    let mut list_view_scroll_index = -1;

    let box_length = (50 * samples.len() as i32).min(500);
    let y_margin = (screen_height - box_length) / 2;

    let frame: Box<dyn FnMut() -> bool> = Box::new(move || {
        match &mut sample {
            None => {
                let mut to_run = None;
                rl.draw(&thread, |d| {
                    d.clear_background(Color::WHITE);

                    let list: Vec<_> = samples.iter().map(|(s, _)| *s).collect();

                    d.gui_list_view_ex(
                        rrect(100.0, y_margin as f32, 600.0, box_length as f32),
                        list.as_slice(),
                        &mut list_view_focus,
                        &mut list_view_scroll_index,
                        &mut list_view_active,
                    );

                    if list_view_active >= 0 {
                        to_run.replace(samples[list_view_active as usize].1);
                    }
                });

                match to_run {
                    Some(run) => sample = Some(run(&mut rl, &thread)),
                    _ => {}
                }
            }

            Some(ref mut run) => {
                (*run)(&mut rl, &thread);
                if rl.is_key_down(EXIT_KEY) {
                    sample = None;
                    rl.set_window_size(screen_width, screen_height);
                    rl.set_window_title(&thread, title);
                    list_view_active = -1;
                }
            }
        };
        #[cfg(not(target_arch = "wasm32"))]
        return rl.window_should_close();
        #[cfg(target_arch = "wasm32")]
        return false;
    });

    APP.with(|app| {
        app.borrow_mut().replace(frame);
    });

    // absolutely NONE of this is necessary. You could use a while !update() {} loop in
    // wasm without any problems as long as you compile with ASYNCIFY.
    // This shows you how to do it using emscripten_set_main_loop.
    #[cfg(not(target_arch = "wasm32"))]
    {
        while !update() {}
    }
    #[cfg(target_arch = "wasm32")]
    unsafe {
        wasm::emscripten_set_main_loop(wasm::_update_wasm, 0, 1);
    }
}

fn update() -> bool {
    APP.with(|app| match *app.borrow_mut() {
        None => false,
        Some(ref mut frame) => frame(),
    })
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
mod wasm {
    use std::os::raw::{c_int, c_uchar};

    #[allow(non_camel_case_types)]
    type em_callback_func = unsafe extern "C" fn();

    extern "C" {
        // This extern is built in by Emscripten.
        pub fn emscripten_sample_gamepad_data();
        pub fn emscripten_run_script_int(x: *const c_uchar) -> c_int;
        pub fn emscripten_cancel_main_loop();
        pub fn emscripten_set_main_loop(
            func: em_callback_func,
            fps: c_int,
            simulate_infinite_loop: c_int,
        );
    }

    pub extern "C" fn _update_wasm() {
        super::update();
    }
}
