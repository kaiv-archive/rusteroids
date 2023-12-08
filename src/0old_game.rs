use bevy::{
    prelude::*,
    sprite::{MaterialMesh2dBundle, Mesh2dHandle, Material2d}, render::{render_resource::{PrimitiveTopology, Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages}, mesh::Indices, camera::RenderTarget, view::RenderLayers}, utils::HashMap, ui::RelativeCursorPosition, diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin}, core_pipeline::{tonemapping::Tonemapping, clear_color::ClearColorConfig, bloom::{BloomSettings, BloomCompositeMode}}, window::WindowResized,
};

use bevy_hanabi::velocity;
use bevy_rapier2d::{prelude::*, na::{Rotation, Rotation3, Vector3}, rapier::prelude::Aabb};
use rand::prelude::*;
use rand_chacha::ChaCha8Rng;



///////////////


#[derive(Component)]
pub struct CameraCanvas;

#[derive(Component)]
pub struct MainCamera;



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
    let second_pass_layer = RenderLayers::layer(1);



    commands.spawn((
        Camera2dBundle {
            camera_2d: Camera2d {
                clear_color: ClearColorConfig::Custom(Color::Rgba { red: 0., green: 0., blue: 0., alpha: 0. }),
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
            transform: Transform::from_scale(Vec3::splat(2.0)),
            ..default()
        },
        BloomSettings{ // 3. Enable bloom for the camera
            composite_mode: BloomCompositeMode::Additive,
            intensity: 0.1,
            ..default()
        },
        MainCamera,
    ));


    // The main pass camera.
    commands.spawn((
    Camera2dBundle {
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
        second_pass_layer,
    ));
}



const TARGET_HEIGHT: f32 = 400.;

pub fn update_pixel_camera(
    resize_event: Res<Events<WindowResized>>,
    mut canvas_q: Query<(&Handle<Image>, &mut Transform), With<CameraCanvas>>,
    mut images: ResMut<Assets<Image>>,
){
    let mut reader = resize_event.get_reader();
    let (image_handle, mut transform) = canvas_q.single_mut();
    for e in reader.iter(&resize_event) {
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
///////////////








const BULLET_LIFETIME: f32 = 30.; // in seconds

// [RESOURCES]

#[derive (Resource)]
pub struct MapSettings{
    pub last_id: u64,
    pub max_size: Vec2,
    pub single_chunk_size: Vec2,
    pub debug_render: bool,
}
impl MapSettings{
    pub fn new_id(&mut self) -> u64{ // ID 0 IS EMPTY!!!!
        self.last_id += 1;
        return self.last_id;
    }
    fn pos_to_chunk(&self, pos: &Vec3) -> Vec2{
        Vec2{x: (pos.x / self.single_chunk_size.x).floor(), y: (pos.y / self.single_chunk_size.y).floor()}
    }
    
    fn pos_to_real_chunk(&self, pos: &Vec3) -> Vec2{
        let chunk = self.pos_to_chunk(pos);
        Vec2{x: chunk.x.rem_euclid(self.max_size.x), y: chunk.y.rem_euclid(self.max_size.y)}
    }
    fn pos_to_chunk_v2(&self, pos: &Vec2) -> Vec2{
        Vec2{x: (pos.x / self.single_chunk_size.x).floor(), y: (pos.y / self.single_chunk_size.y).floor()}
    }
    
    fn pos_to_real_chunk_v2(&self, pos: &Vec2) -> Vec2{
        let chunk = self.pos_to_chunk_v2(pos);
        Vec2{x: chunk.x.rem_euclid(self.max_size.x), y: chunk.y.rem_euclid(self.max_size.y)}
    }

    fn chunk_to_real_chunk_v2(&self, chunk: &Vec2) -> Vec2{
        Vec2{x: chunk.x.rem_euclid(self.max_size.x), y: chunk.y.rem_euclid(self.max_size.y)}
    }

    fn chunk_to_offset(&self, chunk: &Vec2) -> Vec2{
        Vec2{x: chunk.x * self.single_chunk_size.x, y: chunk.y * self.single_chunk_size.y}
    }
}
//////////////






// [EVENTS]
#[derive(Event)]
pub struct GetChunk{ pub chunk: Chunk }

#[derive(Event)]
pub struct BrokeAsteroid( pub Entity );

#[derive(Event)]
pub struct SpawnBullet{ 
    pub transform: Transform, 
    pub velocity: Velocity,
    pub owner: u64 
}

#[derive(Event)]
pub struct SpawnAsteroid{ pub transform: Transform, pub velocity: Velocity, pub seed: u64}
///////////





// [COMPONENTS]
#[derive (Component)]
pub struct Object{id: u64, object_type: ObjectType}

#[derive (Component, Clone)]
pub struct Puppet{id: u64, binded_chunk: Chunk}

impl Puppet{
    pub fn empty() -> Self{
        return Puppet{id:0, binded_chunk: Chunk { pos: Vec2::ZERO }}
    }
}

#[derive (Component, Clone)]
pub struct Chunk{pub pos: Vec2}

#[derive (Component)]
pub struct Bullet{previous_position: Transform, spawn_time: f32, owner: u64}


#[derive (Component)]
pub struct Ship;

#[derive (Component)]
pub struct ControlledPlayer;

#[derive (Component)]
pub struct Debug;

#[derive (Component)]
pub struct PuppetPlayer;

#[derive (Component)]
pub struct Asteroid{
    pub seed: u64,
    pub hp: u64,
}
//////

pub fn init(
    //mut commands: Commands,
    mut spawn_asteroid_event: EventWriter<SpawnAsteroid>,
    map: ResMut<MapSettings>,
){ 
    let asteroid_count = 10;
    for _ in 0..asteroid_count{
        spawn_asteroid_event.send(SpawnAsteroid {
            transform: Transform::from_xyz(
                random::<f32>() * map.max_size.x * map.single_chunk_size.x,
                random::<f32>() * map.max_size.y * map.single_chunk_size.y, 
            0.),
            velocity: Velocity{
                linvel: Vec2{
                    x: (random::<f32>() - 0.5) * 100.,
                    y: (random::<f32>() - 0.5) * 100.,
                },
                angvel: (random::<f32>() - 0.5) * 10.
            },
            seed: random::<u64>()
        });
    }

    //commands.insert_resource(MapSettings{last_id: 0, max_size: Vec2{x: 0., y: 0.}});
    //spawn_asteroid_event.send(SpawnAsteroid { transform: Transform::from_xyz(550., 100., 0.), velocity: Velocity::linear(Vec2{x: 200., y: 500.}), seed: 256, puppet: Puppet::empty() });
    //spawn_asteroid_event.send(SpawnAsteroid { transform: Transform::from_xyz(100., 100., 0.), velocity: Velocity::zero(), seed: 256, puppet: Puppet::empty() });
    //spawn_asteroid_event.send(SpawnAsteroid { transform: Transform::from_xyz(150., 150., 0.), velocity: Velocity::zero(), seed: 256, puppet: Puppet::empty() });
    
    //println!("2 % 3 = {} (2)",  2 % 3);
    //println!("-1 % 3 = {} (2)",  -1_f32.rem_euclid(3.));
    //println!("-1 % 3 = {} (2)",  (-1 as f32).rem_euclid(3.));

}


pub fn spawn_debug(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
){
    let font = asset_server.load("fonts/F77MinecraftRegular-0VYv.ttf"); //REUSE
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::GRAY,
    };
    commands.spawn((
        TextBundle::from_sections([
            TextSection::from_style(TextStyle {
                font: font.clone(),
                font_size: 30.0,
                color: Color::DARK_GRAY,
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            bottom: Val::Px(5.0),
            right: Val::Px(15.0),
            flex_direction: FlexDirection::Column,
            ..default()
        }),
        Debug,
    ));
}

pub fn update_debug(
    time: Res<Time>,
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<Debug>>,
) {
    for mut text in &mut query {
        let mut fps = 0.0;
        if let Some(fps_diagnostic) = diagnostics.get(FrameTimeDiagnosticsPlugin::FPS) {
            if let Some(fps_smoothed) = fps_diagnostic.smoothed() {
                fps = fps_smoothed;
            }
        }

        text.sections[0].value = format!(
            "{fps:.1} fps",
        );

        //text.sections[2].value = format!("{fps:.1}");

        //text.sections[4].value = format!("{frame_time:.3}");
    }
}

pub fn spawn_ship(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
    mut map_settings: ResMut<MapSettings>,
){
    let mut ship = Mesh::new(PrimitiveTopology::TriangleList);
    //
    //     0
    //
    //     *
    //     
    //  1     3
    //     2
    let v_pos = vec![
        [0.0, 12., 0.0],    // 0
        [-7., -5., 0.0], // 1
        [0.0, -8., 0.0],   // 2
        [7., -5., 0.0],  // 3
    ];
    ship.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);

    /* SKINS?
    ship.insert_attribute(Mesh::ATTRIBUTE_UV_0, vec![[0.0, 0.3],    // 0
    */
    //let texture_handle = asset_server.load("a.png");
    
    ship.set_indices(Some(Indices::U32(vec![0, 1, 2, 2, 3, 0])));
    
    ship.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::WHITE.as_rgba_f32(); 4]);
    commands.spawn((
        RigidBody::Dynamic,
        TransformBundle::from(Transform::from_xyz(0.0, 5.0, 0.0)), // SPAWN POSITION
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
            id: map_settings.new_id(),
            object_type: ObjectType::Ship
        },
    )).insert(MaterialMesh2dBundle { //MESH
            mesh: Mesh2dHandle(meshes.add(ship)),
            transform: Transform::from_translation(Vec3::new(0., 0., 0.)),
                //.with_scale(Vec3::splat(32.)),
            material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
            ..default()
        },
    );
}

