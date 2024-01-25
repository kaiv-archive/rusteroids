use std::f32::consts::PI;

use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};
use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite::{Mesh2dHandle, MaterialMesh2dBundle};
use bevy::window::WindowResized;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::rapier::geometry::ColliderEnabled;
use weighted_rand::builder::*;
use rand::Rng;



#[path = "game.rs"] mod game;
use game::*;
//use game::components::*;

use bevy_rapier2d::dynamics::Velocity;
use bevy_rapier2d::plugin::{RapierPhysicsPlugin, NoUserData};
use bevy_rapier2d::render::RapierDebugRenderPlugin;
use bevy_rapier2d::prelude::*;



fn main(){
    let mut app = App::new();
    //let default_settings = settings::GameSettings::init();
    //app.insert_resource(default_settings);
    app.add_plugins((
        DefaultPlugins.set(
                ImagePlugin::default_nearest(),
        ).set(WindowPlugin{ primary_window: Some(Window{ present_mode: bevy::window::PresentMode::AutoVsync, ..default() }), ..default()},),
        WorldInspectorPlugin::new(),
        RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0),
        RapierDebugRenderPlugin{enabled: false, ..default()},
        //FrameTimeDiagnosticsPlugin,
        //LogDiagnosticsPlugin::default(),
    ));


    app.add_systems(
        Startup,
        (
            setup,
        )
    );
    app.add_systems(
        Update, 
        (
            handle_inputs,
            update,
            starfield_update,
            camera_follow,
        ).chain()
    );
    app.insert_resource(GameSettings::default());
    app.insert_resource(InputKeys::default());
    app.insert_resource(InputType::Mouse);
    app.insert_resource(GlobalConfig::default());
    game::init_pixel_camera(&mut app);

    app.run()
}

#[derive(Component)]
struct CameraFollow;

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cfg: ResMut<GlobalConfig>,
    mut window: Query<&mut Window>,
    time: Res<Time>
){
    window.single_mut().resolution.set(1280., 720.);
    
    let player_data = ClientData::for_spawn(0, Color::WHITE, cfg.new_id());
    let e = spawn_ship(false, &mut meshes, &mut materials, &mut commands, &player_data, &mut cfg, &time);

    commands.entity(e).insert(CameraFollow);
    let mut seed = rand::random();
    while crate::game::get_asteroid_size(seed) != 3 {
        seed = rand::random();
    }
    //game::spawn_asteroid(seed, Velocity::zero(), Transform::from_translation(Vec3::splat(3.)), &mut meshes, &mut materials, &mut commands, cfg.new_id(), cfg.get_asteroid_hp(seed));
    // spawn room
    let room_size = Vec2{x: 1600., y: 1200.,};
    let thickness = 10.;
    let vec = vec![
        [-room_size.x / 2., -room_size.y / 2., 0.],
        [room_size.x / 2., -room_size.y / 2., 0.],
        [room_size.x / 2., room_size.y / 2., 0.],
        [-room_size.x / 2., room_size.y / 2., 0.],
    ];
    let ind = vec![0, 1, 1, 2, 2, 3, 3, 0];
    let mut mesh = Mesh::new(PrimitiveTopology::LineList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec);
    mesh.set_indices(Some(Indices::U32(ind)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::WHITE.as_rgba_f32(); 4]);
    
    commands.spawn(MaterialMesh2dBundle { //MESH
        mesh: Mesh2dHandle(meshes.add(mesh)),
        transform: Transform::from_scale(Vec3::splat(1.)),
        material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
        ..default()
    });
    

    let bundle = (
        RigidBody::Fixed,
        Restitution {
            coefficient: 0.3,
            combine_rule: CoefficientCombineRule::Average,
        },
    );

    let x_bundle = (
        bundle.clone(),
        Collider::cuboid(room_size.x / 2., thickness / 2.),
        
    );
    let y_bundle = (
        bundle.clone(),
        Collider::cuboid(thickness / 2., room_size.y / 2.),
    );
    commands.spawn((
        x_bundle.clone(),
        Name::from("Bottom"),
        Transform::from_xyz(0., 1000., 0.0),
    )).insert(TransformBundle::from(Transform::from_xyz(0.0, -(room_size.y  + thickness) / 2., 0.0)));
    commands.spawn((
        x_bundle,
        Name::from("Up"),
    )).insert(TransformBundle::from(Transform::from_xyz(0.0,( room_size.y + thickness) / 2., 0.0)));
    commands.spawn((
        y_bundle.clone(),
        Name::from("Right"),
    )).insert(TransformBundle::from(Transform::from_xyz(-(room_size.x + thickness) / 2., 0.0, 0.0)));
    commands.spawn((
        y_bundle,
        Name::from("Left"),
    )).insert(TransformBundle::from(Transform::from_xyz((room_size.x + thickness) / 2., 0.0, 0.0)));

}


