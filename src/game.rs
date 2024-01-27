use std::f32::consts::PI;

use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle}, render::{render_resource::{PrimitiveTopology, Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages}, mesh::Indices, camera::RenderTarget, view::RenderLayers}, utils::{HashMap, hashbrown::HashSet}, core_pipeline::{tonemapping::{Tonemapping, DebandDither}, bloom::{BloomSettings, BloomCompositeMode}, clear_color::ClearColorConfig}, window::WindowResized,
};

use bevy_rapier2d::{prelude::*, rapier::geometry::CollisionEventFlags};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

#[path = "settings.rs"] pub mod settings;
pub use settings::*;
#[path = "components.rs"] pub(crate) mod components;
pub use components::*;


pub fn pixel_camera_event_listener(
    settings: Res<GameSettings>,
    mut listener: EventReader<ApplyCameraSettings>,
    mut camera: Query<(&mut Tonemapping, &mut BloomSettings, &mut DebandDither), With<PixelCamera>>
){
    for e in listener.read(){
        for (mut tonemapping, mut bloom, mut deband_dither) in camera.iter_mut(){
            match e{
                ApplyCameraSettings::DebandDither => {
                    *deband_dither = settings.deband_dither;
                },
                ApplyCameraSettings::Tonemapping => {
                    *tonemapping = settings.tonemapping;
                },
                ApplyCameraSettings::BloomCompositeMode => {
                    bloom.composite_mode = settings.composite_mode;
                },
                ApplyCameraSettings::Intensity => {
                    bloom.intensity = settings.bloom_intensity;
                },
                ApplyCameraSettings::LowFrequencyBoost => {
                    bloom.low_frequency_boost = settings.low_frequency_boost;
                },
                ApplyCameraSettings::LowFrequencyBoostCurvature => {
                    bloom.low_frequency_boost_curvature = settings.low_frequency_boost_curvature;
                },
                ApplyCameraSettings::HighPassFrequency => {
                    bloom.high_pass_frequency = settings.high_pass_frequency;
                },
                ApplyCameraSettings::Threshold => {
                    bloom.prefilter_settings.threshold = settings.threshold;
                },
                ApplyCameraSettings::ThresholdSoftness => {
                    bloom.prefilter_settings.threshold_softness = settings.threshold_softness;
                },
            }
        }
    }
}

#[allow(dead_code)]
pub fn init_pixel_camera(app: &mut App){
    app.add_event::<ApplyCameraSettings>();
    app.add_systems(Startup, setup_pixel_camera);
    app.add_systems(Update, (update_pixel_camera, pixel_camera_event_listener));
}

fn setup_pixel_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
) {
    let size = Extent3d {
        width: 256,
        height: 180,
        ..default()
    };

    // This is the texture that will be rendered to.
    let mut image: Image = Image {
        texture_descriptor: TextureDescriptor {
            label: None,
            size,
            dimension: TextureDimension::D2,
            format: TextureFormat::Bgra8UnormSrgb,
            mip_level_count: 1,
            sample_count: 1,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        },
        ..default()
    };

    // fill image.data with zeroes
    image.resize(size);

    let image_handle = images.add(image);

    // This specifies the layer used for the first pass, which will be attached to the first pass camera and cube.
    let second_pass_layer = RenderLayers::layer(GameRenderLayers::PixelCamera as u8);


    // MAIN CAMERA
    commands.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::Rgba { red: 0., green: 0., blue: 0., alpha: 1. }),
                ..default()
            },
            camera: Camera {
                // render before the "main pass" camera
                order: 1,
                hdr: true,
                target: RenderTarget::Image(image_handle.clone()),
                msaa_writeback: false,
                ..default()
            },
            tonemapping: Tonemapping::TonyMcMapface,
            deband_dither: DebandDither::Enabled,
            transform: Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
            ..default()
        },
        BloomSettings{ // 3. Enable bloom for the camera
            composite_mode: BloomCompositeMode::Additive,
            intensity: 0.1,
            ..default()
        },
        PixelCamera,
    )).insert(TransformBundle::default());


    // FAKE MAIN CAMERA
    commands.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::Rgba { red: 0., green: 0., blue: 0., alpha: 1. }),
                ..default()
            },
            camera: Camera {
                order: 2,
                ..default()
            },
            transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        },
        second_pass_layer,
    ));
    commands.spawn((
        SpriteBundle {
            texture: image_handle,
            transform: Transform::from_scale(Vec3::splat(5.)),
            ..default()
        },
        CameraCanvas,
        Name::from("PixelCameraCanvas"),
        second_pass_layer,
    ));
}

const TARGET_HEIGHT: f32 = 512.; // todo: add setting

#[allow(dead_code)]
pub fn update_pixel_camera(
    resize_event: Res<Events<WindowResized>>,
    mut canvas_q: Query<(&Handle<Image>, &mut Transform), With<CameraCanvas>>,
    mut images: ResMut<Assets<Image>>,
){
    let mut reader = resize_event.get_reader();
    let (image_handle, mut transform) = canvas_q.single_mut();
    for e in reader.read(&resize_event) {
        if e.height == 0.{continue;}
        let raito = TARGET_HEIGHT / e.height;
        let size = Extent3d {
            width: (e.width * raito) as u32,
            height: (e.height * raito) as u32,
            ..default()
        };
        let target_size = e.width / size.width as f32;
        let img = images.get_mut(image_handle).unwrap();
        img.resize(size);
        transform.scale = Vec3::splat(target_size);
    }
}


