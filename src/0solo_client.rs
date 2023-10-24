use bevy::{
    //diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
    window::{WindowResizeConstraints, WindowResolution, WindowTheme}, sprite::{MaterialMesh2dBundle, Mesh2dHandle}, render::{render_resource::PrimitiveTopology, mesh::Indices}, core_pipeline::{tonemapping::Tonemapping, bloom::{BloomSettings, BloomCompositeMode}}, diagnostic::FrameTimeDiagnosticsPlugin,
};

use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::*;
use rand::prelude::*;

const INITIAL_WIDTH: u32 = 100;
const INITIAL_HEIGHT: u32 = 100;
const SCALE_FACTOR: f32 = 6.0;

#[path = "game.rs"] mod new_game;
#[path = "0old_game.rs"] mod game;
use game::*;

pub fn main() {
    App::new()
    .insert_resource(ClearColor(Color::BLACK))
        .add_plugins((
            DefaultPlugins
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                primary_window: Some(Window {
                    window_theme: Some(WindowTheme::Dark),
                    title: "Rusteroids".to_string(),
                    resolution: WindowResolution::new(
                        INITIAL_WIDTH as f32 * SCALE_FACTOR,
                        INITIAL_HEIGHT as f32 * SCALE_FACTOR,
                    ),
                    resize_constraints: WindowResizeConstraints {
                        min_width: INITIAL_WIDTH as f32 * SCALE_FACTOR,
                        min_height: INITIAL_HEIGHT as f32 * SCALE_FACTOR,
                        ..default()
                    },
                    //fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            //LogDiagnosticsPlugin::default(),
        ))
        .add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0)) // ::<NoUserData>::pixels_per_meter(15.0)
        .add_plugins(RapierDebugRenderPlugin{enabled: false, ..default()})
        .add_plugins(WorldInspectorPlugin::new())
        //.configure_sets(Startup,  (MySet::First, MySet::Second).chain())/
        .add_systems(Startup, (
            //basic_setup,

            game::setup_pixel_camera,

            game::init,
            game::spawn_debug,
            game::spawn_ship.after(game::init),

        ))//spawn_testroom
        .add_systems(Update, (

            game::update_pixel_camera,

            game::update_debug,
            game::spawn_bullet,
            game::check_bullet_collisions_and_lifetime,
            (
                game::snap_objects,
                game::player_movement,
                game::update_chunks_around,
                game::spawn_asteroid,
                
            ).chain(),
        )) // game::modify_collider_active_events
        .add_event::<game::GetChunk>()
        .add_event::<game::SpawnBullet>()
        .add_event::<game::SpawnAsteroid>()
        
        .insert_resource(game::MapSettings{
            last_id: 0,
            max_size: Vec2{x: 10., y: 1.},
            single_chunk_size: Vec2{x: 500., y: 500.},
            debug_render: true,
        })

        // [TEST]
        //.add_systems(Update, asteroid_cycle)
        // [TEST]

        //.add_systems(Update,(bevy::window::close_on_esc, (bounce, movement).chain()),)
        //.add_systems(Draw, (draw_background, draw_objects).chain())
        .run();
}

fn _spawn_testroom(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut spawn_asteroid_event: EventWriter<SpawnAsteroid>,
){
        
    commands.spawn((
        Collider::ball(10.),
        Restitution {
            coefficient: 1.,
            combine_rule: CoefficientCombineRule::Multiply,
        },
        GravityScale(0.0),
        RigidBody::Dynamic,
        ColliderMassProperties::Density(0.1),
        Name::from("BALL!?!?!?")
    )).insert(TransformBundle::from(Transform::from_xyz(0., 300., 0.0)));
    

    //WALLS
    let vec = vec![
        [-790., -390., 0.],
        [790., -390., 0.],
        [790., 390., 0.],
        [-790., 390., 0.]
    ];
    let ind = vec![0, 1, 1, 2, 2, 3, 3, 0];
    let mut mesh = Mesh::new(PrimitiveTopology::LineList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec);
    mesh.set_indices(Some(Indices::U32(ind)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::ORANGE_RED.as_rgba_f32(); 4]);
    
    commands.spawn(MaterialMesh2dBundle { //MESH
        mesh: Mesh2dHandle(meshes.add(mesh)),
        transform: Transform::from_scale(Vec3::splat(1.)),
        material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
        ..default()
    });
    
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(800., 10.),
        Restitution {
            coefficient: 1.,
            combine_rule: CoefficientCombineRule::Multiply,
        },
        Name::from("Bottom"),
        Transform::from_xyz(0., 1000., 0.0),
    )).insert(TransformBundle::from(Transform::from_xyz(0.0, -400.0, 0.0)));
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(800., 10.),
        Restitution {
            coefficient: 1.,
            combine_rule: CoefficientCombineRule::Multiply,
        },
        Name::from("Up"),
    )).insert(TransformBundle::from(Transform::from_xyz(0.0, 400.0, 0.0)));
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(10., 400.),
        Restitution {
            coefficient: 1.,
            combine_rule: CoefficientCombineRule::Multiply,
        },
        Name::from("Right"),
    )).insert(TransformBundle::from(Transform::from_xyz(-800.0, 0.0, 0.0)));
    commands.spawn((
        RigidBody::Fixed,
        Collider::cuboid(10., 400.),
        Restitution {
            coefficient: 1.,
            combine_rule: CoefficientCombineRule::Multiply,
        },
        Name::from("Left"),
    )).insert(TransformBundle::from(Transform::from_xyz(800.0, 0.0, 0.0)));
    //////
    

    
    for _ in 0..100{
        let x = -600.0 + random::<f32>() * 1200.0;
        let y = -350.0 + random::<f32>() * 700.0;
        let seed = rand::random::<u64>();
        spawn_asteroid_event.send(game::SpawnAsteroid{transform: Transform::from_xyz(x, y, 0.), velocity:Velocity::zero(), seed: seed});
    }
}

fn _basic_setup(
    mut commands: Commands,
) {
    commands.spawn((
        Camera2dBundle {
            camera: Camera {
                hdr: true, // 1. HDR is required for bloom
                
                ..default()
            },
            
            //camera_2d: Camera2d { clear_color: ClearColorConfig::Custom(Color::Rgba { red: 0., green: 0., blue: 0., alpha: 0.3}) },
            tonemapping: Tonemapping::TonyMcMapface, // 2. Using a tonemapper that desaturates to white is recommended
            projection: OrthographicProjection{
                scaling_mode: bevy::render::camera::ScalingMode::FixedVertical(128.),
                ..default()
            },
            ..default()
        },
        BloomSettings{ // 3. Enable bloom for the camera
            composite_mode: BloomCompositeMode::Additive,
            intensity: 0.1,
            ..default()
        }, 
    )).insert(Transform::from_scale(Vec3::splat(1.)));//Vec3::splat(1.)
}




