use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle}, render::{render_resource::{PrimitiveTopology, Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages}, mesh::Indices, camera::RenderTarget, view::RenderLayers}, utils::HashMap, core_pipeline::{tonemapping::{Tonemapping, DebandDither}, bloom::{BloomSettings, BloomCompositeMode}, clear_color::ClearColorConfig}, window::WindowResized, input::keyboard::KeyboardInput,
};

use bevy_rapier2d::prelude::*;
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;

#[path = "settings.rs"] pub mod settings;
pub use settings::*;
#[path = "components.rs"] pub(crate) mod components;
pub use components::*;




pub fn pixel_camera_event_listener(
    mut settings: ResMut<GameSettings>,
    mut listener: EventReader<ApplyCameraSettings>,
    mut camera: Query<(&mut Tonemapping, &mut BloomSettings, &mut DebandDither), With<PixelCamera>>
){
    for e in listener.iter(){
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

#[allow(dead_code)]
pub fn setup_pixel_camera(
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
            transform: Transform::from_xyz(0.0, 0.0, 15.0).looking_at(Vec3::ZERO, Vec3::Y),
            ..default()
        }, second_pass_layer,
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

#[allow(dead_code)]
pub const TARGET_HEIGHT: f32 = 512.;

#[allow(dead_code)]
pub fn update_pixel_camera(
    resize_event: Res<Events<WindowResized>>,
    mut canvas_q: Query<(&Handle<Image>, &mut Transform), With<CameraCanvas>>,
    mut images: ResMut<Assets<Image>>,
){
    let mut reader = resize_event.get_reader();
    let (image_handle, mut transform) = canvas_q.single_mut();
    for e in reader.iter(&resize_event) {
        if e.height == 0.{continue;}
        let raito = TARGET_HEIGHT / e.height;
        let size = Extent3d {
            width: (e.width * raito) as u32,
            height: (e.height * raito) as u32,
            ..default()
        };
        let target_size = e.width / size.width as f32;
        let img = images.get_mut(&image_handle).unwrap();
        img.resize(size);
        transform.scale = Vec3::splat(target_size);
    }
}

//#[allow(dead_code)]
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
    0 standart
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

    //let permutation = permutation::sort(&z_indexes);
    //let vertices = permutation.apply_slice(&vertices);
    //let indices = permutation.apply_slice(&indices);
    
    return (vertices, indices);
}


fn get_asteroid_size(seed: u64) -> i32{
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
     match rng.gen_range(0..16) {
        0..=6 => 1,
        7..=14 => 2,
        15..=16 => 3,
        e => {println!("{}", e); 1}
    }
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


pub fn spawn_asteroid( // replace to generate vertices function for showcase sending it to clients (OR NO!)
    mut events: EventReader<SpawnAsteroid>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut commands: Commands,
    mut map_settings: ResMut<MapSettings>,
    //asset_server: Res<AssetServer>,
){
    for data in events.iter(){
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);

        let seed = data.seed;
        
        let (vec, ind) = generate_asteroid_vertices(seed);

        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec.clone());
        mesh.set_indices(Some(Indices::U32(ind.clone())));

        mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::WHITE.as_rgba_f32(); 16]);
        
        let (vertices, indices) = prepate_for_polyline(vec, ind);
        //let (vertices, indices) = prepate_for_trimesh(vec, ind);
        commands.spawn((
            RigidBody::Dynamic,
            //TransformBundle::from(Transform::from_xyz(0.0, 5.0, 0.0)), // SPAWN POSITION
            data.velocity,
            Friction{ // DISABLE ROTATING WHET COLLIDING TO ANYTHING ( MAYBE REPLACE IT ONLY FOR WALLS FOR FUN )
                coefficient: 0.3,
                combine_rule: CoefficientCombineRule::Min
            },
            GravityScale(0.0),
            Sleeping::disabled(),
            Ccd::enabled(),

            Object{
                id: map_settings.new_id(),
                object_type: ObjectType::Asteroid
            },
            Collider::convex_decomposition(&vertices, &indices),
            //Collider::trimesh(vertices, indices), // trimesh is shit for dynamic bodies

           

            //Collider::ball(get_asteroid_size(seed) as f32 * 10.0),
            Restitution {
                coefficient: 1.,
                combine_rule: CoefficientCombineRule::Multiply,
            },
            Name::new("ASTEROID"),
            Asteroid{seed: seed, hp: 1 }, // TAG get_asteroid_size(seed) * 2 - 1 
            
        )).insert(MaterialMesh2dBundle { //MESH
            mesh: Mesh2dHandle(meshes.add(mesh)),
            transform: Transform::from_translation(data.transform.translation)
                .with_scale(Vec3::splat(2.)),
            material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
            ..default()
        },);
        
    }
}