pub fn get_ship_vertices(style: u8) -> (Vec<Vec<(f32, f32)>>, Vec<Vec<u32>>){
    //let mut style: u8 = 0b_10001110_u8;
    let mut style2 = style.clone();
    let mut bits: [bool; 8] = [false; 8];
    
    for n in 0..=7{
        let m = 2_u8.pow(7 - n);
        //print!("{:?}", m);
        let i = style2 / m;
        //println!(" {:?}", i);
        if i == 1{
            bits[n as usize] = i != 0;
        }
        style2 = style2 % m;
    }
    
    
    let is_lined = bits[2];
    let is_spear = bits[3];
    let is_spikes = bits[4];
    let is_gem = bits[5];
    let is_shards = bits[6];
    let _is_shield = bits[7];


    let mut vertices: Vec<Vec<(f32, f32)>> = vec![];
    let mut indices = vec![];
    let mut z_indexes: Vec<i8> = vec![]; 
    /*
    -1 shards, spikes
    0 standard
    1 gem
    */


    if is_shards{
        vertices.push(vec![(-1., -3.), (0., -6.), (1., -3.)]);
        vertices.push(vec![(1., -3.), (3., -5.5), (3., -2.5)]);
        vertices.push(vec![(-1., -3.), (-3., -5.5), (-3., -2.5)]);
        z_indexes.push(-1);
        z_indexes.push(-1);
        z_indexes.push(-1);
    }
    
    // base
    match bits[0] as i32 * 2 + bits[1] as i32{
        0 => {
            vertices.push(vec![(0., 5.), (3., -2.5), (0., -5.), (-3., -2.5)]);
            z_indexes.push(0);
        },
        1 => {
            vertices.push(vec![(0., 5.), (4., -5.), (1., -3.), (0., -5.), (-1., -3.), (-4., -5.)]);
            z_indexes.push(0);
        },
        2 => {
            vertices.push(vec![(0., 5.), (4., -5.), (-4., -5.)]);
            z_indexes.push(0);
        },
        3 => {
            vertices.push(vec![(0., 5.), (4., -5.), (0., -3.), (-4., -5.)]);
            z_indexes.push(0);
        },
        _ => {},
    };
    
    // deco
    if is_spear{
        vertices.push(vec![(0., 6.), (-1., 4.), (0., 3.5), (1., 4.)]);
        z_indexes.push(1);
    }
    if is_spikes{
        vertices.push(vec![(0.5, 3.75), (3., 0.75), (2.5, -1.25)]);//vertices.push(vec![(0.5, 4.), (3., 1.), (2.5, -1.)]);
        vertices.push(vec![(-0.5, 3.75), (-3., 0.75), (-2.5, -1.25)]);//vertices.push(vec![(-0.5, 4.), (-3., 1.), (-2.5, -1.)]);
        z_indexes.push(-1);
        z_indexes.push(-1);
    }
    if is_gem{
        vertices.push(vec![(0., 3.), (-2., -2.), (0., -3.), (2., -2.)]);
        z_indexes.push(1);
    }

    if is_lined {
        let mut last_i: u32 = 0;
        for i in 0..vertices.len(){
            let mut d: Vec<u32> = vec![];
            for ind in 0..(vertices[i].len() as u32){
                d.push(last_i + ind);
                d.push(last_i + ((ind + 1) % vertices[i].len() as u32));
            }
            last_i += vertices[i].len() as u32 + 1;

            indices.push(d);
            let l = vertices[i][0];
            vertices[i].push(l);
        }
    } else {
        let mut last_i: u32 = 0;
        for i in 0..vertices.len(){
            let mut d: Vec<u32> = vec![];
            for i in 2..(vertices[i].len() as u32){
                d.push(last_i + 0);
                d.push(last_i + i-1);
                d.push(last_i + i);
            }
            last_i += vertices[i].len() as u32;
            indices.push(d);
        }
    }

    //let permutation = permutation::sort(&z_indexes); // todo: exclude permutation from deps (if unused)
    //let vertices = permutation.apply_slice(&vertices);
    //let indices = permutation.apply_slice(&indices);
    return (vertices, indices);
}


fn generate_asteroid_vertices(seed: u64) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let size = get_asteroid_size(seed);

    /*
    GENERATING ROUND ASTEROID AND OFFSET EVERY VERTEX
          10 09 08
       11          07
    12                06
    13       *        05
    14                04
       15          03 
          00 01 02
    (* - is ZERO)
    */
    

    let sides = 16; // number of verteces
    let mut vec = Vec::new();
    let mut ind = Vec::new();
    for side in 0..sides{
        let mut point: [f32; 3];
        match side {
            0..=2 => point = [-1.0 + side as f32, -3.0, 0.0],
            3 => point = [2.0, -2.0, 0.0],
            4..=6 => point = [3.0, -1.0 + (side as f32 - 4.), 0.0],
            7 => point = [2.0, 2.0, 0.0],
            8..=10 => point = [1.0 - (side as f32 - 8.), 3.0, 0.0],
            11 => point = [-2.0, 2.0, 0.0],
            12..=14 => point = [-3.0, 1.0 - (side as f32 - 12.), 0.0],
            15 => point = [-2.0, -2.0, 0.0],
            _ => point = [-10.0, -10.0, -10.0]
        };
        //                            pre-mul            rand             *    max off   | * size 
        point = point.map(|v| (v * 2.0  + rng.gen_range(-100..101) as f32 * 0.01 ) * size as f32);  
        point[2] = 0.0;
        vec.push(point);
        ind.push(side);
        ind.push((side + 1) % sides); // for loop
    }
    (vec, ind)
}


fn prepate_for_polyline(vec: Vec<[f32; 3]>, ind: Vec<u32>) -> (Vec<Vect>, Vec<[u32; 2]>) {
    let mut vertices: Vec<Vect> = Vec::new();
    let mut indexes = Vec::new();
    for i in 0..ind.len(){
        if i * 2 + 1 >= ind.len() {break}

        indexes.push([ind[i*2], ind[i*2 + 1]])
    }
    for v in vec.iter(){
        vertices.push(Vect::from((v[0], v[1])));
    }
    
    (vertices, indexes)
}


pub fn spawn_asteroid(
    seed: u64,
    velocity: Velocity,
    transform: Transform,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    commands: &mut Commands,
    object_id: u64,
    hp: u8
    //asset_server: Res<AssetServer>,
) -> Entity{
    
    let mut mesh = Mesh::new(PrimitiveTopology::LineList);
    let mut shadow_mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let seed = seed;
    
    let (vec, ind) = generate_asteroid_vertices(seed);
    let mut shadow_vec = vec.clone();
    let mut shadow_ind = vec![];
    shadow_vec.push([0., 0., 0.,]);
    for i in 0..ind.len(){
        shadow_ind.push(*ind.get(i).unwrap());
        if (i + 1) % 2 == 0{
            shadow_ind.push(shadow_vec.len() as u32 - 1);
        }
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec.clone());
    shadow_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, shadow_vec.clone());
    mesh.set_indices(Some(Indices::U32(ind.clone())));
    shadow_mesh.set_indices(Some(Indices::U32(shadow_ind.clone())));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![(Color::WHITE * 2.).as_rgba_f32(); vec.len()]);
    shadow_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::BLACK.as_rgba_f32(); shadow_vec.len()]);
    
    let (vertices, indices) = prepate_for_polyline(vec, ind);
    //let (vertices, indices) = prepate_for_trimesh(vec, ind);
    let mut summ = Vec2::ZERO;
    let mut count = 0;
    for vert in vertices.iter(){
        summ += *vert;
        count += 1;
    }
    let center = summ / count as f32;

    let shadow = commands.spawn((
        MaterialMesh2dBundle { //MESH
            mesh: Mesh2dHandle(meshes.add(shadow_mesh)),
            transform: Transform { translation: (-center).extend(-1.), scale: Vec3::splat(1.), ..default()},
            material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
            ..default()
        },
    )).id();

    return commands.spawn((
        RigidBody::Dynamic,
        //TransformBundle::from(Transform::from_xyz(0.0, 5.0, 0.0)), // SPAWN POSITION
        velocity,
        Friction::default(),
        GravityScale(0.0),
        Sleeping::disabled(),
        Ccd::enabled(),
        Object{
            id: object_id,
            object_type: ObjectType::Asteroid{seed: seed, hp: hp}
        },
        Collider::convex_decomposition(&vertices, &indices),
        //Collider::trimesh(vertices, indices), // trimesh is shit for dynamic bodies
        
        //Collider::ball(get_asteroid_size(seed) as f32 * 10.0),
        Restitution {
            coefficient: 0.1,
            combine_rule: CoefficientCombineRule::Average,
        },
        Name::new("ASTEROID"),
        Asteroid, // TAG get_asteroid_size(seed) * 2 - 1 
        
    )).insert(MaterialMesh2dBundle { //MESH
        mesh: Mesh2dHandle(meshes.add(mesh)),
        transform: transform.with_scale(Vec3::splat(1.)),
        material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
        ..default()
    }).add_child(shadow).id();
}

