use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite::{Mesh2dHandle, MaterialMesh2dBundle};
use bevy::window::WindowResized;
use bevy_inspector_egui::quick::WorldInspectorPlugin;




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
        ImagePlugin::default_nearest()
        ),
        WorldInspectorPlugin::new(),
        RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0), // ::<NoUserData>::pixels_per_meter(15.0)
        RapierDebugRenderPlugin{enabled: true, ..default()}
    ));


    app.add_systems(
        Startup,
        (
            setup,
    ));
    app.add_systems(
        Update, 
        
        (handle_inputs,
        update,
        camera_follow,
        starfield_update).chain()
    );
    app.insert_resource(GameSettings::default());
    app.insert_resource(Inputs::default());
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
){
    



    let player_data = ClientData::for_spawn(0, [1.; 3], cfg.new_id());
    let e = spawn_ship(false, &mut meshes, &mut materials, &mut commands, &player_data);

    commands.entity(e).insert(CameraFollow);

    //let seed = rand::random();
    //game::spawn_asteroid(seed, Velocity::zero(), Transform::from_translation(Vec3::splat(0.)), &mut meshes, &mut materials, &mut commands, cfg.new_id(), cfg.get_asteroid_hp(seed));







    // spawn room
    let room_size = Vec2{x: 800., y: 600.,};
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
            coefficient: 1.,
            combine_rule: CoefficientCombineRule::Multiply,
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
    /*commands.spawn((
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
    )).insert(TransformBundle::from(Transform::from_xyz((room_size.x + thickness) / 2., 0.0, 0.0)));*/

}

#[derive(Resource)]
pub struct Inputs{
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub rotate_left: bool,
    pub rotate_right: bool,
    pub sabilize: bool,
    pub shoot: bool,
    pub dash: bool,
}

impl Default for Inputs{
    fn default() -> Self {
        Inputs {
            up: false,
            down: false,
            left: false,
            right: false,
            rotate_left: false,
            rotate_right: false,
            sabilize: false,
            shoot: false,
            dash: false,
        }
    }
}

fn handle_inputs(
    mut inp: ResMut<Inputs>,
    mut player_data: Query<(&mut Velocity, &Transform, &Object), With<CameraFollow>>, 
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    window: Query<&mut Window>,
    camera_q: Query<(&Camera, &GlobalTransform), (With<Camera>, Without<PixelCamera>)>,
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
    inp.sabilize = false;
    inp.shoot = false;
    inp.dash = false;
    if keys.pressed(KeyCode::W){inp.up = true} //  || buttons.pressed(MouseButton::Right
    if keys.pressed(KeyCode::S){inp.down = true}
    if keys.pressed(KeyCode::A){inp.rotate_left = true}
    if keys.pressed(KeyCode::D){inp.rotate_right = true}
    if keys.pressed(KeyCode::L){inp.left = true}
    if keys.pressed(KeyCode::Semicolon){inp.right = true}

}

fn camera_follow(
    player_data: Query<&Transform, (With<CameraFollow>, Without<Camera>)>,
    mut camera_translation: Query<&mut Transform, (With<Camera>, With<PixelCamera>, Without<Object>)>,
){
    let camera_translation = camera_translation.get_single_mut();
    let player_data = player_data.get_single();
    if camera_translation.is_err() || player_data.is_err(){
        return;
    };
    let mut camera_translation = camera_translation.unwrap();
    let player_data = player_data.unwrap();
    camera_translation.translation = player_data.translation;
    camera_translation.rotation = player_data.rotation;
}

fn update(
    inp: Res<Inputs>,
    mut res: Query<(&mut Velocity, &mut Transform), With<CameraFollow>>
){
    


    // INPUTS
    let (mut velocity, transform) = res.single_mut();

    if inp.rotate_left {velocity.angvel += 0.05;}
    if inp.rotate_right {velocity.angvel -= 0.05;}
    

    //.clamp(-3., 3.) * 0.01;

    let mut target_direction = Vec2::ZERO;
    if inp.up    {target_direction.y += 1.5;} //  || buttons.pressed(MouseButton::Right
    if inp.down  {target_direction.y -= 0.75;}
    if inp.right {target_direction.x += 1.0;}
    if inp.left  {target_direction.x -= 1.0;}
    
    velocity.linvel += Vec2::from((transform.up().x, transform.up().y)) * target_direction.y * 2.0;
    velocity.linvel += Vec2::from((transform.right().x, transform.right().y)) * target_direction.x * 2.0;

    // dumping
    let max_linvel = 500.;
    if velocity.linvel.length_squared() > (max_linvel * max_linvel){
        velocity.linvel *= 0.9;
    }
    let max_angvel = 5.;
    //if velocity.angvel > max_angvel{
    //    velocity.angvel *= 0.9;
    //}
}