pub fn player_movement(
    mut player_data: Query<(&mut Velocity, &Transform, &Object), (With<ControlledPlayer>, Without<Camera>)>,
    mut spawn_bullet_event: EventWriter<SpawnBullet>,
    mut chunk_event: EventWriter<GetChunk>,
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    
    window: Query<&mut Window>,
    mut camera_translation: Query<&mut Transform, (With<Camera>, With<MainCamera>, Without<Object>)>,
    mut map_settings: ResMut<MapSettings>,

    // query to get camera transform
    camera_q: Query<(&Camera, &GlobalTransform), (With<Camera>, Without<MainCamera>)>,
){

    /*
    INPUTS:
                          ________________________________________
                         | <W / S> for forward / backward movement  
                         | <Q / E> for left / righlt movement (UNUSED!!!)
                         | <A / D> for rotate yourself
                         |
                         v
                [.] . . . . . . . . . . . .  [ .... ]
                [..] {q W e}. . . . . . . . . [ ... ]
                [...] {A S D}. . . . . . . . [ .... ]
            .-> [LSHIFT] . . . . . . . . . . [RSHIFT] <-.
       ____/    [..][.][..] -  - SPACE -  - ][..][..]    \_____
       DASH                        ^                      BRAKE
                             _____/
                             FIRE!
    
    
    
    //let delta = time.delta_seconds();
    let mut target_direction = Vec2::ZERO;
    if keys.pressed(KeyCode::W){target_direction.y += 1.0;}
    if keys.pressed(KeyCode::S){target_direction.y -= 0.5;}
    if keys.pressed(KeyCode::A){target_direction.x -= 1.0;}
    if keys.pressed(KeyCode::D){target_direction.x += 1.0;}
    


    

    if keys.pressed(KeyCode::ShiftLeft){//DASH
        vel.linvel = Vec2::from((transform.up().x, transform.up().y)) * 300.;
        vel.angvel = 0.;
    }

    vel.linvel += Vec2::from((transform.up().x, transform.up().y)) * target_direction.y * 2.0;
    vel.angvel += -target_direction.x * 0.2;

    if keys.pressed(KeyCode::ShiftRight){ // BRAKE
        vel.linvel = vel.linvel * 0.97;
    }

    




    //let delta = time.delta_seconds();
    //camera_translation.single_mut().translation = camera_translation.single_mut().translation.lerp(transform.translation, delta * 5.);
    
    
    */

    
    
    if keys.just_pressed(KeyCode::F3){
        map_settings.debug_render = !map_settings.debug_render;
    }

    let (mut vel, transform, object) = player_data.single_mut();


    // get the window that the camera is displaying to (or the primary window)
    let window = window.single();
    
    // check if the cursor is inside the window and get its position
    // then, ask bevy to convert into world coordinates, and truncate to discard Z
    
    let mut target_direction = Vec2::ZERO;

    if let Ok(t) = camera_q.get_single(){
        let (camera, camera_transform) = t;
        if buttons.pressed(MouseButton::Right){
            if let Some(world_position) = window.cursor_position()
                .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
                .map(|ray| ray.origin.truncate())
            {
                let target_vector = world_position;// - Vec2{x: transform.translation.x, y: transform.translation.y}; 
                let pos = Vec2{x: transform.up().x, y: transform.up().y};
                let target_angle = (target_vector - pos).angle_between(pos);
                if !target_angle.is_nan(){
                    /* 
                    if angle.abs() > 0.1{
                        vel.angvel = -angle.signum() * 10.;
                    } else {
                        vel.angvel = -angle;
                    }*/
                    vel.angvel = -target_angle * 5.
                }
            }
        }
    }
    if keys.pressed(KeyCode::W){target_direction.y += 2.0;} //  || buttons.pressed(MouseButton::Right
    if keys.pressed(KeyCode::S){target_direction.y -= 0.5;}
    if keys.pressed(KeyCode::A){target_direction.x -= 1.0;}
    if keys.pressed(KeyCode::D){target_direction.x += 1.0;}
    vel.linvel += Vec2::from((transform.up().x, transform.up().y)) * target_direction.y * 2.0;
    vel.linvel += Vec2::from((transform.right().x, transform.right().y)) * target_direction.x * 2.0;
    


    let chunk_position = Vec2{x: (transform.translation.x / map_settings.single_chunk_size.x).floor(), y: (transform.translation.y / map_settings.single_chunk_size.y).floor() };

    camera_translation.single_mut().translation = transform.translation; //+ Vec3{x: vel.linvel.x, y: vel.linvel.y, z: 0.} / 50.0 ;
    chunk_event.send(GetChunk { chunk: Chunk{ pos: chunk_position}});
    
    if keys.just_pressed(KeyCode::Space){//FIRE 
        spawn_bullet_event.send(SpawnBullet{transform: transform.clone().into(), owner: object.id, velocity: *vel});
        //spawn_bullet()
    }



}