pub fn spawn_ship(
    mesh_only: bool,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    commands: &mut Commands,
    player_data: &ClientData,
    cfg: &mut ResMut<GlobalConfig>,
    time: &Time,
) -> Entity {
    let color = Color::from(player_data.color).as_rgba_f32();
    let target_style = player_data.style;
    
    let (triangle_vertices, mut triangle_indices) = get_ship_vertices(target_style);

    

    let mut vertices = vec![];
    let mut vertex_colors = vec![];
    let mut indices = vec![];


    let mut style2 = target_style.clone();
    let mut bits: [bool; 8] = [false; 8];
        
    for n in 0..=7{
        let m = 2_u8.pow(7 - n);
        //print!("{:?}", m);
        let i = style2 / m;
        //println!(" {:?}", i);
        if i == 1{
            bits[n as usize] = i != 0;
        }
        style2 = style2 % m;
    }
        
        
    //let is_lined = bits[2];
    //let is_spear = bits[3];
    //let is_spikes = bits[4];
    //let is_gem = bits[5];
    //let is_shards = bits[6];
    
    let offset: f32 = -(1. / 3.); // for fixing center of body. // todo: maybe it outdated

    let is_aspects = bits[7];
    let is_lined = bits[2];
    for i in 0..triangle_vertices.len(){
        let mut v = triangle_vertices[i].iter().map(|&p| Vec3::from((p.0, p.1 - 2.*offset, 0.))).collect::<Vec<_>>();

        let is_body = triangle_vertices[i].contains(&(0., 5.));
        let color = if is_body && is_aspects {Color::WHITE.as_rgba_f32()} else {color};

        vertex_colors.append(&mut vec![color; v.len()]);
        vertices.append(&mut v);
        indices.append(&mut triangle_indices[i]);

        //let ind = indices[i].clone();
        //
        //if ship_style.2{
        //    shapes.push(epaint::Shape::line(v, egui::Stroke{width:1., color: color}))
        //} else {
        //    shapes.push(epaint::Shape::mesh(
        //        epaint::Mesh{
        //            indices: ind,
        //            vertices: v.iter().map(|&p| Vertex { pos: p, uv: Pos2{x: 0., y:0.}, color: color }).collect::<Vec<_>>(),
        //            ..default()
        //        }
        //    ));
        //}
    }
    let mut mesh = if is_lined{
        Mesh::new(PrimitiveTopology::LineList)
    } else {
        Mesh::new(PrimitiveTopology::TriangleList)
    };
    vertices = vertices.iter().map(|x| *x * 3.).collect();
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);        
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vertex_colors);

    let entity = if !mesh_only{
        commands.spawn((
            RigidBody::Dynamic,
            Velocity {              // VELOCITY
                linvel: Vec2::new(0.0, 0.0),
                angvel: 0.0
            },
            Friction::default(),
            ColliderMassProperties::Density(2.),
            GravityScale(0.0),
            Sleeping::disabled(),
            Ccd::enabled(),
            Collider::trimesh(vec![Vec2::new(0., 17.), Vec2::new(-9., -4.), Vec2::new(9., -4.), Vec2::new(0., -13.)], vec![[0, 1, 3], [3, 2, 0]]),
            Restitution {
                coefficient: 0.1,
                combine_rule: CoefficientCombineRule::Average,
            },
            Name::new("Player"),
            ActiveEvents::CONTACT_FORCE_EVENTS,
            Ship,
            Object{
                id: player_data.object_id,
                object_type: ObjectType::Ship { style: target_style, color: player_data.color, shields: cfg.player_shields, hp: cfg.player_hp }
            },
            ShipStatuses{
                current: HashMap::new()
            },

            ShipState::Regular { spawn_time: time.elapsed_seconds() }

        ))
        .insert(LastDamageTaken{time: 0.})
        .insert(MaterialMesh2dBundle { //MESH
            
                mesh: Mesh2dHandle(meshes.add(mesh)),
                transform: Transform::from_translation(Vec3::ZERO),
                material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                ..default()
            },
        ).id()
    } else {
        commands.spawn(
            MaterialMesh2dBundle { //MESH
                mesh: Mesh2dHandle(meshes.add(mesh)),
                transform: Transform::from_translation(Vec3::ZERO),
                    //,
                material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                ..default()
            },
        ).id()
    };
    return entity
}


pub fn spawn_powerup(
    powerup_type: PowerUPType,
    pos: Vec3,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    asset_server: &Res<AssetServer>,
    object_id: u64,
) -> Entity{
    let mut mesh = Mesh::new(PrimitiveTopology::LineList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![
        Vec3{x: 1., y: 1., z: 1.},
        Vec3{x: 1., y: 1., z:-1.},
        Vec3{x: 1., y:-1., z: 1.},
        Vec3{x: 1., y:-1., z:-1.},
        Vec3{x:-1., y: 1., z: 1.},
        Vec3{x:-1., y: 1., z:-1.},
        Vec3{x:-1., y:-1., z: 1.},
        Vec3{x:-1., y:-1., z:-1.},
    ]);
    mesh.set_indices(Some(Indices::U32(vec![
        0, 1,
        0, 2,
        1, 3,
        2, 3,
        4, 5,
        4, 6,
        5, 7,
        6, 7,
        0, 4,
        1, 5,
        2, 6,
        3, 7
        ])));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![(Color::from([0.05, 0.05, 0.05, 1.])).as_rgba_f32(); 8]);

    let powerup_box = commands.spawn((
        MaterialMesh2dBundle { //MESH
            mesh: Mesh2dHandle(meshes.add(mesh)),
            transform: Transform::from_xyz(0., 0., 0.).with_scale(Vec3::splat(5.)),
            material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
            ..default()
        },
        PowerUPCube
    )).id();



    let powerup_image = commands.spawn((
        SpriteBundle {
            texture: asset_server.load(powerup_type.texture_path()),
            transform: Transform::from_xyz(0., 0., 0.01).with_scale(Vec3::splat(1.2)),
            ..default()
        },
        PowerUPImage
    )).id();
    return commands.spawn((
        Object{
            id: object_id,
            object_type: ObjectType::PickUP { pickup_type: powerup_type },
        },
        PowerUP,
        RigidBody::Fixed,
        ActiveEvents::COLLISION_EVENTS,
        Sensor,
        Collider::ball(10.),
        Velocity::zero(),
        VisibilityBundle::default(),
        TransformBundle::default()
    )).insert(Transform::from_translation(pos).with_scale(Vec3::splat(1.))).add_child(powerup_image).add_child(powerup_box).id();
}

