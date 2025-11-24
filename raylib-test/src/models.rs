#[cfg(test)]
mod model_test {
    use crate::tests::*;
    use raylib::prelude::*;
    use raylib::error::{InvalidMeshError};

    ray_test!(test_load_model);
    fn test_load_model(thread: &RaylibThread) {
        let mut handle = TEST_HANDLE.write().unwrap();
        let rl = handle.as_mut().unwrap();
        let _ = rl.load_model(thread, "resources/cube.obj");
        let _ = rl.load_model(thread, "resources/pbr/trooper.obj");
    }

    ray_test!(test_load_meshes);
    fn test_load_meshes(_thread: &RaylibThread) {
        // TODO run this test when Raysan implements LoadMeshes
        // let m = Mesh::load_meshes(thread, "resources/cube.obj").expect("couldn't load any meshes");
    }

    // ray_test!(test_load_anims);

    ray_test!(test_load_anims);
    fn test_load_anims(thread: &RaylibThread) {
        let mut handle = TEST_HANDLE.write().unwrap();
        let rl = handle.as_mut().unwrap();

        let _ = rl
            .load_model_animations(&thread, "resources/guy/guyanim.iqm")
            .expect("could not load model animations");
    }

    ray_test!(test_model_from_generated_mesh);
    fn test_model_from_generated_mesh(thread: &RaylibThread) {
        let mut handle = TEST_HANDLE.write().unwrap();
        let rl = handle.as_mut().unwrap();

        let mesh = Mesh::gen_mesh_cube(&thread, 1.0, 1.0, 1.0);
        let model = rl.load_model_from_mesh(&thread, mesh).unwrap();

        let zero = Vector3::ZERO;

        let camera = Camera3D::perspective(zero, zero, zero, 10.0);

        let mut d = rl.begin_drawing(&thread);
        let mut world = d.begin_mode3D(&camera);

        world.draw_model(&model, zero, 1.0, Color::RED);
    }

    //TODO: LATER REMOVE TESTS ABOVE, MERGE OR REWRITE THEM, FOR NOW MAIN TESTS ARE BELOW

    ray_test!(test_basic_construction_time_guarantees);
    fn test_basic_construction_time_guarantees(thread: &RaylibThread) {
        println!("\n[VALIDATION] All mesh entry points validate");
        let mut handle = TEST_HANDLE.write().unwrap();
        let rl = handle.as_mut().unwrap();

        // BUILDER
        const BAD_INDEX: u16 = 999;
        let builder_result = Mesh::init_mesh(&[Vector3::ZERO; 3])
            .indices(&[0, 1, BAD_INDEX])
            .build_cpu();
        assert!(builder_result.is_err(), "builder validates");
        println!("MESH BUILDER VALIDATED");

        // GEN MESH
        let _ = Mesh::try_gen_mesh_cube(thread, 1.0, 1.0, 1.0).unwrap();
        println!("GEN MESH VALIDATED");

        // FILE LOAD 1
        let _ = rl.load_model(thread, "resources/cube.obj").unwrap();
        println!("FILE LOAD MESH VALIDATED");
        // FILE LOAD 2
        let _ = rl.load_model(thread, "resources/pbr/trooper.obj");
        println!("FILE LOAD MESH VALIDATED");
    }

    ray_test!(test_indexed_mesh_triangle_iteration);
    fn test_indexed_mesh_triangle_iteration(thread: &RaylibThread) {
        let vertices = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
        ];
        let indices = vec![0, 1, 2, 1, 3, 2];
        let mesh = Mesh::init_mesh(&vertices)
            .indices(&indices)
            .build(&thread)
            .unwrap();