pub fn spawn_bullet(
    mut map_settings: ResMut<MapSettings>,
    mut events: EventReader<SpawnBullet>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    time: Res<Time>,
){
    let mut mesh = Mesh::new(PrimitiveTopology::LineList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0., 0., 0.,], [0., -50., 0.,]]);
    mesh.set_indices(Some(Indices::U32(vec![0, 1])));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::Rgba { red: 3., green: 3., blue: 3., alpha: 3. }.as_rgba_f32() ; 2]);
    for t in events.iter(){
        let vel = t.transform.up() * 1000.;
        let vel = Vec2 { x: vel.x, y: vel.y } + t.velocity.linvel;
        //let vel = Vec2{x: 0.0, y: 0.0};
        let pos = t.transform.translation + t.transform.up() * 30. + Vec3{x: t.velocity.linvel.x, y: t.velocity.linvel.y, z: 0.} / 50.; 
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
            id: map_settings.new_id(),
            object_type: ObjectType::Bullet
        },
        Bullet{previous_position: Transform::from_translation(pos), spawn_time: time.elapsed().as_secs_f32(), owner: t.owner}
        ))
        .insert(MaterialMesh2dBundle {
            mesh: Mesh2dHandle(meshes.add(mesh.clone())),
            material: materials.add(ColorMaterial::default()),
            transform: Transform::from_matrix(Mat4::from_rotation_translation(t.transform.rotation, pos)),
            ..default()})
        //.insert(TransformBundle::from(Transform::from_translation(pos)))
        ;
    }
}

pub fn get_asteroid_size(seed: u64) -> i32{
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
     match rng.gen_range(0..16) {
        0..=6 => 1,
        7..=14 => 2,
        15..=16 => 3,
        e => {println!("{}", e); 1}
    }
}