fn handle_inputs(
    mut inp: ResMut<InputKeys>,
    mut player_data: Query<(&mut Velocity, &Transform, &Object), With<CameraFollow>>, 
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    window: Query<&mut Window>,
    camera_q: Query<(&Camera, &GlobalTransform), (With<Camera>, Without<PixelCamera>)>,
    input_type: Res<InputType>
){
    let player_data = player_data.get_single_mut();
    if player_data.is_err(){
        return;
    };
    let (mut vel, transform, object) = player_data.unwrap();
    inp.up = false;
    inp.down = false;
    inp.left = false;
    inp.right = false;
    inp.rotate_left = false;
    inp.rotate_right = false;
    inp.stabilize = false;
    inp.shoot = false;
    inp.dash = false;
    inp.rotation_target = Vec2::ZERO;
    match *input_type{
        InputType::Keyboard => {
            inp.fixed_camera_z = false;
            if keys.pressed(KeyCode::W){inp.up = true} // || buttons.pressed(MouseButton::Right
            if keys.pressed(KeyCode::S){inp.down = true}
            if keys.pressed(KeyCode::A){inp.rotate_left = true}
            if keys.pressed(KeyCode::D){inp.rotate_right = true}
            if keys.pressed(KeyCode::K){inp.left = true}
            if keys.pressed(KeyCode::L){inp.right = true}

            if keys.just_pressed(KeyCode::ShiftLeft){//DASH
                inp.dash = true;
            }

            if keys.just_pressed(KeyCode::Space){ //DASH
                inp.shoot = true;
            }

            if keys.pressed(KeyCode::ShiftRight){ // BRAKE
                inp.stabilize = true;
            }
        }
        InputType::Mouse => {
            inp.fixed_camera_z = true;
            if keys.pressed(KeyCode::W){inp.up = true} //  || buttons.pressed(MouseButton::Right
            if keys.pressed(KeyCode::S){inp.down = true}
            if keys.pressed(KeyCode::A){inp.left = true}
            if keys.pressed(KeyCode::D){inp.right = true}

            if keys.just_pressed(KeyCode::ShiftLeft){//DASH
                inp.dash = true;
            }

            if keys.just_pressed(KeyCode::Space){ //DASH
                inp.shoot = true;
            }

            if keys.pressed(KeyCode::ShiftRight){ // BRAKE
                inp.stabilize = true;
            }

            if let Ok(t) = camera_q.get_single(){
                let (camera, camera_transform) = t;
                if let Some(world_position) = window.single().cursor_position()
                    .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
                    .map(|ray| ray.origin.truncate())
                {
                    let target_vector = world_position;// - Vec2{x: transform.translation.x, y: transform.translation.y}; 
                    let pos = Vec2::Y;
                    inp.rotation_target = target_vector;
                    /*if !target_angle.is_nan(){
                        inp.rotation_target = -target_angle;
                        if target_angle < 0.{
                            inp.rotate_left = true;
                        } else {
                            inp.rotate_right = true;
                        }
                    }*/
                }
            }
        }
    }
}