pub fn spawn_ship(
    mesh_only: bool,
    meshes: &mut ResMut<Assets<Mesh>>,
    materials: &mut ResMut<Assets<ColorMaterial>>,
    commands: &mut Commands,
    player_data: &ClientData,
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


    let is_aspects = bits[7];
    let is_lined = bits[2];
    for i in 0..triangle_vertices.len(){
        let mut v = triangle_vertices[i].iter().map(|&p| Vec3::from((p.0, p.1, 0.))).collect::<Vec<_>>();

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
            Friction{ // DISABLE ROTATING WHET COLLIDING TO ANYTHING ( MAYBE REPLACE IT ONLY FOR WALLS FOR FUN )
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min
            },
            GravityScale(0.0),
            Sleeping::disabled(),
            Ccd::enabled(),
            Collider::triangle(Vec2::new(0., 12.0), Vec2::new(-6., -6.), Vec2::new(6., -6.)),
            Restitution {
                coefficient: 1.,
                combine_rule: CoefficientCombineRule::Multiply,
            },
            Name::new("Player"),
            ActiveEvents::CONTACT_FORCE_EVENTS,
            ControlledPlayer,
            Ship,
            Object{
                id: player_data.object_id,
                object_type: ObjectType::Ship
            },
        )).insert(MaterialMesh2dBundle { //MESH
                mesh: Mesh2dHandle(meshes.add(mesh)),
                transform: Transform::from_translation(Vec3::new(0., 0., 0.)).with_scale(Vec3::splat(3.)),
                material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                ..default()
            },
        ).id()
    } else {
        commands.spawn(
            MaterialMesh2dBundle { //MESH
                mesh: Mesh2dHandle(meshes.add(mesh)),
                transform: Transform::from_translation(Vec3::new(0., 0., 0.)).with_scale(Vec3::splat(32.)),
                    //,
                material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                ..default()
            },
        ).id()
    };
    return entity
}