pub fn generate_asteroid_vertices(seed: u64) -> (Vec<[f32; 3]>, Vec<u32>) {
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

pub fn prepate_for_polyline(vec: Vec<[f32; 3]>, ind: Vec<u32>) -> (Vec<Vect>, Vec<[u32; 2]>) {
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

pub fn _prepate_for_trimesh(vec: Vec<[f32; 3]>, ind: Vec<u32>) -> (Vec<Vect>, Vec<[u32; 3]>) {
    let mut vertices: Vec<Vect> = Vec::new();
    let mut indexes = Vec::new();
    vertices.push(Vect::from((0., 0.)));
    for i in 0..ind.len(){
        if i * 2 + 1 >= ind.len() {break}
        indexes.push([0, 1 + ind[i*2] , 1 + ind[i*2 + 1]]);  
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
    asset_server: Res<AssetServer>,
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
                coefficient: 0.0,
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

pub fn check_bullet_collisions_and_lifetime(
    mut bullets_data: Query<(Entity, &Transform, &mut Bullet), (With<Bullet>, Without<Puppet>)>,
    rapier_context: Res<RapierContext>,
    mut commands: Commands,
    mut query_asteroid: Query<(&mut Asteroid, &Velocity), Without<Puppet>>,
    mut query_ship: Query<&mut Object, With<Ship>>,
    mut spawn_asteroid_event: EventWriter<SpawnAsteroid>,
    time: Res<Time>
){
    for (bullet_entity, transform,  mut bullet) in bullets_data.iter_mut() {

        // HANDLE COLLISIONS
        let previous_pos = bullet.previous_position.translation;
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
                if let Ok((mut e, velocity)) = query_asteroid.get_mut(entity){
                    if e.hp != 0{
                        e.hp = e.hp - 1;
                    }
                    
                    commands.entity(bullet_entity).despawn_recursive();
                    if e.hp <= 0{
                        commands.entity(entity).despawn_recursive();                        
                        /*
                             |
                             v
                        o <- O -> o
                         SPLIT ASTEROID
                        */

                        let current_size = get_asteroid_size(e.seed);
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
                            spawn_asteroid_event.send(SpawnAsteroid{
                                transform: Transform::from_translation(transform.translation + dir1 * current_size as f32 * 5.),
                                velocity: vel1,
                                seed: new_seed_1
                            });
                            spawn_asteroid_event.send(
                                SpawnAsteroid{
                                    transform: Transform::from_translation(transform.translation + dir2 * current_size as f32 * 5.),
                                    velocity: vel2,
                                    seed: new_seed_2
                            });
                        }
                    };
                    return true
                    //commands.entity(*bullet).despawn_recursive();
                } else if let Ok(e) = query_ship.get_mut(entity){ // check ownership
                    if e.id != bullet.owner { // check ownership
                        commands.entity(bullet_entity).despawn_recursive();
                        return true
                    }
                } else {
                    commands.entity(bullet_entity).despawn_recursive();
                    return true
                }
                false // Return `false` instead if we want to stop searching for other hits.
        });
        // UPDATE
        bullet.previous_position = *transform;
        //LIFETIME
        if time.elapsed().as_secs_f32() - bullet.spawn_time > BULLET_LIFETIME{
            commands.entity(bullet_entity).despawn_recursive();
        }
    }
    
}

pub fn _handle_collision_events(
    mut bullet_events: EventReader<CollisionEvent>, 
    mut ship_events: EventReader<ContactForceEvent>,
    mut commands: Commands,
    mut query_asteroid: Query<&mut Asteroid>,
){
    for event in bullet_events.iter(){
        //CollisionEvent 
        //if event ==    
        match event{
            CollisionEvent::Started(bullet, body, collision_type) => {
                println!("E1 {:?}", bullet);
                println!("E2 {:?}", body);
                println!("E2 {:?}", collision_type);
                
                if let Ok(mut e) = query_asteroid.get_mut(*body){
                    println!("ASTEROID with seed {}", e.seed);
                    e.hp = e.hp - 1;
                    if e.hp <= 0{
                        commands.entity(*body).despawn_recursive();
                    };
                    commands.entity(*bullet).despawn_recursive();
                }
            },
            _ => {}
        }
    }
    for event in ship_events.iter(){
        //CollisionEvent
        println!("COllision {:?}", event);
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

enum ObjectType{
    Asteroid,
    Bullet,
    Ship,
}

pub fn update_chunks_around(
    mut chunk_event: EventReader<GetChunk>,
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
    ship_q: Query<&Ship, (With<Object>, Without<Puppet>)>,
){
    
    let font = asset_server.load("fonts/F77MinecraftRegular-0VYv.ttf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::GRAY,
    };
    
   

    let radius = 1;

    for event in chunk_event.iter(){

        // GET CHUNKS AROUND
        let pos =  event.chunk.pos;
        let mut chunks_around: Vec<Vec2> = vec![];
        for x in -radius..radius+1{
            for y in -radius..radius+1{
                chunks_around.push(Vec2{x: pos.x + x as f32, y:pos.y + y as f32})
            }
        }
        
        // GET REAL CHUNKS AND DRAW OTHER CHUNKS FOR DEBUG
        let mut real_chunks: Vec<Vec2> = vec![];

        let mut existing_debug_chunks: Vec<Vec2> = vec![];
        for (c, _) in chunks_q.iter(){
            existing_debug_chunks.push(c.pos);
        }

        for chunk in chunks_around.iter(){
            let mut isreal = false;
            if map.chunk_to_real_chunk_v2(chunk) == *chunk{
                real_chunks.push(*chunk);
                isreal = true;
            }
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
        }
        // REMOVE UNUSED DEBUG CHUNKS
        for (chunk, entity) in chunks_q.iter(){
            if existing_debug_chunks.contains(&chunk.pos){
                commands.entity(entity).despawn_recursive();
            }
        }
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
            if !real_chunks.contains(&chunk){ // IF CHUNK IS SHADOW
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
                                        Name::new("ASTEROID"),
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
                                    let mut ship = Mesh::new(PrimitiveTopology::TriangleList);
                                    let v_pos = vec![
                                        [0.0, 0.4, 0.0],    // 0
                                        [-0.3, -0.3, 0.0], // 1
                                        [0.0, -0.5, 0.0],   // 2
                                        [0.3, -0.3, 0.0],  // 3
                                    ];
                                    ship.insert_attribute(Mesh::ATTRIBUTE_POSITION, v_pos);
                                    ship.set_indices(Some(Indices::U32(vec![0, 1, 2, 2, 3, 0])));
                                    ship.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::WHITE.as_rgba_f32(); 4]);
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
                                    )).insert(MaterialMesh2dBundle { //MESH
                                        mesh: Mesh2dHandle(meshes.add(ship)),
                                        transform: transform.with_translation(pos).with_scale(Vec3::splat(32.)),
                                        material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                                        ..default()
                                    },);
                                }
                            }



                            
                        }
                    }
                }
            }
        }
    }
    return ()
}