fn camera_follow(
    player_data: Query<&Transform, (With<CameraFollow>, Without<Camera>)>,
    mut camera_translation: Query<&mut Transform, (With<Camera>, With<PixelCamera>, Without<Object>)>,
    input_type: Res<InputType>,
){
    let camera_translation = camera_translation.get_single_mut();
    let player_data = player_data.get_single();
    if camera_translation.is_err() || player_data.is_err(){
        return;
    };
    let mut camera_translation = camera_translation.unwrap();
    let player_data = player_data.unwrap();
    camera_translation.translation = player_data.translation;

    match *input_type{
        InputType::Keyboard => {
            camera_translation.rotation = player_data.rotation;
        }
        InputType::Mouse => {
            camera_translation.rotation = Quat::from_rotation_z(0.);
        }
    }
    
}

fn update(
    inp: Res<InputKeys>,
    mut res: Query<(&mut Velocity, &mut Transform), With<CameraFollow>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
    mut cfg: ResMut<GlobalConfig>,
    input_type: Res<InputType>,
){
    // INPUTS
    let (mut velocity, transform) = res.single_mut();
    
    let mut rotation_direction = 0.;
    if inp.rotate_left {
        rotation_direction += 1.;
    }
    if inp.rotate_right {
        rotation_direction -= 1.;
    }
    // let it be...
    let target = inp.rotation_target;
    let target_angle = transform.up().truncate().angle_between(target);
    if !target_angle.is_nan(){
        velocity.angvel += (target_angle.clamp(-1., 1.) * 180. / PI - velocity.angvel) * 0.5;//.clamp(-1.5, 1.5);
    }
    /*velocity.angvel += -k * velocity.angvel * (velocity.angvel / maxspeed) * ((inp.rotation_target * 180. / PI)/PI);
    //if velocity.angvel == 0.{velocity.angvel = 0.1};
    //velocity.angvel += -k * ();
    let speed = 0.2;
    if (velocity.angvel > 0.) == (rotation_direction > 0.){
        if velocity.angvel.abs() * 0.1 > inp.rotation_target.abs().pow(2) { // id
            //break
            println!("break! corr");
            velocity.angvel -= inp.rotation_target * speed;
        } else {
            //speed up
            println!("speed up!"); 
            velocity.angvel += inp.rotation_target * speed;
        }
    } else {
        velocity.angvel += inp.rotation_target * speed * 2.;
        println!("break! dir");
    }
    velocity.angvel += inp.rotation_target * speed;


    let rotate_speed: f32 = 5.;
    let mut rotation_direction = 0.;
    if inp.rotate_left {
        rotation_direction += 1.;
    }
    if inp.rotate_right {
        rotation_direction -= 1.;
    }

    

    if (velocity.angvel > 0.) == (rotation_direction > 0.){ // eq direction
        velocity.angvel = rotation_direction * rotate_speed;
        println!("speed");
    } else {
        velocity.angvel = rotation_direction * rotate_speed * 2.;
        println!("twice speed");
    }*/
    
    let mut target_direction = Vec2::ZERO;
    if inp.up    {target_direction.y += 1.5;} //  || buttons.pressed(MouseButton::Right
    if inp.down  {target_direction.y -= 0.75;}
    if inp.right {target_direction.x += 1.0;}
    if inp.left  {target_direction.x -= 1.0;}
    
    if inp.stabilize{
        velocity.linvel = velocity.linvel * 0.97;
        velocity.angvel = velocity.angvel * 0.97;
    }
    
    let target_vector = if inp.fixed_camera_z{
        target_direction * 2.
    } else {
        transform.up().truncate() * target_direction.y * 2.0 + transform.right().truncate() * target_direction.x * 2.0
    };
    
    velocity.linvel += target_vector;

    if inp.dash{
        //velocity.linvel = transform.up().truncate() * 3000.;
    }

    if inp.shoot{
        spawn_bullet(transform.up().truncate() * 1000. + velocity.linvel, *transform, cfg.new_id(), cfg.new_id(), 3000., &asset_server, &mut commands);
    }
    
    let max_linvel = 700.;
    if velocity.linvel.length_squared() > (max_linvel * max_linvel){
        velocity.linvel *= 0.8;
    }
    let max_angvel = 100.;
    if velocity.angvel.abs() > max_angvel{
        velocity.angvel *= 0.8;
    }
}