#[allow(dead_code)]
pub fn update_powerups_animation(
    mut images_q: Query<&mut Transform, (With<PowerUPImage>, Without<Object>, Without<PowerUPCube>)>,
    mut cube_q: Query<&mut Transform, (With<PowerUPCube>, Without<Object>, Without<PowerUPImage>)>,
    time: Res<Time>,
){
    for mut cube in cube_q.iter_mut(){
        cube.rotate_local_x(time.delta_seconds() * 0.5);
        cube.rotate_local_y(-time.delta_seconds() * 1.);
        cube.rotate_local_z(time.delta_seconds() * 2.);
        cube.scale = Vec3::splat(5. + ((time.elapsed_seconds() * 3.).sin() + 1.) * 0.5);
    }
    for mut image in images_q.iter_mut(){
        image.rotate_local_y(time.delta_seconds() * 2.);
    }
}

pub fn debug_chunk_render( // todo: do something :D
    chunks_q: Query<(&Chunk, Entity)>,
    mut cfg: ResMut<GlobalConfig>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    keys: Res<Input<KeyCode>>, // todo: add check previous value
){
    if keys.just_pressed(KeyCode::F3){
        cfg.debug_render = !cfg.debug_render;
    }

    if !cfg.debug_render {
        for (_, e) in chunks_q.iter(){
            commands.entity(e).despawn();
        }
        return;
    }

    let font = asset_server.load("../assets/fonts/VecTerminus12Medium.otf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::GRAY,
    };
    let mut chunks_around: Vec<Vec2> = vec![];
    for x in -1..(cfg.map_size_chunks.x as i32+1){
        for y in -1..(cfg.map_size_chunks.y as i32+1){
            chunks_around.push(Vec2{x: x as f32, y: y as f32})
        }
    }
    let mut existing_debug_chunks: Vec<Vec2> = vec![];
    for (c, _) in chunks_q.iter(){
        existing_debug_chunks.push(c.pos);
    }
    for chunk in chunks_around.iter(){
        let isreal = cfg.chunk_to_real_chunk_v2(chunk) == *chunk;
        if cfg.debug_render {
            if !existing_debug_chunks.contains(chunk){
                let mut mesh = Mesh::new(PrimitiveTopology::LineList);
                let chunk_size = cfg.single_chunk_size;
                let vec = vec![
                    [-chunk_size.x/2. + 1., -chunk_size.y/2. + 1., 0.],
                    [ chunk_size.x/2. - 1., -chunk_size.y/2. + 1., 0.],
                    [ chunk_size.x/2. - 1.,  chunk_size.y/2. - 1., 0.],
                    [-chunk_size.x/2. + 1.,  chunk_size.y/2. - 1., 0.],
                ];
                let ind = vec![0, 1, 1, 2, 2, 3, 3, 0];
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec);
                mesh.set_indices(Some(Indices::U32(ind)));
                if isreal{
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::DARK_GRAY.as_rgba_f32(); 4]);
                } else {
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::ORANGE_RED.as_rgba_f32(); 4]);
                }
                let real_chunk_pos = cfg.chunk_to_real_chunk_v2(chunk);
                commands.spawn((
                    Text2dBundle {
                        text: Text::from_section( format!("{} {}\n({} {})", chunk.x, chunk.y, real_chunk_pos.x, real_chunk_pos.y), text_style.clone())
                            .with_alignment(TextAlignment::Center),
                        ..default()
                    },
                    Chunk{pos: Vec2{x: chunk.x, y: chunk.y}},
                )).insert(
                    MaterialMesh2dBundle { //MESH
                        mesh: Mesh2dHandle(meshes.add(mesh)),
                        transform: Transform::from_scale(Vec3::splat(1.)),
                        material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                        ..default()
                    }
                )
                .insert(Transform::from_xyz(
                    chunk_size.x/2. + chunk.x * chunk_size.x, 
                    chunk_size.y/2. + chunk.y * chunk_size.y, 
                    -1.0));
            } else {
                existing_debug_chunks.remove(existing_debug_chunks.iter().position(|x| x == chunk).unwrap());
            }
        }   
    }
}

#[allow(dead_code)]
pub fn snap_objects(                                                     
    cfg: ResMut<GlobalConfig>,
    mut objects: Query<&mut Transform, (With<Object>, Without<Puppet>)>, // ADD SNAPPING TO PUPPETS
){
    let xsize = cfg.map_size_chunks.x * cfg.single_chunk_size.x;
    let ysize = cfg.map_size_chunks.y * cfg.single_chunk_size.y;
    for mut transform in objects.iter_mut(){
        if transform.translation.x < 0.{
            transform.translation.x = (transform.translation.x + xsize) % xsize;
        } else {
            transform.translation.x = transform.translation.x % xsize;
        }
        if transform.translation.y < 0.{
            transform.translation.y = (transform.translation.y + ysize) % ysize;
        } else {
            transform.translation.y = transform.translation.y % ysize;
        }
    }
}