pub fn debug_chunk_render(
    chunks_q: Query<(&Chunk, Entity)>,
    mut map: ResMut<MapSettings>,
    asset_server: Res<AssetServer>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    keys: Res<Input<KeyCode>>,
){
    if keys.just_pressed(KeyCode::F3){
        map.debug_render = !map.debug_render;
    }

    if !map.debug_render {
        for (_, e) in chunks_q.iter(){
            commands.entity(e).despawn();
        }
        return;
    }

    let font = asset_server.load("fonts/F77MinecraftRegular-0VYv.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::GRAY,
    };
    let mut chunks_around: Vec<Vec2> = vec![];
    for x in -1..(map.max_size.x as i32+1){
        for y in -1..(map.max_size.y as i32+1){
            chunks_around.push(Vec2{x: x as f32, y: y as f32})
        }
    }
    let mut existing_debug_chunks: Vec<Vec2> = vec![];
    for (c, _) in chunks_q.iter(){
        existing_debug_chunks.push(c.pos);
    }
    for chunk in chunks_around.iter(){
        let isreal = map.chunk_to_real_chunk_v2(chunk) == *chunk;
        if map.debug_render {
            if !existing_debug_chunks.contains(chunk){
                let mut mesh = Mesh::new(PrimitiveTopology::LineList);
                let chunk_size = map.single_chunk_size;
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
                let real_chunk_pos = map.chunk_to_real_chunk_v2(chunk);
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

pub fn snap_objects(                                                     
    map_settings: ResMut<MapSettings>,
    mut objects: Query<&mut Transform, (With<Object>, Without<Puppet>)>, // ADD SNAPPING TO PUPPETS
){
    let xsize = map_settings.max_size.x * map_settings.single_chunk_size.x;
    let ysize = map_settings.max_size.y * map_settings.single_chunk_size.y;
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
        //transform.translation.x = transform.translation.x % xsize;
    }
}


pub fn update_chunks_around(
    loaded_chunks: Res<LoadedChunks>,
    mut commands: Commands,

    time: Res<Time>,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,

    chunks_q: Query<(&Chunk, Entity)>,
    map: ResMut<MapSettings>,

    mut puppet_objects: Query<(&mut Transform, &Object, &Puppet, &mut Velocity, Entity), (With<Object>, With<Puppet>)>,
    objects: Query<(&Transform, &Object, &Velocity, Entity), (With<Object>, Without<Puppet>)>,

    asteroid_q: Query<(&Asteroid,  &Collider), (With<Object>, Without<Puppet>)>,
    bullet_q: Query<&Bullet, (With<Object>, Without<Puppet>)>,

    clients_data: Res<ClientsData>
    //ship_q: Query<&Ship, (With<Object>, Without<Puppet>)>,
){
    let font = asset_server.load("fonts/F77MinecraftRegular-0VYv.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::GRAY,
    };
    

    let mut chunks_around: Vec<Vec2> = vec![];
    for c in loaded_chunks.chunks.iter(){
        chunks_around.push(c.pos);
    }
    
    // GET REAL CHUNKS AND DRAW OTHER CHUNKS FOR DEBUG
    let mut real_chunks: Vec<Vec2> = vec![];

    let mut existing_debug_chunks: Vec<Vec2> = vec![];
    for (c, _) in chunks_q.iter(){
        existing_debug_chunks.push(c.pos);
    }

    for chunk in chunks_around.iter(){

        let isreal = map.chunk_to_real_chunk_v2(chunk) == *chunk;
        if isreal {real_chunks.push(*chunk);};
        /*
        if map.debug_render {
            if !existing_debug_chunks.contains(chunk){
                let mut mesh = Mesh::new(PrimitiveTopology::LineList);
                let chunk_size = map.single_chunk_size;
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
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::ORANGE_RED.as_rgba_f32(); 4]);
                } else {
                    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::DARK_GRAY.as_rgba_f32(); 4]);
                }
                let real_chunk_pos = map.chunk_to_real_chunk_v2(chunk);
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
        */
    }
    /* 
    // REMOVE UNUSED DEBUG CHUNKS
    for (chunk, entity) in chunks_q.iter(){
        if existing_debug_chunks.contains(&chunk.pos){
            commands.entity(entity).despawn_recursive();
        }
    }*/
    drop(existing_debug_chunks);


    // COLLECT ALL REAL OBJECTS
    let mut real_objects_chunks: HashMap<(i64, i64), Vec<(&Transform, &Object, &Velocity, Entity)>> = HashMap::new();
    let mut real_objects: HashMap<u64, (&Transform, &Object, &Velocity, Entity)> = HashMap::new();

    for (transform, object, velocity,  entity) in objects.iter(){
        let real_chunk_pos = map.pos_to_real_chunk(&transform.translation);
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
        
        let puppet_position_chunk = map.pos_to_chunk(&puppet_transform.translation);
        let key = (puppet_object.id, puppet_position_chunk.x as i64, puppet_position_chunk.y as i64);
        if real_objects.contains_key(&puppet_object.id) &&    // COND 1 ( EXISTING OF REAL OBJECT )
        chunks_around.contains(&puppet_position_chunk) &&    // COND 2 ( EXISTING OF REAL CHUNK )
        puppet_position_chunk == puppet.binded_chunk.pos && // COND 3 ( STILL IN THEIR SHADOW CHUNK? )
        !existing_puppets.contains(&key) &&                // COND 4 ( DOES THAT PUPPET ALREADY EXISTS )
                                                            // COND 5 ( DOES REAL OBJECT IN THEIR CHUNK )
        map.pos_to_real_chunk(&puppet_transform.translation) == map.pos_to_chunk(&real_objects.get(&puppet_object.id).unwrap().0.translation)
        { 
            // APPLY REAL OBJECT's TRANSFORMS
            existing_puppets.push(key);
            let (transform, _, velocity, _) = real_objects.get(&puppet_object.id).unwrap();
            let offset = map.chunk_to_offset(&puppet_position_chunk);
            puppet_transform.translation = (transform.translation % Vec3 { x: map.single_chunk_size.x, y: map.single_chunk_size.y, z: 1. }) + Vec3{x: offset.x, y: offset.y, z:0.};
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
            let real_chunk = map.chunk_to_real_chunk_v2(chunk);
            let key = &(real_chunk.x as i64, real_chunk.y as i64);
            if real_objects_chunks.contains_key(key){ // IF ASTEROIDS IN CHUNK
                for (transform, object, velocity, entity) in real_objects_chunks.get(key).unwrap(){
                    if !existing_puppets.contains(&(object.id, chunk.x as i64, chunk.y as i64)){ // IF NOT ALREADY EXISTS
                        let pos = (transform.translation % Vec3 { x: map.single_chunk_size.x, y: map.single_chunk_size.y, z: 1. }) + // INCHUNK OFFSET
                            Vec3{x: chunk.x * map.single_chunk_size.x, y: chunk.y * map.single_chunk_size.y, z: 0.};             // CHUNK OFFSET
                        
                        match object.object_type{
                            ObjectType::Asteroid => {
                                let (asteroid, collider) = asteroid_q.get(*entity).unwrap();
                                let mut mesh = Mesh::new(PrimitiveTopology::LineList);
                                let seed = asteroid.seed;
                                let (vec, ind) = generate_asteroid_vertices(seed);

                                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec.clone());
                                mesh.set_indices(Some(Indices::U32(ind.clone())));

                                
                                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::WHITE.as_rgba_f32(); 16]);
                                
                                //let (vertices, indices) = prepate_for_trimesh(vec, ind);

                                commands.spawn((
                                    RigidBody::Dynamic,
                                    **velocity,
                                    Friction{ // DISABLE ROTATING WHET COLLIDING TO ANYTHING ( MAYBE REPLACE IT ONLY FOR WALLS FOR FUN )
                                        coefficient: 0.0,
                                        combine_rule: CoefficientCombineRule::Min
                                    },
                                    GravityScale(0.0),
                                    Sleeping::disabled(),
                                    Ccd::enabled(),
                                    Restitution {
                                        coefficient: 1.,
                                        combine_rule: CoefficientCombineRule::Multiply,
                                    },
                                    Name::new("ASTEROID PUPPET"),
                                    Asteroid{seed: seed, hp: 1 }, // TAG get_asteroid_size(seed) * 2 - 1 
                                    Puppet {
                                        id: object.id,
                                        binded_chunk: Chunk {
                                            pos: *chunk
                                        }
                                    },
                                    Object{
                                        id: object.id,
                                        object_type: ObjectType::Asteroid
                                    },
                                    Collider::from(collider.raw.clone())
                                )).insert(MaterialMesh2dBundle { //MESH
                                    mesh: Mesh2dHandle(meshes.add(mesh)),
                                    transform: transform.with_translation(pos).with_scale(Vec3::splat(2.)),
                                    material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                                    ..default()
                        },);
                            },
                            ObjectType::Bullet => {
                                let owner = bullet_q.get(*entity).unwrap().owner;
                                let mut mesh = Mesh::new(PrimitiveTopology::LineList);
                                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0., 0., 0.,], [0., -50., 0.,]]);
                                mesh.set_indices(Some(Indices::U32(vec![0, 1])));
                                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::Rgba { red: 3., green: 3., blue: 3., alpha: 3. }.as_rgba_f32() ; 2]);
                                let vel = transform.up() * 1000.;
                                let vel = Vec2 { x: vel.x, y: vel.y };
                                //let vel = Vec2{x: 0.0, y: 0.0};

                                commands.spawn((RigidBody::Dynamic,
                                //TransformBundle::from(Transform::from_xyz(0.0, 5.0, 0.0)), // SPAWN POSITION
                                Velocity {              // VELOCITY
                                    linvel: vel,
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
                                    id: object.id,
                                    object_type: ObjectType::Bullet
                                },
                                Puppet {
                                    id: object.id,
                                    binded_chunk: Chunk {
                                        pos: *chunk
                                    }
                                },
                                Bullet{previous_position: transform.with_translation(pos), spawn_time: time.elapsed().as_secs_f32(), owner: owner}
                                ))
                                .insert(MaterialMesh2dBundle {
                                    mesh: Mesh2dHandle(meshes.add(mesh.clone())),
                                    material: materials.add(ColorMaterial::default()),
                                    transform: transform.with_translation(pos),
                                    ..default()}
                                )
                                .insert(transform.with_translation(pos));

                            },
                            ObjectType::Ship => {
                                //todo ЗАМЕНИТЬ НА spawn/get_ship_bundle
                                let player_data = clients_data.get_by_object_id(object.id);
                                let entity = spawn_ship(true, &mut meshes, &mut materials, &mut commands, player_data);

                                commands.entity(entity).insert((
                                    RigidBody::Dynamic,
                                    **velocity,
                                    Friction{ // DISABLE ROTATING WHET COLLIDING TO ANYTHING ( MAYBE REPLACE IT ONLY FOR WALLS FOR FUN )
                                        coefficient: 0.0,
                                        combine_rule: CoefficientCombineRule::Min
                                    },
                                    GravityScale(0.0),
                                    Sleeping::disabled(),
                                    Ccd::enabled(),
                                    Collider::triangle(Vec2::new(0., 0.5), Vec2::new(-0.33, -0.4), Vec2::new(0.33, -0.4)),
                                    Restitution {
                                        coefficient: 1.,
                                        combine_rule: CoefficientCombineRule::Multiply,
                                    },
                                    Name::new("PlayerPuppet"),
                                    ActiveEvents::CONTACT_FORCE_EVENTS,
                                    Ship{},
                                    Puppet {
                                        id: object.id,
                                        binded_chunk: Chunk {
                                            pos: *chunk
                                        }
                                    },
                                    Object{
                                        id: object.id,
                                        object_type: ObjectType::Ship
                                    },
                                    transform.with_translation(pos),
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