#[derive(Component)]
struct Star{depth: f32}

const STARFIELD_LAYERS : i8 = 32;
const STARFIELD_STARS : usize = 128;

fn starfield_update(
    mut resize_event: Res<Events<WindowResized>>,
    mut commands: Commands,
    mut star_q: Query<(&mut Transform, &mut Sprite), (With<Star>, Without<Camera>, Without<CameraFollow>)>,

    player: Query<(&Transform, &Velocity), (With<CameraFollow>, Without<Star>, Without<Camera>)>,

    asset_server: Res<AssetServer>,
    window: Query<&mut Window>,
    mut camera:  Query<(&Camera, &mut GlobalTransform), (With<Camera>, With<PixelCamera>, Without<Star>, Without<CameraFollow>)>,
    mut star: Local<u64>,
    mut max_dist: Local<f32>,
    mut max_dist_squared: Local<f32>
){
    let mut reader = resize_event.get_reader();
    let (camera, mut camera_global_transform) = camera.single_mut();
    let camera_global_transform = camera_global_transform.compute_transform();
    let padding = 10.;
    
    if reader.iter(&resize_event).len() > 0 ||
    star_q.iter().len() == 0{
        let window_size = camera.ndc_to_world(
            &GlobalTransform::from(camera_global_transform.with_rotation(Quat::from_axis_angle(Vec3::Z, 0.)).with_translation(Vec3::ZERO)),
            Vec3::ONE
        ).unwrap();
        let max_size = window_size.x.round().max(window_size.y.round());
        *max_dist_squared = 2. * (max_size.powf(2.));
        *max_dist = max_dist_squared.sqrt();
    }

    let (_, player_velocity) = player.single();

    for star_data in star_q.iter_mut(){
        let (mut transform, mut sprite) = star_data;

        let camera_transfrom = camera_global_transform.translation.truncate();
        let star_transform =  transform.translation;
        //let right_up_corner = camera_transfrom + Vec2::splat(*max_dist);
        //let left_down_corner = camera_transfrom - Vec2::splat(*max_dist);
        
        if camera_transfrom.distance_squared(star_transform.truncate()) < *max_dist_squared + padding{ // inside "keep" circle

        } else {
            if rand::random::<f32>() < 0.02{ // some random
                let mut new_pos = camera_global_transform.translation;
                if rand::random::<bool>(){ // choose a random side
                    new_pos.x += 2. * *max_dist * rand::random::<f32>() - *max_dist;
                    if player_velocity.linvel.y.is_sign_positive(){
                        new_pos.y += 1. * *max_dist;
                    } else {
                        new_pos.y -= 1. * *max_dist;
                    }
                } else {
                    new_pos.y += 2. * *max_dist * rand::random::<f32>() - *max_dist;
                    if player_velocity.linvel.x.is_sign_positive(){
                        new_pos.x += 1. * *max_dist;
                    } else {
                        new_pos.x -= 1. * *max_dist;
                    }
                }
            
                
                sprite.color.set_a(rand::random::<f32>() * 0.5);


                transform.translation = camera_global_transform.translation + 
                    Vec2::from_angle(
                        (player_velocity.linvel.normalize())
                            .angle_between(Vec2::X) * -1. + PI * rand::random::<f32>() - PI / 2.
                    ).extend(0.) * *max_dist;

                transform.rotation = Quat::from_axis_angle(Vec3::Z, PI * 2. * rand::random::<f32>());
            }
        }



        
        //let (mut star_transform, _, _) = star_q.get_mut(Entity::from_bits(*star)).unwrap();
            
        
    }
    let curr_stars_count = star_q.into_iter().len();
    
    if curr_stars_count < STARFIELD_STARS{
        for star in 0..STARFIELD_STARS{
            let texture_path = [
            "star1.png",
            "star2.png",
            "smoothstar.png",
        ]; 
        let mut new_pos = Vec3::ZERO;
        new_pos.x += 2. * *max_dist * rand::random::<f32>() - *max_dist;
        new_pos.y += 2. * *max_dist * rand::random::<f32>() - *max_dist;
        
        commands.spawn((
            SpriteBundle {
                transform: Transform::from_translation(camera_global_transform.translation + new_pos)
                    .with_rotation(Quat::from_axis_angle(Vec3::Z, PI / 2. * rand::random::<f32>())),
                texture: asset_server.load(texture_path[(rand::random::<f32>() * texture_path.len() as f32) as usize]),
                sprite: Sprite { color: Color::Rgba {alpha: rand::random::<f32>() * 0.25, red: 1., green: 1., blue: 1. }, ..default() },
                ..default()
            },
            Star{depth: rand::random()}
        ));
        }
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