pub fn update_chunks_around(
    loaded_chunks: Res<LoadedChunks>,
    mut commands: Commands,

    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,

    chunks_q: Query<(&Chunk, Entity)>, // todo: do smt with chunk
    mut cfg: ResMut<GlobalConfig>,

    mut puppet_objects: Query<(&mut Transform, &Object, &Puppet, &mut Velocity, Entity), (With<Object>, With<Puppet>)>,
    objects: Query<(&Transform, &Object, &Velocity, Entity), (With<Object>, Without<Puppet>)>,

    asteroid_q: Query<(&Asteroid,  &Collider), (With<Object>, Without<Puppet>)>,
    bullet_q: Query<&Bullet, (With<Object>, Without<Puppet>)>,

    clients_data: Res<ClientsData>
    //ship_q: Query<&Ship, (With<Object>, Without<Puppet>)>,
){

    let mut chunks_around: Vec<Vec2> = vec![];
    for c in loaded_chunks.chunks.iter(){
        chunks_around.push(c.pos);
    }
    
    // GET REAL CHUNKS AND DRAW OTHER CHUNKS FOR DEBUG
    let mut real_chunks: Vec<Vec2> = vec![];


    for chunk in chunks_around.iter(){
        let isreal = cfg.chunk_to_real_chunk_v2(chunk) == *chunk;
        if isreal {real_chunks.push(*chunk);};
    }


    // COLLECT ALL REAL OBJECTS
    let mut real_objects_chunks: HashMap<(i64, i64), Vec<(&Transform, &Object, &Velocity, Entity)>> = HashMap::new();
    let mut real_objects: HashMap<u64, (&Transform, &Object, &Velocity, Entity)> = HashMap::new();

    for (transform, object, velocity,  entity) in objects.iter(){
        let real_chunk_pos = cfg.pos_to_real_chunk(&transform.translation);
        let key = (real_chunk_pos.x as i64, real_chunk_pos.y as i64);
        if real_objects_chunks.contains_key(&key){
            let data = real_objects_chunks.get_mut(&key).unwrap();
            data.push(( transform, object, velocity,  entity));
        } else {
            real_objects_chunks.insert(key, vec![(transform, object, velocity, entity)]);
        }
        real_objects.insert(object.id, (transform, object, velocity, entity));
    }
    
    // GET NEED-TO-SHADOW-CHUNKS (NOT NEED) AROUND_CHUNKS - REAL_CHUNKS = SHADOW CHUNKS
    // COLLECT AND MOVE EXISTED PUPPETS AND DELETE NOT NEEDED 
    //                            id  chunk pos
    let mut existing_puppets: Vec<(u64, i64, i64)> = vec![];
    
    for (mut puppet_transform, puppet_object, puppet, mut puppet_velocity, puppet_entity) in puppet_objects.iter_mut(){
        
        let puppet_position_chunk = cfg.pos_to_chunk(&puppet_transform.translation);
        let key = (puppet_object.id, puppet_position_chunk.x as i64, puppet_position_chunk.y as i64);
        if real_objects.contains_key(&puppet_object.id) &&    // COND 1 ( EXISTING OF REAL OBJECT )
        chunks_around.contains(&puppet_position_chunk) &&    // COND 2 ( EXISTING OF REAL CHUNK )
        puppet_position_chunk == puppet.binded_chunk.pos && // COND 3 ( STILL IN THEIR SHADOW CHUNK? )
        !existing_puppets.contains(&key) &&                // COND 4 ( DOES THAT PUPPET ALREADY EXISTS )
                                                            // COND 5 ( DOES REAL OBJECT IN THEIR CHUNK )
        cfg.pos_to_real_chunk(&puppet_transform.translation) == cfg.pos_to_chunk(&real_objects.get(&puppet_object.id).unwrap().0.translation)
        { 
            // APPLY REAL OBJECT's TRANSFORMS
            existing_puppets.push(key);
            let (transform, _, velocity, _) = real_objects.get(&puppet_object.id).unwrap();
            let offset = cfg.chunk_to_offset(&puppet_position_chunk);
            puppet_transform.translation = (transform.translation % Vec3 { x: cfg.single_chunk_size.x, y: cfg.single_chunk_size.y, z: 1. }) + Vec3{x: offset.x, y: offset.y, z:0.};
            puppet_transform.rotation = transform.rotation;
            puppet_velocity.angvel = velocity.angvel;
            puppet_velocity.linvel = velocity.linvel;
        } else {

            //println!("{:?}",  bullet_q.contains(*e));
            //println!("COND1 {}", real_objects.contains_key(&puppet_object.id));
            //println!("COND2 {}", chunks_around.contains(&puppet_position_chunk));
            //println!("COND3 {}", puppet_position_chunk == puppet.binded_chunk.pos);
            //println!("COND4 {} {:?}", !existing_puppets.contains(&key), key);
            //println!("COND5 {}", map.pos_to_real_chunk(&puppet_transform.translation) == map.pos_to_chunk(&real_objects.get(&puppet_object.id).unwrap().0.translation));
            
            // DESPAWN
            commands.entity(puppet_entity).despawn_recursive();
        }
    }
    //println!("{:?}", existing_puppets);

    // SPAWN NEW
    for chunk in chunks_around.iter(){
        if !real_chunks.contains(&chunk){ // IF CHUNK IS SHADOW-CHUNK
            let real_chunk = cfg.chunk_to_real_chunk_v2(chunk);
            let key = &(real_chunk.x as i64, real_chunk.y as i64);
            if real_objects_chunks.contains_key(key){ // IF ASTEROIDS IN CHUNK
                for (transform, object, velocity, entity) in real_objects_chunks.get(key).unwrap(){
                    if !existing_puppets.contains(&(object.id, chunk.x as i64, chunk.y as i64)){ // IF NOT ALREADY EXISTS
                        let pos = (transform.translation % Vec3 { x: cfg.single_chunk_size.x, y: cfg.single_chunk_size.y, z: 1. }) + // INCHUNK OFFSET
                            Vec3{x: chunk.x * cfg.single_chunk_size.x, y: chunk.y * cfg.single_chunk_size.y, z: 0.};             // CHUNK OFFSET
                        
                        match object.object_type{
                            ObjectType::Asteroid {seed, hp} => {
                                let entity = spawn_asteroid(seed, **velocity, transform.with_translation(pos), &mut meshes, &mut materials, &mut commands, object.id, cfg.get_asteroid_hp(seed));
                                commands.entity(entity).insert(
                                    (
                                        Puppet {
                                            id: object.id,
                                            binded_chunk: Chunk {
                                                pos: *chunk
                                            }
                                        },//.with_scale(Vec3::splat(2.))
                                        Name::new("ASTEROID PUPPET"),
                                    )
                                );
                            },
                            ObjectType::Bullet { previous_position, spawn_time, owner } => {
                                let entity = spawn_bullet(velocity.linvel, transform.with_translation(pos), object.id, owner, spawn_time, &asset_server, &mut commands);
                                commands.entity(entity).insert(
                                    (
                                        Puppet {
                                            id: object.id,
                                            binded_chunk: Chunk {
                                                pos: *chunk
                                        }
                                    },
                                    Name::new("BULLET PUPPET"),
                                    )
                                );
                            },
                            ObjectType::Ship { style, color, shields, hp } => {
                                let player_data = clients_data.get_option_by_object_id(object.id);
                                if player_data.is_some(){
                                    let player_data = player_data.unwrap();
                                    let entity = spawn_ship(false, &mut meshes, &mut materials, &mut commands, player_data, &mut cfg, &time);
                                    commands.entity(entity).insert((
                                        **velocity,
                                        Transform::from_translation(pos),
                                        Name::new(format!("Player Puppet of {}:{}", object.id, player_data.client_id)),
                                        ActiveEvents::CONTACT_FORCE_EVENTS,
                                        Puppet {
                                            id: object.id,
                                            binded_chunk: Chunk {
                                                pos: *chunk
                                            }
                                        },
                                        Object{
                                            id: object.id,
                                            object_type: ObjectType::Ship { style, color, shields, hp }
                                        },
                                    ));
                                }
                            }
                            ObjectType::PickUP{ pickup_type } => {
                                let entity = spawn_powerup(pickup_type, pos, &mut commands, &mut meshes, &mut materials, &asset_server, object.id);
                                commands.entity(entity).insert((
                                    **velocity,
                                    Transform::from_translation(pos),
                                    ActiveEvents::CONTACT_FORCE_EVENTS,
                                    Puppet {
                                        id: object.id,
                                        binded_chunk: Chunk {
                                            pos: *chunk
                                        }
                                    },
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
    
    return ()
}


pub fn spawn_bullet(
    target_velocity: Vec2,
    transform: Transform,
    object_id: u64,
    owner: u64,
    spawn_time: f32, // time.elapsed().as_secs_f32()
    asset_server: &Res<AssetServer>,
    commands: &mut Commands,
) -> Entity{
    commands.spawn((
        RigidBody::Dynamic,
        //TransformBundle::from(Transform::from_xyz(0.0, 5.0, 0.0)), // SPAWN POSITION
        Velocity {              // VELOCITY
            linvel: target_velocity,
            angvel: 0.0
        },
        Friction{ // DISABLE ROTATING WHET COLLIDING TO ANYTHING ( MAYBE REPLACE IT ONLY FOR WALLS FOR FUN )
            coefficient: 0.0,
            combine_rule: CoefficientCombineRule::Min
        },
        GravityScale(0.0),
        Sleeping::disabled(),
        Ccd::enabled(),
        //Collider::cuboid(2., 30.),
        Restitution {
            coefficient: 1.,
            combine_rule: CoefficientCombineRule::Multiply,
        },
        Name::new("Bullet"),
        //Sensor,
        ActiveEvents::COLLISION_EVENTS,
        Object{
            id: object_id,
            object_type: ObjectType::Bullet{previous_position: Transform::from_translation(transform.translation), spawn_time: spawn_time, owner: owner}
        },
        Bullet
    ))
    .insert(
        SpriteBundle {
            transform: Transform::from_matrix(Mat4::from_rotation_translation(Quat::from_rotation_z(Vec2::X.angle_between(target_velocity) + PI / 2.), transform.translation)),
            texture: asset_server.load("bullet.png"),
            ..default()
    }).id()
    
}


pub fn asteroids_refiller(
    mut objects_distribution: ResMut<ObjectsDistribution>,
    mut cfg: ResMut<GlobalConfig>,
    asteroids_q: Query<&Asteroid, Without<Puppet>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
){
    if asteroids_q.into_iter().len() < cfg.map_size_chunks.x as usize * cfg.map_size_chunks.y as usize{
        let seed = random::<u64>();
        let pos = get_pos_to_spawn(&mut objects_distribution, &cfg).extend(0.);
        let velocity = Velocity{
            linvel: (Vec2::from(random::<(f32, f32)>()) - Vec2::ONE * 0.5) * 0.,// todo: 300.
            angvel: (random::<f32>() - 0.5) * 5. 
        };
        spawn_asteroid(seed, velocity, Transform::from_translation(pos), &mut meshes, &mut materials, &mut commands, cfg.new_id(), cfg.get_asteroid_hp(seed));
    }
}



pub fn get_pos_to_spawn( // todo: make fast version, time usage might be insane!
    objects_distribution: &mut ResMut<ObjectsDistribution>,
    cfg: &ResMut<GlobalConfig>,
) -> Vec2 {

    
    
    let size = cfg.map_size_chunks;
    let size = (size.x as u32, size.y as u32);
    // chunks with player overrides chunks around
    let mut chunks_with_player_set: HashSet<(u32, u32)> = HashSet::new();
    let mut chunks_around_player_set: HashSet<(u32, u32)> = HashSet::new();

    
    let mut all_chunks: HashMap<u32, Vec<((u32, u32), Vec<Vec2>)>> = HashMap::new();;
    for x in 0..size.0{
        for y in 0..size.1{
            let key = (x, y);
            if objects_distribution.data.contains_key(&key){
                let (number, has_player, objects) = objects_distribution.data.get(&key).unwrap().clone();

                if all_chunks.contains_key(&number){
                    all_chunks.get_mut(&number).unwrap().push((key, objects));
                } else {
                    all_chunks.insert(number,  vec![(key, objects)]);
                }   

                if has_player {
                    let mut chunks: HashSet<(i32, i32)> = HashSet::new();
                    for cx in x as i32 - 1 ..= x as i32 + 1 {
                        for cy in y as i32 - 1 ..= y as i32 + 1 {
                            chunks.insert((cx, cy));
                        }
                    }
                    for chunk_pos in chunks.iter(){
                        let real_chunk_pos = ((chunk_pos.0 + size.0 as i32) % size.0 as i32, (chunk_pos.1 + size.1 as i32) % size.1 as i32);
                        let real_chunk_pos = (real_chunk_pos.0 as u32, real_chunk_pos.1 as u32); // after % it will never < 0 
                        if real_chunk_pos != (x, y){
                            chunks_around_player_set.insert(real_chunk_pos);
                        }
                    }
                    chunks_with_player_set.insert((x, y));
                }
            } else {
                if all_chunks.contains_key(&0){
                    all_chunks.get_mut(&0).unwrap().push((key, vec![]));
                } else {
                    all_chunks.insert(0,  vec![(key, vec![])]);
                }   
            }
        }
    }
    let mut rng = rand::thread_rng();
    
    // other -> around -> player
    let mut keys: Vec<u32> = all_chunks.keys().map(|x| *x).collect();
    keys.sort();

    let min_amount = keys.get(0).unwrap();
    let chunks_objects = all_chunks.get(min_amount).unwrap(); // get chunk with minimal amount of objects
    if *min_amount == 0 { // choose random location in chunk and return it
        let chunk = chunks_objects.get(rng.gen_range(0..chunks_objects.len())).unwrap();
        let chunk_pos = (chunk.0.0 * cfg.single_chunk_size.x as u32, chunk.0.1 * cfg.single_chunk_size.y as u32);
        let offset = (cfg.single_chunk_size.x * random::<f32>(), cfg.single_chunk_size.y * random::<f32>());
        return Vec2::from([chunk_pos.0 as f32 + offset.0, chunk_pos.1 as f32 + offset.1]);
    } else { // > 0 objects in chunk
        let mut without: HashMap<u32, Vec<((u32, u32), Vec<Vec2>)>> = HashMap::new();
        let mut around: HashMap<u32, Vec<((u32, u32), Vec<Vec2>)>> = HashMap::new();
        let mut with: HashMap<u32, Vec<((u32, u32), Vec<Vec2>)>> = HashMap::new();
        for k in keys.iter(){
            for tuple in all_chunks.get(k).unwrap().iter(){
                if chunks_with_player_set.contains(&tuple.0){
                    if with.contains_key(k){
                        with.get_mut(k).unwrap().push(tuple.clone());
                    } else {
                        with.insert(*k, vec![tuple.clone()]);
                    }
                } else if chunks_around_player_set.contains(&tuple.0){
                    if around.contains_key(k){
                        around.get_mut(k).unwrap().push(tuple.clone());
                    } else {
                        around.insert(*k, vec![tuple.clone()]);
                    }
                } else {
                    if without.contains_key(k){
                        without.get_mut(k).unwrap().push(tuple.clone());
                    } else {
                        without.insert(*k, vec![tuple.clone()]);
                    }
                }
            }
        }

        let find_dot = |chunk_data: &((u32, u32), Vec<Vec2>)| -> Vec2{
            let chunk_start = cfg.single_chunk_size * Vec2::from([chunk_data.0.0 as f32, chunk_data.0.1 as f32]); // left bottom corner
            let margins = cfg.single_chunk_size * 0.05; // 5%
            let data = chunk_data.1.clone();
            
            let dots = (0 .. data.len() + 1).map(|_|
                Vec2::from((
                    chunk_start.x + margins.x + ((cfg.single_chunk_size.x - margins.x * 2.) * random::<f32>()),
                    chunk_start.y + margins.y + ((cfg.single_chunk_size.y - margins.y * 2.) * random::<f32>())
                ))
            ).collect::<Vec<Vec2>>();
            
            let mut max_min_dist_dot = dots.get(0).unwrap(); // find dot with maxiaml of minimal distances to objects and "walls"
            let mut max_min_sq_dist = 0.;
            for dot_pos in dots.iter(){
                let mut min_sq_dist = dot_pos.distance_squared(*data.get(0).unwrap()); // data.len != 0 !!!

                min_sq_dist = min_sq_dist.min((dot_pos.x - chunk_start.x).powi(2)); // distance to left
                min_sq_dist = min_sq_dist.min((dot_pos.y - chunk_start.y).powi(2)); // distance to bottom
                min_sq_dist = min_sq_dist.min((dot_pos.x - (chunk_start.x + cfg.single_chunk_size.x)).powi(2)); // distance to right
                min_sq_dist = min_sq_dist.min((dot_pos.y - (chunk_start.y + cfg.single_chunk_size.y)).powi(2)); // distance to top

                for object_pos in data.iter(){
                    min_sq_dist = min_sq_dist.min(dot_pos.distance_squared(*object_pos));
                }
                if max_min_sq_dist < min_sq_dist {
                    max_min_dist_dot = dot_pos;
                    max_min_sq_dist = min_sq_dist;
                }
            }
            return *max_min_dist_dot;
        };

        if !without.is_empty(){
            let minimal = without.keys().min().unwrap();
            let chunks_data = without.get(minimal).unwrap();
            let chunk_data: &((u32, u32), Vec<Vec2>) = chunks_data.get(rng.gen_range(0..chunks_data.len())).unwrap();
            let dot = find_dot(chunk_data);
            return dot;
        } else if !around.is_empty(){
            let minimal = around.keys().min().unwrap();
            let chunks_data = around.get(minimal).unwrap();
            let chunk_data: &((u32, u32), Vec<Vec2>) = chunks_data.get(rng.gen_range(0..chunks_data.len())).unwrap();
            let dot = find_dot(chunk_data);
            return dot;
        } else if !with.is_empty(){
            let minimal = with.keys().min().unwrap();
            let chunks_data = with.get(minimal).unwrap();
            let chunk_data: &((u32, u32), Vec<Vec2>) = chunks_data.get(rng.gen_range(0..chunks_data.len())).unwrap();
            let dot = find_dot(chunk_data);
            return dot;
        }
    }
    println!("PIZDEC!");
    return Vec2::ZERO;
}


pub fn check_bullet_collisions_and_lifetime(
    mut bullets_data: Query<(Entity, &Transform, &mut Object), (With<Bullet>, Without<Puppet>)>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    /*mut query_asteroid: Query<(&mut Asteroid, &Velocity, &mut Object), Without<Puppet>>,
    mut query_ship: Query<&mut Object, With<Ship>>,*/
    states_q: Query<&ShipState, Without<Puppet>>,
    mut query_object: Query<(&mut Object, &mut Velocity), (Without<Puppet>, Without<Bullet>)>,
    mut cfg: ResMut<GlobalConfig>,
    asset_server: Res<AssetServer>,
    time: Res<Time>
){
    let mut to_despawn = HashSet::new();
    for (bullet_entity, transform, mut object) in bullets_data.iter_mut() { // todo: may crash when two bullets "touches" same asteroid at the same tick. fix!
        match object.object_type{
            ObjectType::Bullet { previous_position, spawn_time, owner} => {
                // HANDLE COLLISIONS
                let previous_pos = previous_position.translation;
                let previous_pos = Vec2::new(previous_pos.x, previous_pos.y);
                let point = Vec2::new(transform.translation.x, transform.translation.y);
                let dir = point - previous_pos;
                let len = dir.length();
                let filter = QueryFilter::default();


                
                // check collisions
                rapier_context.intersections_with_ray(
                    point, dir, len, true, filter,
                    |entity, intersection| {
                        let hit_point = intersection.point;
                        
                        let ray_len = (hit_point - point).length(); // check 
                        if ray_len > len {
                            return true
                        }

                        

                        //let hit_normal = intersection.normal; // USE FOR PARTILCES


                        // Check if entity is asteroid
                        if let Ok(tuple) = query_object.get_mut(entity){
                            let (mut object, velocity) = tuple;
                            match object.object_type{
                                ObjectType::Asteroid { seed, mut hp } => {
                                    if hp != 0{ // inserting everything
                                        hp = hp - 1;
                                    }
                                    object.object_type = ObjectType::Asteroid { seed: seed, hp: hp };
                                    commands.entity(entity).insert(object.clone());
                                    
                                    commands.entity(bullet_entity).despawn_recursive();
                                    if hp <= 0{
                                        commands.entity(entity).despawn_recursive();
                                        
                                        /*
                                            |
                                            v
                                        o <- O -> o
                                        SPLIT ASTEROID
                                        todo: fix velocity
                                        todo: fix client lag when asteroid splits
                                        */
        
                                        let current_size = get_asteroid_size(seed);
                                        if current_size != 1{
                                            let dir = (hit_point - dir).normalize().perp();
                                            let dir = Vec3{x: dir.x, y: dir.y, z:0.0};
                                            let dir1 = dir;
                                            let dir2 = -dir;
                                            let vel1 = Velocity{linvel: Vec2{x: velocity.linvel.x + dir1.x * (1. - random::<f32>()) * 50., y: velocity.linvel.y + dir1.x * (1. - random::<f32>()) * 50.}, angvel: velocity.angvel + (1. - random::<f32>() * 4.)};
                                            let vel2 = Velocity{linvel: Vec2{x: velocity.linvel.x + dir2.x * (1. - random::<f32>()) * 50., y: velocity.linvel.y + dir2.x * (1. - random::<f32>()) * 50.}, angvel: velocity.angvel + (1. - random::<f32>() * 4.)};
        
                                            let mut new_seed_1 = random::<u64>();
                                            while current_size - 1 != get_asteroid_size(new_seed_1){
                                                new_seed_1 = random::<u64>();
                                            }
                                            let mut new_seed_2 = random::<u64>();
                                            while current_size - 1 != get_asteroid_size(new_seed_2){
                                                new_seed_2 = random::<u64>();
                                            }
                                            spawn_asteroid(
                                                new_seed_1,
                                                vel1,
                                                Transform::from_translation(transform.translation + dir1 * current_size as f32 * 5.),
                                                &mut meshes,
                                                &mut materials,
                                                &mut commands,
                                                cfg.new_id(),
                                                cfg.get_asteroid_hp(new_seed_1)
                                            );
                                            spawn_asteroid(
                                                new_seed_2,
                                                vel2,
                                                Transform::from_translation(transform.translation + dir2 * current_size as f32 * 5.),
                                                &mut meshes,
                                                &mut materials,
                                                &mut commands,
                                                cfg.new_id(),
                                                cfg.get_asteroid_hp(new_seed_2)
                                            );
                                        }
                                        // spawn powerup
                                        if rand::random::<f32>() < cfg.powerup_drop_chances{
                                            let powerup_type = match rand::thread_rng().gen_range(0..=4){
                                                0 => {PowerUPType::Repair}
                                                1 => {PowerUPType::ExtraDamage}
                                                2 => {PowerUPType::Haste}
                                                3 => {PowerUPType::SuperShield}
                                                _ => {PowerUPType::Invisibility}
                                            };
                                            spawn_powerup(powerup_type, transform.translation, &mut commands, &mut meshes, &mut materials, &asset_server, cfg.new_id());
                                        }
                                    };
                                    return false
                                }
                                ObjectType::Ship { style, color, mut shields, mut hp} => {
                                    match states_q.get(entity).unwrap() {
                                        ShipState::Dash { start_time: _, direction: _ } => {
                                            to_despawn.insert(bullet_entity);
                                            return false
                                        },
                                        ShipState::Regular { spawn_time } => {
                                            if spawn_time + cfg.spawn_immunity_time > time.elapsed_seconds(){
                                                to_despawn.insert(bullet_entity);
                                                return false
                                            }
                                        },
                                        ShipState::Dead { time: _ } => {
                                            return true
                                        },
                                    }
                                    if !(object.id == owner && time.elapsed().as_secs_f32() - spawn_time < 0.3){ // check ownership, after some time bullet will damage owner
                                        commands.entity(entity).insert(LastDamageTaken{time: time.elapsed_seconds()});
                                        if shields > 0.{
                                            shields -= cfg.bullet_damage;
                                            //println!("s {}", shields);
                                            shields = if shields > 0. {shields} else {0.};
                                            //println!("s {}", shields);
                                        } else {                                            
                                            hp -= cfg.bullet_damage;
                                            //println!("h {}", hp);
                                            hp = if hp > 0. {hp} else {0.};
                                            //println!("h {}", hp);
                                        }
                                        
                                        if hp <= 0. {
                                            let mut object_copy = object.clone();
                                            object_copy.object_type = ObjectType::Ship { style , color,  shields, hp };
                                            commands.entity(entity).insert((
                                                ShipState::Dead { time: 0. },
                                                //Visibility::Hidden,
                                                ColliderDisabled,
                                                Velocity::zero(),
                                                object_copy
                                            ));

                                        } else {
                                            let mut object_copy = object.clone();
                                            object_copy.object_type = ObjectType::Ship { style , color,  shields, hp };
                                            commands.entity(entity).insert(object_copy);
                                            //println!("-> {} hp {} sh {}", object.id, hp, shields);
                                        }
                                        to_despawn.insert(bullet_entity);
                                        return false
                                    }
                                    
                                }
                                _ => {}
                            }
                        }
                        return true // Return `false` instead if we want to stop searching for other hits.
                });
                // UPDATE
                //println!("pos {} -> {}", previous_position.translation.truncate(), transform.translation.truncate());
                object.object_type = ObjectType::Bullet {
                    previous_position: *transform,
                    spawn_time,
                    owner
                };
                //LIFETIME
                if time.elapsed().as_secs_f32() - spawn_time > cfg.bullet_lifetime_secs{
                    to_despawn.insert(bullet_entity);
                }
            }
            _ => {}
        }
    }
    for e in to_despawn.iter(){
        commands.entity(*e).despawn();
    }
}

pub fn check_ship_force_events( // todo: take damage on colide with asteroid
    mut commands: Commands,
    mut query_ship: Query<(Entity, &mut Object), (With<Ship>, Without<Puppet>, Without<Bullet>)>,
    mut cfg: ResMut<GlobalConfig>,
    mut contact_force_events: EventReader<ContactForceEvent>,
    time: Res<Time>
){
    for contact_force_event in contact_force_events.read() {
        println!("Received contact force event: {:?}", contact_force_event);
    }
    // ContactForceEventThreshold

    /*for (ship_entity, mut object) in query_ship.iter_mut() { // check if respawn needed
        /*match object.object_type{
            ObjectType::Ship { style, color, shields: _, hp: _, death_time } => {
                if time.elapsed().as_secs_f32() - death_time > cfg.respawn_time_secs {
                    let mut object_copy = object.clone();
                    object_copy.object_type = ObjectType::Ship { style, color, shields: cfg.player_shields, hp: cfg.player_hp, death_time: 0. };
                    commands.entity(ship_entity).remove::<ColliderDisabled>();
                    commands.entity(ship_entity).insert((
                        Visibility::Inherited,
                        object_copy
                    ));
                }
            },
            _ => {}
        }*/
    }*/
}

pub fn check_ship_effects(

){

}

pub fn check_pickups_collisions_and_lifetime(
    mut collision_events: EventReader<CollisionEvent>,
    mut ship_q: Query<&mut ShipStatuses, (With<Ship>, Without<Puppet>)>,
    mut powerup_q: Query<(Entity, &Object), (With<PowerUP>, Without<Puppet>)>,
    mut commands: Commands,
    cfg: Res<GlobalConfig>,
){
    for collision_event in collision_events.read() {
        match *collision_event { // todo: may crash when two ships "touches" same pup at the same tick. fix!
            CollisionEvent::Started(e0, e1, flags) => {
                match flags {
                    CollisionEventFlags::SENSOR => {}
                    _ => {continue;}
                }
                let e0_ship = ship_q.get(e0);
                let e1_ship = ship_q.get(e1);
                let e0_powerup = powerup_q.get(e0);
                let e1_powerup = powerup_q.get(e1);

                let (mut ship_effects, powerup) = 
                    if e0_ship.is_ok() && e1_powerup.is_ok() {(ship_q.get_mut(e0).unwrap(), e1_powerup.unwrap())}
                else
                    if e0_powerup.is_ok() && e1_ship.is_ok() {(ship_q.get_mut(e1).unwrap(), e0_powerup.unwrap())}
                else
                    {continue;};
                let (powerup_e, powerup_object) = powerup;

                
                
                match powerup_object.object_type {
                    ObjectType::PickUP { pickup_type } => {
                        ship_effects.current.insert(pickup_type, cfg.get_power_up_effect(pickup_type));
                        println!("zxc! {}", ship_effects.current.len());
                    }
                    _ => {}
                }
                
                commands.entity(powerup_e).despawn_recursive();
            },
            _ => {/* pass */},
        }
    }

    
} // todo