        let mut triangle_count = 0;
        for [a, b, c] in mesh.triangles() {
            let _v1 = mesh.vertices()[a];
            let _v2 = mesh.vertices()[b];
            let _v3 = mesh.vertices()[c];
            triangle_count += 1;
        }
        const EXPECTED_TRIANGLES: usize = 2;
        assert_eq!(triangle_count, EXPECTED_TRIANGLES, "Expected {} triangles", EXPECTED_TRIANGLES);
    }

    ray_test!(test_basic_invalid_mesh_sources);
    fn test_basic_invalid_mesh_sources(_thread: &RaylibThread) { //TODO: use thread when adding later try_upload tests
        println!("\n[VALIDATION] validate_mesh() catches all invalid patterns");

        // Pattern 1: Out of bounds index
        const PATTERN1_VERTEX_COUNT: i32 = 3;
        const PATTERN1_TRIANGLE_COUNT: i32 = 1;
        const PATTERN1_BAD_INDEX: u16 = 999;
        let mut verts1 = vec![Vector3::ZERO, Vector3::ZERO, Vector3::ZERO];
        let mut indices1 = vec![0u16, 1, PATTERN1_BAD_INDEX];
        let mesh1 = ffi::Mesh {
            vertexCount: PATTERN1_VERTEX_COUNT,
            triangleCount: PATTERN1_TRIANGLE_COUNT,
            vertices: verts1.as_mut_ptr().cast(),
            indices: indices1.as_mut_ptr(),
            ..Default::default()
        };
        assert!(matches!(
            validate_mesh(&mesh1),
            Err(InvalidMeshError::IndexOutOfBounds)
        ));

        // Pattern 2: vertices Null with positive count
        const PATTERN2_VERTEX_COUNT: i32 = 3;
        const PATTERN2_TRIANGLE_COUNT: i32 = 1;
        let mesh2 = ffi::Mesh {
            vertexCount: PATTERN2_VERTEX_COUNT,
            triangleCount: PATTERN2_TRIANGLE_COUNT,
            vertices: std::ptr::null_mut(),
            ..Default::default()
        };
        assert!(matches!(
            validate_mesh(&mesh2),
            Err(InvalidMeshError::VerticesPointerNull)
        ));

        // Pattern 3: Insufficient vertices
        const PATTERN3_VERTEX_COUNT: i32 = 2;
        const PATTERN3_TRIANGLE_COUNT: i32 = 1; // Needs 3 for 1 triangle
        let mut verts3 = vec![Vector3::ZERO, Vector3::ZERO];
        let mesh3 = ffi::Mesh {
            vertexCount: PATTERN3_VERTEX_COUNT,
            triangleCount: PATTERN3_TRIANGLE_COUNT,
            vertices: verts3.as_mut_ptr().cast(),
            indices: std::ptr::null_mut(),
            ..Default::default()
        };
        assert!(matches!(
            validate_mesh(&mesh3),
            Err(InvalidMeshError::VertexCountInsufficient)
        ));

        // Pattern 4: Negative counts
        const PATTERN4_VERTEX_COUNT: i32 = -5;
        const PATTERN4_TRIANGLE_COUNT: i32 = -2;
        let mesh4 = ffi::Mesh {
            vertexCount: PATTERN4_VERTEX_COUNT,
            triangleCount: PATTERN4_TRIANGLE_COUNT,
            ..Default::default()
        };
        assert!(matches!(
            validate_mesh(&mesh4),
            Err(InvalidMeshError::NegativeCount)
        ));
    }

    ray_test!(test_unindexed_mesh_triangle_iteration);
    fn test_unindexed_mesh_triangle_iteration(thread: &RaylibThread) {
        let vertices = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(1.0, 1.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ];

        let mesh = Mesh::init_mesh(&vertices).build(&thread).unwrap();

        assert!(mesh.indices().is_none(), "Unindexed mesh has no indices");
        let triangles: Vec<_> = mesh.triangles().collect();
        const EXPECTED_TRIANGLES: usize = 2;
        const EXPECTED_TRI0: [usize; 3] = [0, 1, 2];
        const EXPECTED_TRI1: [usize; 3] = [3, 4, 5];
        assert_eq!(
            triangles.len(),
            EXPECTED_TRIANGLES,
            "Expected {} triangles",
            EXPECTED_TRIANGLES
        );
        assert_eq!(triangles[0], EXPECTED_TRI0, "First triangle");
        assert_eq!(triangles[1], EXPECTED_TRI1, "Second triangle");
    }

    ray_test!(test_builder_ergonomics_opt_pattern);
    fn test_builder_ergonomics_opt_pattern(thread: &RaylibThread) {
        let gen_mesh = Mesh::try_gen_mesh_cube(thread, 1.0, 1.0, 1.0).unwrap();

        // demonstrate copy mesh not using _opt pattern:
        let mut builder = Mesh::init_mesh(gen_mesh.vertices());
        if let Some(texcoords) = gen_mesh.texcoords() {
            builder = builder.texcoords(texcoords);
        }
        if let Some(colors) = gen_mesh.colors() {
            builder = builder.colors(colors);
        }
        if let Some(indices) = gen_mesh.indices() {
            builder = builder.indices(indices);
        }
        let _ = builder.build_cpu().unwrap(); // just for demonstration

        // using _opt pattern idea, no if Some blocks breaking up the builder
        let mesh = Mesh::init_mesh(gen_mesh.vertices())
            .texcoords_opt(gen_mesh.texcoords())
            .colors_opt(gen_mesh.colors())
            .indices_opt(gen_mesh.indices())
            .build_cpu()
            .unwrap();

        assert_eq!(mesh.vertices().len(), gen_mesh.vertices().len());
        assert!(mesh.texcoords().is_some());
        assert!(mesh.colors().is_none());
        assert!(mesh.indices().is_some());
    }