pub fn __update_chunks_around(
    mut chunk_event: EventReader<GetChunk>,
    mut spawn_asteroid_event: EventWriter<SpawnAsteroid>,

    mut commands: Commands,

    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,

    chunks_queue: Query<(&Chunk, Entity)>,
    map: ResMut<MapSettings>,

    mut puppet_asteroids: Query<(&mut Transform, &Object, &Puppet, &mut Velocity, Entity), (With<Asteroid>, With<Puppet>, Without<Ship>)>,
    mut asteroids: Query<(&Asteroid, &Transform, &Object, &Velocity, &Collider), (With<Asteroid>, Without<Puppet>, Without<Ship>)>,

    mut puppet_ships: Query<(&mut Transform, &Object, &Puppet, &mut Velocity, Entity), (With<Ship>, With<Puppet>, Without<Asteroid>)>,
    mut ships: Query<(&Ship, &Transform, &Object, &Velocity, &Collider), (With<Ship>, Without<Puppet>, Without<Asteroid>)>,
){
    
    let font = asset_server.load("fonts/F77MinecraftRegular-0VYv.ttf"); //REUSE
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::GRAY,
    };
    

    let radius = 2;

    for event in chunk_event.iter(){

        // GET CHUNKS AROUND
        let pos =  event.chunk.pos;
        let mut chunks_around: Vec<Vec2> = vec![];
        for x in -radius..radius+1{
            for y in -radius..radius+1{
                chunks_around.push(Vec2{x: pos.x + x as f32, y:pos.y + y as f32})
            }
        }
        
        // GET REAL CHUNKS AND DRAW OTHER CHUNKS FOR DEBUG
        let mut real_chunks: Vec<Vec2> = vec![];

        let mut existing_debug_chunks: Vec<Vec2> = vec![];
        for (c, _) in chunks_queue.iter(){
            existing_debug_chunks.push(c.pos);
        }

        for chunk in chunks_around.iter(){
            let mut isreal = false;
            if map.chunk_to_real_chunk_v2(chunk) == *chunk{
                real_chunks.push(*chunk);
                isreal = true;
            }
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
        }
        // REMOVE UNUSED DEBUG CHUNKS
        for (chunk, entity) in chunks_queue.iter(){
            if existing_debug_chunks.contains(&chunk.pos){
                commands.entity(entity).despawn_recursive();
            }
        }
        drop(existing_debug_chunks);

        // COLLECT ALL REAL ASTEROIDS 

        let mut real_asteroids_chunks: HashMap<(i64, i64), Vec<(&Asteroid, &Transform, &Object, &Velocity, &Collider)>> = HashMap::new();
        let mut real_asteroids: HashMap<u64, (&Asteroid, &Transform, &Object, &Velocity, &Collider)> = HashMap::new();

        for (asteroid, transform, object, velocity, collider) in asteroids.iter(){
            let real_chunk_pos = map.pos_to_real_chunk(&transform.translation);
            let key = (real_chunk_pos.x as i64, real_chunk_pos.y as i64);
            if real_asteroids_chunks.contains_key(&key){
                let data = real_asteroids_chunks.get_mut(&key).unwrap();
                data.push((asteroid, transform, object, velocity, collider));
            } else {
                real_asteroids_chunks.insert(key, vec![(asteroid, transform, object, velocity, collider)]);
            }
            real_asteroids.insert(object.id, (asteroid, transform, object, velocity, collider));
        }

        // GET NEED-TO-SHADOW-CHUNKS (NOT NEED) AROUND_CHUNKS - REAL_CHUNKS = SHADOW CHUNKS
        //map_settings.pos_to_real_chunk(&transform.translation) == map_settings.pos_to_chunk(&real_transform.translation) // ПРОВЕРКА СУЩЕСТВОВАНИЯ ОПРЕДЕЛЕННОГО АСТЕРОИДА В ОПРЕДЕЛЕННОМ ЧАНКЕ
        // COLLECT AND MOVE EXISTED PUPPETS AND DELETE NOT NEEDED 
        //                            id  chunk pos
        let mut existing_puppets: Vec<(u64, i64, i64)> = vec![];
        
        for (mut puppet_transform, puppet_object, puppet, mut puppet_velocity, puppet_entity) in puppet_asteroids.iter_mut(){
            
            let puppet_position_chunk = map.pos_to_chunk(&puppet_transform.translation);
            let key = (puppet_object.id, puppet_position_chunk.x as i64, puppet_position_chunk.y as i64);
            if real_asteroids.contains_key(&puppet_object.id) &&  // COND 1 ( EXISTING OF REAL ASTEROID )
            chunks_around.contains(&puppet_position_chunk) &&    // COND 2 ( EXISTING OF REAL CHUNK )
            puppet_position_chunk == puppet.binded_chunk.pos && // COND 3 ( STILL IN THEIR SHADOW CHUNK? )
            !existing_puppets.contains(&key) &&                // COND 4 ( DOES PUPPET ALREADY EXISTS )
                                                              // COND 5 ( DOES REAL ASTEROID IN BINDED CHUNK )
            map.pos_to_real_chunk(&puppet_transform.translation) == map.pos_to_chunk(&real_asteroids.get(&puppet_object.id).unwrap().1.translation)
            { 
                // APPLY REAL ASTEROID's TRANSFORMS
                existing_puppets.push(key);
                let (_, transform, _, velocity, _) = real_asteroids.get(&puppet_object.id).unwrap();
                let offset = map.chunk_to_offset(&puppet_position_chunk);
                puppet_transform.translation = (transform.translation % Vec3 { x: map.single_chunk_size.x, y: map.single_chunk_size.y, z: 1. }) + Vec3{x: offset.x, y: offset.y, z:0.};
                puppet_transform.rotation = transform.rotation;
                puppet_velocity.angvel = velocity.angvel;
                puppet_velocity.linvel = velocity.linvel;
            } else {
                //println!("COND1 {}", real_asteroids.contains_key(&puppet_object.id));
                //println!("COND2 {}", chunks_around.contains(&puppet_position_chunk));
                //println!("COND3 {}", puppet_position_chunk == puppet.binded_chunk.pos);
                //println!("COND4 {} {:?}", existing_puppets.contains(&key), key);
                //println!("COND5 {}", map.pos_to_real_chunk(&puppet_transform.translation) == map.pos_to_chunk(&real_asteroids.get(&puppet_object.id).unwrap().1.translation));
                // DESPAWN
                commands.entity(puppet_entity).despawn_recursive();
            }
        }
        //println!("{:?}", existing_puppets);

        // SPAWN NEW
        for chunk in chunks_around.iter(){
            if !real_chunks.contains(&chunk){ // IF CHUNK IS SHADOW
                let real_chunk = map.chunk_to_real_chunk_v2(chunk);
                let key = &(real_chunk.x as i64, real_chunk.y as i64);
                if real_asteroids_chunks.contains_key(key){ // IF ASTEROIDS IN CHUNK
                    for (asteroid, transform, object, velocity, collider) in real_asteroids_chunks.get(key).unwrap(){
                        if !existing_puppets.contains(&(object.id, chunk.x as i64, chunk.y as i64)){ // IF NOT ALREADY EXISTS

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
                                Name::new("ASTEROID"),
                                Asteroid{seed: seed, hp: 1 }, // TAG get_asteroid_size(seed) * 2 - 1 
                                Puppet {
                                    id: object.id,
                                    binded_chunk: Chunk {
                                        pos: *chunk
                                    }
                                },
                                Object{id: object.id,
                                object_type: ObjectType::Asteroid},
                                Collider::from(collider.raw.clone())
                            )).insert(MaterialMesh2dBundle { //MESH
                                mesh: Mesh2dHandle(meshes.add(mesh)),
                                transform: transform.with_translation(
                                    (transform.translation % Vec3 { x: map.single_chunk_size.x, y: map.single_chunk_size.y, z: 1. }) + // INCHUNK OFFSET
                                    Vec3{x: chunk.x * map.single_chunk_size.x, y: chunk.y * map.single_chunk_size.y, z: 0.             // CHUNK OFFSET
                                }).with_scale(Vec3::splat(2.)),
                                material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                                ..default()
                            },);
                            /*spawn_asteroid_event.send(SpawnAsteroid{
                                transform: transform.with_translation(
                                    (transform.translation % Vec3 { x: map.single_chunk_size.x, y: map.single_chunk_size.y, z: 1. }) + // INCHUNK OFFSET
                                    Vec3{x: chunk.x * map.single_chunk_size.x, y: chunk.y * map.single_chunk_size.y, z: 0.             // CHUNK OFFSET
                                }),
                                velocity: **velocity,
                                seed: asteroid.seed,
                                puppet: Puppet {
                                    id: object.id,
                                    binded_chunk: Chunk {
                                        pos: *chunk
                                    }
                                }
                            });*/
                        }
                    }
                }
            }
        }
    }
    return ()
}