const STARFIELD_STARS : usize = 1500;

fn distance_distribution(x: f32) -> f32{
    x
    /*if x < 0.5{
        0.1 + x / 5.
    } else{
        (1. - (1. - x.powi(10)).powi(2)) * 0.9 + 0.2
    } */
}

#[derive(Component)]
struct Star{depth: f32}

#[derive(Component)]
struct StarsLayer;

#[derive(Clone, Copy)]
pub struct StarClass{
    size: (f32, f32),
    chance: f32,
    color: Color,
}

fn starfield_update(
    resize_event: Res<Events<WindowResized>>,
    mut commands: Commands,
    mut star_q: Query<(&mut Transform, &mut Sprite, &Star, Entity), (With<Star>, Without<StarsLayer>, Without<Camera>, Without<CameraFollow>)>,
    mut star_layer_q: Query<(&mut Transform, Entity), (With<StarsLayer>, Without<Star>, Without<Camera>, Without<CameraFollow>)>,
    keys: Res<Input<KeyCode>>,
    player: Query<(&Transform, &Velocity), (With<CameraFollow>, Without<Star>, Without<Camera>)>,
    asset_server: Res<AssetServer>,
    mut camera:  Query<(&Camera, &mut GlobalTransform), (With<Camera>, With<PixelCamera>, Without<StarsLayer>, Without<Star>, Without<CameraFollow>)>,
    time: Res<Time>,
    mut max_dist: Local<f32>,
    mut max_dist_squared: Local<f32>
){
    if camera.is_empty(){return;}
    let (camera, camera_global_transform) = camera.single_mut();
    let camera_global_transform = camera_global_transform.compute_transform();
    let padding = 10.;
    let mut reader = resize_event.get_reader();
    if reader.read(&resize_event).len() > 0 || *max_dist < 1. || keys.just_pressed(KeyCode::P){ // todo: fix bug with first frame; after using it in game it might fix itself.
        let window_size = camera.ndc_to_world(
            &GlobalTransform::from(camera_global_transform.with_rotation(Quat::from_axis_angle(Vec3::Z, 0.)).with_translation(Vec3::ZERO)),
            Vec3::ONE
        ).unwrap();
        let max_size = window_size.x.round().max(window_size.y.round());
        *max_dist_squared = 2. * (max_size.powi(2));
        *max_dist = max_dist_squared.sqrt();
    }
    let (player_transform, player_velocity) = player.single();
    if star_layer_q.get_single().is_ok(){
        star_layer_q.single_mut().0.translation = player_transform.translation;
    }
    for star_data in star_q.iter_mut(){
        let (mut transform, mut sprite, star, e) = star_data;
        let camera_transfrom = camera_global_transform.translation.truncate();
        let star_transform =  transform.translation;
        //let right_up_corner = camera_transfrom + Vec2::splat(*max_dist);
        //let left_down_corner = camera_transfrom - Vec2::splat(*max_dist);
        if star_transform.truncate().length_squared() < *max_dist_squared + padding{ // inside "keep" circle
            transform.translation += -player_velocity.linvel.extend(0.) * time.delta_seconds() * (0.1 + star.depth * 0.3);//
        } else {
            if rand::random::<f32>() < 0.1 { // some random
                commands.entity(e).remove_parent();
                commands.entity(e).despawn();
                /*sprite.color.set_a(rand::random::<f32>() * 0.5);
                transform.translation = //camera_global_transform.translation + 
                    Vec2::from_angle(
                        (player_velocity.linvel.normalize())
                            .angle_between(Vec2::X) * -1. + PI * rand::random::<f32>() - PI / 2.
                    ).extend(0.) * *max_dist;

                transform.rotation = Quat::from_axis_angle(Vec3::Z, PI * 2. * rand::random::<f32>());*/
            }
        }
        //let (mut star_transform, _, _) = star_q.get_mut(Entity::from_bits(*star)).unwrap();
    }
    let curr_stars_count = star_q.into_iter().len();
    let mut rng = rand::thread_rng();
    let texture_path = [
        "star1.png",
        "star2.png",
        "star3.png",
        "star4.png",
        "star5.png",
    ];
    let weak = 1.5;
    let medium = 2.;
    let bright = 5.;
    let insane = 5.;
    let star_classes = [
        StarClass{ // sapphire
            size: (1., 1.),
            chance: 0.1,
            color: Color::Rgba { red: 0.1, green: 0.15, blue: 1., alpha: 1. } * insane,
        },
        StarClass{ // amethyst
            size: (1., 1.),
            chance: 0.1,
            color: Color::Rgba { red: 1., green: 0.0, blue: 0.8, alpha: 1. } * insane,
        },
        StarClass{ // ruby
            size: (1., 1.),
            chance: 0.1,
            color: Color::Rgba { red: 1., green: 0.0, blue: 0.45, alpha: 1. } * insane,
        },
        StarClass{ // emerald
            size: (1., 1.),
            chance: 0.1,
            color: Color::Rgba { red: 0.2, green: 1., blue: 0.4, alpha: 1. } * insane,
        },
        StarClass{ // golden
            size: (1., 1.),
            chance: 0.1,
            color: Color::Rgba { red: 1., green: 0.8, blue: 0.2, alpha: 1. } * insane,
        },

        StarClass{ // weak white
            size: (1., 1.),
            chance: 10.,
            color: Color::Rgba { red: 1., green: 1., blue: 1., alpha: 1. } * weak,
        },
        StarClass{ // medium white
            size: (1., 1.),
            chance: 100.,
            color: Color::Rgba { red: 1., green: 1., blue: 1., alpha: 1. } * medium, 
        },
        StarClass{ // light purple
            size: (1., 1.),
            chance: 30.,
            color: Color::Rgba { red: 0.9, green: 0.8, blue: 1., alpha: 1. } * medium,
        },
        StarClass{ // light blue
            size: (1., 1.),
            chance: 200.,
            color: Color::Rgba { red: 0.60, green: 0.67, blue: 0.98, alpha: 1. } * weak, 
        },
        StarClass{ // red
            size: (0.5, 0.7),
            chance: 8.,
            color: Color::Rgba { red: 0.5, green: 0.2, blue: 0.2, alpha: 1. } * weak,
        },
        StarClass{ // orange
            size: (1., 1.),
            chance: 23.,
            color: Color::Rgba { red: 1., green: 0.8, blue: 0.5, alpha: 1. } * medium,
        },
        StarClass{ // yellow
            size: (1., 1.),
            chance: 15.,
            color: Color::Rgba { red: 1., green: 1., blue: 0.2, alpha: 1. } * medium,
        },
    ];
    /*let main_colors = [
        Color::Rgba {alpha: 1., red: 2., green: 2., blue: 2. },
        Color::Rgba {alpha: 1., red: 1.3, green: 1.1, blue: 0.7 },
        Color::Rgba {alpha: 1., red: 1.9, green: 0.3, blue: 0.2 },
        Color::Rgba {alpha: 1., red: 1.9, green: 1.9, blue: 1.1 },
        Color::Rgba {alpha: 1., red: 0.5, green: 0.7, blue: 6. },
    ];

    let index = match rand::random::<f32>(){
        n if n < 0.65 => {0}, // white
        n if n < 0.75 => {1}, // orange
        n if n < 0.85 => {2}, // red
        n if n < 0.99 => {3}, // yellow
        _ => {4}, // blue
    };
    let color = main_colors[index];*/
    let weights = &star_classes.map(|c| c.chance);
    let builder = WalkerTableBuilder::new(weights);
    let class_table = builder.build();

    if curr_stars_count < STARFIELD_STARS{ // todo: move to init and add varables to settings
        let layer = star_layer_q.get_single();
        let layer = if layer.is_ok(){
            layer.unwrap().1
        } else {
            commands.spawn((
                StarsLayer,
                TransformBundle::default(),
                VisibilityBundle::default(),
                Name::new("Stars Layer")
            )).id()
        };
        
        let diff = STARFIELD_STARS - curr_stars_count;
        let init_spawn = curr_stars_count == 0;
        for _ in 0..diff{
            let depth = distance_distribution(rand::random());
            
            let class_id = class_table.next();
            let class = (star_classes).get(class_id).unwrap();
            let color = class.color;
            let size_properties = class.size;
            let size = size_properties.0 + rand::random::<f32>() * (size_properties.1 - size_properties.0);

            let mut new_pos = Vec3::ZERO;
            if init_spawn {
                new_pos.x += 2. * *max_dist * rand::random::<f32>() - *max_dist;
                new_pos.y += 2. * *max_dist * rand::random::<f32>() - *max_dist;
            } else {
                new_pos = Vec2::from_angle(
                    (player_velocity.linvel.normalize())
                        .angle_between(Vec2::X) * -1. + PI * rand::random::<f32>() - PI / 2.
                ).extend(0.) * *max_dist;
            }
            
            commands.spawn((
                SpriteBundle {
                    transform: Transform::from_translation((new_pos) - Vec3::Z )
                        .with_rotation(Quat::from_axis_angle(Vec3::Z, PI / 2. * rand::random::<f32>()))
                        .with_scale(Vec3::splat(0.15 + depth * size * 0.45)),//0.11 + depth * size * 0.8
                        // 0.2 -> 0.65
                    texture: asset_server.load(texture_path[rng.gen_range(0..texture_path.len())]),
                    sprite: Sprite { color: color.with_a(depth * 0.35), ..default() }, // ADD RANDOM COLORS
                    ..default()
                },
                Star{depth: depth},
                Name::new("Star")
            )).set_parent(layer);
        }
        /*if curr_stars_count == 0{ // init spawn
            
        } else { // respawn
            for _ in 0..diff{
                let depth = distance_distribution(rand::random());
                
                let class_id = class_table.next();
                let class = (star_classes).get(class_id).unwrap();
                let color = class.color;
                let size_properties = class.size;
                let size = size_properties.0 + rand::random::<f32>() * (size_properties.1 - size_properties.0);

                let mut new_pos = Vec2::from_angle(
                    (player_velocity.linvel.normalize())
                        .angle_between(Vec2::X) * -1. + PI * rand::random::<f32>() - PI / 2.
                ).extend(0.) * *max_dist;
                
                commands.spawn((
                    SpriteBundle {
                        transform: Transform::from_translation((new_pos) - Vec3::Z )
                            .with_rotation(Quat::from_axis_angle(Vec3::Z, PI / 2. * rand::random::<f32>()))
                            .with_scale(Vec3::splat(0.15 + depth * size * 0.45)),
                        texture: asset_server.load(texture_path[rng.gen_range(0..texture_path.len())]),
                        sprite: Sprite { color: color.with_a(depth * 0.35), ..default() }, // ADD RANDOM COLORS
                        ..default()
                    },
                    Star{depth: depth},
                    Name::new("Star")
                )).set_parent(layer);
            }
        }*/
    }
    if keys.just_pressed(KeyCode::P){
        commands.entity(star_layer_q.single().1).despawn_recursive();
    }
    /*
    if *star != 0{
        
        //let pos = camera.viewport_to_world_2d(camera_transform, Vec2::from([window.width() / 1.5, window.height() / 1.5])).unwrap();
        
        //star_transform.translation = player.single().translation  + Vec3::from([max_size, max_size, 0.]);

    } else {
        let id = commands.spawn((
            SpriteBundle {
                transform: Transform::from_xyz(0., 0., 0.),
                texture: asset_server.load("star1.png"),
                ..default()
            },
            Star{depth: rand::random()}
        )).id();
        *star = id.to_bits();
    }
    */  
}