//TODO: >>>>the following tests NOT intended to be merged<<<<
// i am hoping they can serve as temporary communication around middle-ground design and testing UB traps
    // ray_test!(test_corrupt_indexed_mesh_out_of_bounds_index_and_unchecked_vertex_read);
    #[ignore = " Corrupt indexed mesh by writing an out-of-bounds index, then try to  read vertices unchecked"]
    fn test_corrupt_indexed_mesh_out_of_bounds_index_and_unchecked_vertex_read(_thread: &RaylibThread) {
        let vertices = vec![
            Vector3::new(0.0, 0.0, 0.0),
            Vector3::new(1.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        ];
        let indices = vec![0u16, 1u16, 2u16];
        let mut mesh = Mesh::init_mesh(&vertices)
            .indices(&indices)
            .build_cpu()
            .unwrap();

        let vertex_count_from_mesh = mesh.vertices().len();
        let indices_slice = unsafe { mesh.indices_mut().unwrap() };
        let out_of_bounds_index_value = vertex_count_from_mesh as u16; // 3
        indices_slice[0] = out_of_bounds_index_value;

        for [first_triangle_index, second_triangle_index, third_triangle_index] in mesh.triangles() {
            unsafe {
                let _first_vertex = *mesh.vertices().get_unchecked(first_triangle_index);
                let _second_vertex = *mesh.vertices().get_unchecked(second_triangle_index);
                let _third_vertex = *mesh.vertices().get_unchecked(third_triangle_index);
            }
        }
    }

    //ray_test!(test_null_vertices_and_nonzero_vertex_count);
    #[ignore = "Pass a NULL vertices while vertexCount > 0, Constructing a slice from NULL with a nonzero length -> UB"]
    fn test_null_vertices_and_nonzero_vertex_count(_thread: &RaylibThread) {
        const VERTEX_COUNT: i32 = 3;
        const TRIANGLE_COUNT: i32 = 1;
        let raw_mesh = ffi::Mesh {
            vertexCount: VERTEX_COUNT,
            triangleCount: TRIANGLE_COUNT,
            vertices: std::ptr::null_mut(),
            indices: std::ptr::null_mut(),
            ..Default::default()
        };

        let mesh = unsafe { Mesh::from_raw_unchecked(raw_mesh) };
        let vertices_slice = mesh.vertices();
        unsafe {
            let _first_vertex = *vertices_slice.get_unchecked(0);
        }
    }
}