//CLIENT ONLY
pub fn _update_chunks_around( 
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut chunk_event: EventReader<GetChunk>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    chunks: Query<(&Chunk, Entity)>,
    map_settings: ResMut<MapSettings>,

    mut spawn_asteroid_event: EventWriter<SpawnAsteroid>,

    mut puppet_asteroids: Query<(&Asteroid, &mut Transform, &Object, &Puppet, Entity), With<Puppet>>,
    mut asteroids: Query<(&Asteroid, &Transform, &Object), Without<Puppet>>,

    //mut puppet_objects: Query<(&Object, &Transform), With<Puppet>>,
    //mut objects: Query<(&Object, &Transform), Without<Puppet>>,
){

    let font = asset_server.load("fonts/F77MinecraftRegular-0VYv.ttf"); //REUSE
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 60.0,
        color: Color::GRAY,
    };
    let mut existing_chunks: Vec<Vec2> = vec![];
    for (chunk, _) in chunks.iter(){
        existing_chunks.push(chunk.pos);
    }

    let max_x = map_settings.max_size.x;
    let max_y = map_settings.max_size.y;
    let chunk_size_x = map_settings.single_chunk_size.x;
    let chunk_size_y = map_settings.single_chunk_size.y;
    let map_size_x = max_x * chunk_size_x;
    let map_size_y = max_y * chunk_size_y;

    

   
   
    for center_chunk in chunk_event.iter(){
        let mut real_chunks: Vec<Vec2> = vec![];
         // DEBUG RENDER
        
        
        
        let mut to_spawn: Vec<Vec2> = vec![];
        let radius = 3;
        for x in -radius..radius+1{
            for y in -radius..radius+1{
                to_spawn.push(Vec2 { 
                    x: x as f32 + center_chunk.chunk.pos.x,
                    y: y as f32 + center_chunk.chunk.pos.y,
                });
            }
        }
        
        for chunk in to_spawn.iter(){

            if 0. <= chunk.x && chunk.x < max_x && 0. <= chunk.y && chunk.y < max_y{
                real_chunks.push(*chunk)
            }

            if existing_chunks.contains(&chunk) {
                continue;
            }
            let x = map_settings.single_chunk_size.x;
            let y = map_settings.single_chunk_size.y;
            

            let real_chunk_pos = Vec2{x: chunk.x.abs() % max_x, y: chunk.y.abs() % max_y};


            let vec = vec![
                [-x/2. + 1., -y/2. + 1., 0.],
                [ x/2. - 1., -y/2. + 1., 0.],
                [ x/2. - 1.,  y/2. - 1., 0.],
                [-x/2. + 1.,  y/2. - 1., 0.],
            ];
            let ind = vec![0, 1, 1, 2, 2, 3, 3, 0];
            let mut mesh = Mesh::new(PrimitiveTopology::LineList);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec);
            mesh.set_indices(Some(Indices::U32(ind)));
            if *chunk == real_chunk_pos{
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::ORANGE_RED.as_rgba_f32(); 4]);
            } else {
                mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::DARK_GRAY.as_rgba_f32(); 4]);
            }
            
            
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
            .insert(Transform::from_xyz(x/2. + chunk.x * x, y/2. + chunk.y * y, -1.0)); // 250 is offset
            
        }

        for (chunk, e) in chunks.iter(){
            if ! to_spawn.contains(&chunk.pos){
                commands.entity(e).despawn()
            }
        }
        
        // DEBUG RENDER


        // CHECK NEEDED ASTEROIDS
        let mut real_asteroids: HashMap<u64, (&Asteroid, &Transform, &Object, Vec2)> = HashMap::new();
        let mut real_asteroids_chunks: HashMap<(i64, i64), Vec<(&Asteroid, &Transform, &Object)>> = HashMap::new();
        for (asteroid_data, transform, object) in asteroids.iter(){
            // POS TO CHUNK
            let chunk_position = Vec2{x: (transform.translation.x / map_settings.single_chunk_size.x).floor(), y: (transform.translation.y / map_settings.single_chunk_size.y).floor() };
            //println!("{:?}", chunk_position);
            //println!("{:?}", real_chunks);

            //if real_chunks.contains(&chunk_position){

            real_asteroids.insert(object.id, (asteroid_data, transform, object, chunk_position));
            if real_asteroids_chunks.contains_key(&(chunk_position.x as i64, chunk_position.y as i64)){
                let vec = real_asteroids_chunks.get_mut(&(chunk_position.x as i64, chunk_position.y as i64)).unwrap();
                vec.push((asteroid_data, transform, object));
            } else {
                real_asteroids_chunks.insert((chunk_position.x as i64, chunk_position.y as i64), vec![(asteroid_data, transform, object)]);
            }
           // }
            // IF CHUNK IS NEED -> SAVE DATA
        }
        // GET NEED-TO-SHADOW-CHUNKS
        let mut need_to_shadow_chunks: Vec<Vec2> = vec![];
        for c in to_spawn{
            if !real_chunks.contains(&c){
                need_to_shadow_chunks.push(c);
            }
        }
        
        //                      chunk: x    y   id
        let mut existed_puppets: Vec<(i64, i64, u64)> = vec![];


        
        
        
        
        // COLLECT EXISTED PUPPETS AND DELETE NOT NEEDED 
        for (asteroid_data, mut transform, object, puppet, puppet_entity) in puppet_asteroids.iter_mut(){
            // POS TO CHUNK
            let chunk_position = Vec2{x: (transform.translation.x / map_settings.single_chunk_size.x).floor(), y: (transform.translation.y / map_settings.single_chunk_size.y).floor() };
            
            if real_asteroids.contains_key(&object.id){ // IF SHADOWING OBJECT IS EXIST
                let (real_asteroid_data, real_transform, real_object, real_chunk_pos) = real_asteroids.get(&object.id).unwrap(); 
                if need_to_shadow_chunks.contains(&chunk_position) &&
                //&&  r_chunk_pos == &puppet.chunk.pos 
                !existed_puppets.contains(&(chunk_position.x as i64, chunk_position.y as i64, object.id)) &&//(WHO GARANTEED THAT PUPPET IS SINGLE FOR EVERY ONE CHUNK?) vvv
                chunk_position == puppet.binded_chunk.pos &&
                map_settings.pos_to_real_chunk(&transform.translation) == map_settings.pos_to_chunk(&real_transform.translation) // ПРОВЕРКА СУЩЕСТВОВАНИЯ ОПРЕДЕЛЕННОГО АСТЕРОИДА В ОПРЕДЕЛЕННОМ ЧАНКЕ
                {
                    
                    
                    transform.rotation = real_transform.rotation; 
                    //                                                 vv INCHUNK
                    transform.translation = (real_transform.translation % Vec3 { x: chunk_size_x, y: chunk_size_y, z: 1. }) + Vec3{x: chunk_position.x * map_settings.single_chunk_size.x, y: chunk_position.y * map_settings.single_chunk_size.y, z: 0.};
                    existed_puppets.push((chunk_position.x as i64, chunk_position.y as i64, object.id));
                } else {
                    
                // println!("COND1 {}", real_asteroids.contains_key(&object.id));
                    //println!("COND2 {} DATA: {:?} {:?}", need_to_shadow_chunks.contains(&chunk_position), need_to_shadow_chunks, chunk_position);
                    //println!("COND3 {} DATA: {:?} {:?} {:?}", !existed_puppets.contains(&(chunk_position.x as i64, chunk_position.y as i64, object.id)), existed_puppets, (chunk_position.x as u64, chunk_position.y as u64 as u64, object.id), (chunk_position.x, chunk_position.y, object.id));
                    //println!("COND4 {}", chunk_position == puppet.chunk.pos);
                    
                    commands.entity(puppet_entity).despawn()
                }
            }
        }


        
        //println!();
        // SPAWN NEW
        for chunk in need_to_shadow_chunks.iter(){
            let original_chunk_pos = Vec2{x: chunk.x.abs() % max_x, y: chunk.y.abs() % max_y};
            //println!("{:?} {:?}", original_chunk_pos, chunk);
            let key = &(original_chunk_pos.x as i64, original_chunk_pos.y as i64);
            if real_asteroids_chunks.contains_key(key){
                for original_asteroid in real_asteroids_chunks.get(key).unwrap().iter(){
                    let (data, transform, object) = original_asteroid;

                    if !existed_puppets.contains(&(chunk.x as i64, chunk.y as i64, object.id)){ // CHECK IF EXISTS
                        /*spawn_asteroid_event.send(SpawnAsteroid{
                            transform: transform.with_translation((transform.translation % Vec3 { x: chunk_size_x, y: chunk_size_y, z: 1. }) + Vec3{x: chunk.x * chunk_size_x, y: chunk.y * chunk_size_y, z: 0.}),
                            velocity: Velocity::zero(),
                            seed: data.seed,
                            puppet: Puppet {
                                id: object.id,
                                binded_chunk: Chunk {
                                    pos: *chunk
                                }
                            }
                        });*/
                    }
                }
            }
        }
    }
    


/*
    let mut asteroids_to_spawn: HashMap<(u64, u64), (&Asteroid, &Transform, &Object)> = HashMap::new();
    let mut asteroids_to_exist: Vec<u64> = vec![];
    // CHECK EXISTING OBJECTS
    for (asteroid_data, transform, object) in asteroids.iter(){
        let real_chunk_pos = Vec2{x: transform.translation.x.abs() % max_x, y: transform.translation.y.abs() % max_y};
        asteroids_to_spawn.insert((real_chunk_pos.x as u64, real_chunk_pos.y as u64), (asteroid_data, transform, object));

        //asteroids_to_spawn.push((object, asteroid_data, transform));
        asteroids_to_exist.push(object.id);
    }

    let mut existing_asteroid_puppets: HashMap<(u64, u64, u64), (&Asteroid, Mut<Transform>, &Puppet)> = HashMap::new();
    // PARSE PUPPETS BY CHUNKS AND DELETE UNUSED PUPPETS 
    for (asteroid_data, transform, object, puppet, puppet_entity) in puppet_asteroids.iter_mut(){
        let real_chunk_asteroid_pos = Vec2{x: transform.translation.x.abs() % (max_x * chunk_size_x), y: transform.translation.y.abs() % (max_y * chunk_size_y)};
        if !asteroids_to_exist.contains(&object.id) {
            commands.entity(puppet_entity).despawn();
        } else {

            existing_asteroid_puppets.insert((puppet.chunk.pos.x as u64, puppet.chunk.pos.y as u64, object.id), (asteroid_data, transform, puppet));
            /* 
            let real_chunk_pos = Vec2{x: transform.translation.x.abs() % max_x, y: transform.translation.y.abs() % max_y};
            let key = (real_chunk_pos.x as u64, real_chunk_pos.y as u64);

            if !existing_asteroid_puppets.contains_key(&key){
                existing_asteroid_puppets.insert(key, vec![(asteroid_data, transform, object)]);
            } else {
                
                let vec = existing_asteroid_puppets.get_mut(&key).unwrap(); // TEST
                vec.push((asteroid_data, transform, object));

            }*/
        }
    }

    

    // SPAWN NEW PUPPETS AND UPDATE EXISTED POSITIONS
    for chunk in existing_chunks.iter(){

        if real_chunks.contains(chunk){continue;}

        let real_chunk_pos = Vec2{x: chunk.x.abs() % max_x, y: chunk.y.abs() % max_y};
        
        let key = (real_chunk_pos.x as u64, real_chunk_pos.y as u64);
        /* 
        //let already_exising_asteroid_puppets = existing_asteroid_puppets.get(&key);
        let mut already_exising_asteroid_puppets: &Vec<(&Asteroid, &Transform, &Object)> = &vec![];
        if existing_asteroid_puppets.contains_key(&key){
            already_exising_asteroid_puppets = existing_asteroid_puppets.get(&key).unwrap();
        }*/
        


        for (asteroid_data, transform, original_object) in asteroids_to_spawn.get(&key).iter_mut() {
            
            if existing_asteroid_puppets.contains_key(&(key.0, key.1, original_object.id)){ // UPDATE POS
                let (_asteroid, puppet_transform, puppet) = existing_asteroid_puppets.get_mut(&(key.0, key.1, original_object.id)).unwrap(); // BY CHUNK
                //puppet_transform.translation = transform.translation + Vec3{x: puppet.chunk.pos.x * chunk_size_x, y: puppet.chunk.pos.y * chunk_size_y, z: 0.};
                //puppet_transform.translation = transform.translation + Vec3{x: chunk.x * chunk_size_x, y: chunk.y * chunk_size_y, z: 0.};
            } else { // SPAWN NEW
                
                spawn_asteroid_event.send(SpawnAsteroid{
                    transform: transform.with_translation(transform.translation + Vec3{x: chunk.x * chunk_size_x, y: chunk.y * chunk_size_y, z: 0.}),
                    velocity: Velocity::zero(),
                    seed: asteroid_data.seed,
                    puppet: Puppet {
                        id: original_object.id,
                        chunk: Chunk {
                            pos: Vec2 { x: key.0 as f32, y: key.1 as f32 }
                        }
                    }
                });
            }
        }
    }
*/
}
