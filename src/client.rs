use std::{net::UdpSocket, time::SystemTime, f32::consts::PI};

use bevy::{prelude::*, render::{mesh::Indices, render_resource::PrimitiveTopology}, sprite::{MaterialMesh2dBundle, Mesh2dBindGroup, Mesh2dHandle}, transform, utils::{HashMap, HashSet}, window::WindowResized, DefaultPlugins};

use bevy_inspector_egui::{quick::WorldInspectorPlugin, bevy_egui::EguiPlugin};
use bevy_rapier2d::{na::Translation, plugin::{NoUserData, RapierPhysicsPlugin}, prelude::Velocity, render::{DebugRenderContext, RapierDebugRenderPlugin}};
use bevy_renet::{renet::{*, transport::*}, transport::NetcodeClientPlugin, RenetClientPlugin};
use rand::Rng;
use renet_visualizer::RenetServerVisualizer;

#[path = "client_menu.rs"] mod client_menu;
use client_menu::*;
#[path = "game.rs"] mod game;
use game::*;
use game::components::*;
use serde::de::value;
use weighted_rand::builder::*;

fn main(){
    let mut app = App::new();

    //let default_settings = settings::GameSettings::init();

    //app.insert_resource(default_settings);
    app.init_resource::<GameSettings>();
    app.add_state::<ClientState>();
    // todo: USE RAPIER PHYSICS ON CLIENT???
    app.add_plugins(RenetClientPlugin);
    app.add_plugins(RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0));
    app.add_plugins(RapierDebugRenderPlugin::default());
    app.add_plugins(NetcodeClientPlugin);
    app.insert_resource(RenetServerVisualizer::<200>::default());          
    app.insert_resource(RenetClient::new(ConnectionConfig::default()));
    app.insert_resource(GlobalConfig::default());
    app.insert_resource(ClientsData::default());
    app.insert_resource(LoadedChunks{chunks: vec![]});
    app.add_plugins((DefaultPlugins.set(
        ImagePlugin::default_nearest()
        ).set(WindowPlugin {
            primary_window: Some(Window {
                title: "RUSTEROIDS".into(),
                ..default()
            }),
            ..default()
        }),
        EguiPlugin,
        WorldInspectorPlugin::new()
    ));


    app.add_systems(
        OnEnter(ClientState::Menu), 
        (
            setup_splash,
            setup_preview_camera
    ));
    app.add_systems(
        Update, 
        (
            update_menu,
            update_beams,
            egui_based_menu,
            update_preview_ship,
    ).run_if(in_state(ClientState::Menu)));
    app.add_systems(
        OnExit(ClientState::Menu), 
        (
            despawn_menu,
    ));


    app.add_systems(
        OnEnter(ClientState::InGame), 
        (
            init_client,
    ));
    app.add_systems(
        Update, 
        (
            debug_chunk_render,
            update_powerups_animation,
            (receive_message_system, snap_objects, update_chunks_around, starfield_update, camera_follow, ship_labels).chain(),
            
            handle_inputs_system,
            tab_menu, // todo
            esc_menu
            
    ).run_if(in_state(ClientState::InGame)));
    app.add_systems(
        OnExit(ClientState::InGame), 
        (
            on_ingame_exit,
    ));



    app.insert_resource(ConnectProperties{adress: "".into()});


    game::init_pixel_camera(&mut app);

    app.run()
}

fn on_ingame_exit(
    mut commands: Commands,
    ship_labels_q: Query<Entity, With<ShipLabel>>,
    objects_q: Query<Entity, With<Object>>,
    debug_chuncs_q: Query<Entity, With<Chunk>>,
    mut clients_data: ResMut<ClientsData>,
    mut star_layer_q: Query<Entity, With<StarsLayer>>,
    mut renet_client: ResMut<RenetClient>,
    mut camera_translation: Query<&mut Transform, (With<Camera>, With<PixelCamera>, Without<Object>)>,
){
    // todo: respawn everything
    for e in objects_q.iter(){
        commands.entity(e).despawn();
    }
    for e in debug_chuncs_q.iter(){
        commands.entity(e).despawn();
    }
    for e in ship_labels_q.iter(){
        commands.entity(e).despawn();
    }
    star_layer_q.get_single().is_ok().then(|| { commands.entity(star_layer_q.single()).despawn_recursive();});
    camera_translation.single_mut().translation = Vec3::ZERO;
    renet_client.disconnect();
    //clients_data.clean_exclude_me();
}

fn init_client(
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut commands: Commands,
    mut connect_properties: ResMut<ConnectProperties>,
    mut clients_data: ResMut<ClientsData>,
){  
   
    // COLOR
    //let color = settings.color;
    
    // STYLE
    //e.style;

    //let name = settings.name;

    println!("ADRESS IS {}", connect_properties.adress);
    let server_addr = connect_properties.adress.parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    //UdpSocket::
    const GAME_PROTOCOL_ID: u64 = 0;

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    let transport = NetcodeClientTransport::new(
        current_time, 
        ClientAuthentication::Unsecure {
            protocol_id: GAME_PROTOCOL_ID,
            client_id: current_time.as_millis() as u64,
            server_addr: server_addr,
            user_data: None
        }, 
        socket
    ).unwrap();
    commands.insert_resource(RenetClient::new(connection_config()));
    commands.insert_resource(transport);
    
    
    //let for_spawn_cl_data = ClientData::for_spawn(e.style, color, 0);
    /*let client_data = ClientData {
        client_id: 0,
        object_id: 1,
        style: e.style,
        entity: entity,
        color: color,
        name: "SELF".into()
    };*/
    //clients_data.add(client_data);
    //let player_data = clients_data.get_by_client_id(0);
    
}

fn send_message(
    renet_client: &mut ResMut<RenetClient>,
    chanel: ClientChannel,
    message: Message
){
    let encoded_message: Vec<u8> = bincode::serialize(&message).unwrap();
    renet_client.send_message(chanel, encoded_message);
}

fn handle_inputs_system(
    mut renet_client: ResMut<RenetClient>,
    mut player_data: Query<(&mut Velocity, &Transform, &Object), With<CameraFollow>>,
    keys: Res<Input<KeyCode>>,
    buttons: Res<Input<MouseButton>>,
    window: Query<&mut Window>,
    camera_q: Query<(&Camera, &GlobalTransform), (With<Camera>, Without<PixelCamera>)>,
){
    let mut inp = InputKeys::default();
    /*inp.up = false;
    inp.down = false;
    inp.left = false;
    inp.right = false;
    inp.rotate_left = false;
    inp.rotate_right = false;
    inp.stabilize = false;
    inp.shoot = false;
    inp.dash = false;
    inp.rotation_target = Vec2::ZERO;*/
    let player_data = player_data.get_single_mut();
    if player_data.is_err(){
        return;
    };
    let (mut vel, transform, object) = player_data.unwrap();

    
    if keys.pressed(KeyCode::W){inp.input_vector += Vec2::Y} //  || buttons.pressed(MouseButton::Right
    if keys.pressed(KeyCode::S){inp.input_vector -= Vec2::Y}
    if keys.pressed(KeyCode::A){inp.input_vector += Vec2::X}
    if keys.pressed(KeyCode::D){inp.input_vector -= Vec2::X}
    if keys.pressed(KeyCode::ShiftLeft){inp.dash = true}
    if keys.pressed(KeyCode::Space){inp.shoot = true}
    
    
    if let Ok(t) = camera_q.get_single(){
        let (camera, camera_transform) = t;
        if let Some(world_position) = window.single().cursor_position()
            .and_then(|cursor| camera.viewport_to_world(camera_transform, cursor))
            .map(|ray| ray.origin.truncate())
        {
            inp.rotation_target = world_position;
        }
    }
    
    send_message(&mut renet_client, ClientChannel::Fast, Message::Inputs { inputs: inp });

    //println!("{:?}", (up, down, right, left));
}

#[derive(Component)]
struct CameraFollow;

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
    camera_translation.translation = player_data.unwrap().translation;
}

#[derive(Component)]
struct DeathLabel;

fn receive_message_system(
    mut client: ResMut<RenetClient>,
    mut cfg: ResMut<GlobalConfig>,
    mut next_state: ResMut<NextState<ClientState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    settings: Res<GameSettings>,
    transport: Res<NetcodeClientTransport>,
    mut local_clients_data: ResMut<ClientsData>,
    mut commands: Commands,
    mut objects_q: Query<(Entity, &Object, &mut Velocity, &mut Transform), (With<Object>, Without<Puppet>)>,
    //mut followed_q: Query<(Entity, &Object), With<CameraFollow>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    asset_server: Res<AssetServer>,
    time: Res<Time>,
    mut death_label_q: Query<(Entity, &mut Text), With<DeathLabel>>,
    mut is_dead: Local<(bool, bool, Vec3, f32)>, // first is current, second is previous val, third is pos
){
    if client.is_disconnected(){
        next_state.set(ClientState::Menu);
    }
    let mut existing_objects = HashMap::new();
    for (e, object, _, _) in objects_q.iter(){
        existing_objects.insert(object.id, e);
    }
    let mut entities_to_keep = vec![];
    let mut data_to_update = vec![];
    while let Some(message) = client.receive_message(ServerChannel::Fast) {
        let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
        match msg {
            Message::Update { data } => {
                data_to_update = data;
            }
            msg_type => {
                warn!("Unhandled message recived on client!");
            }
        }
    }

    // UPDATE OBJECTS
    if data_to_update.len() != 0{
        for object_data in data_to_update.iter(){
            if existing_objects.contains_key(&object_data.object.id){
                // UPDATE ENTITY
                let object_r = objects_q.get_mut(*existing_objects.get(&object_data.object.id).unwrap());
                let (e, _, mut velocity, mut transform) = object_r.unwrap();
                velocity.angvel = object_data.angular_velocity;
                velocity.linvel = object_data.linear_velocity;
                transform.translation = object_data.translation;
                transform.rotation = object_data.rotation;
                match object_data.object.object_type{ // update properties
                    ObjectType::Ship{ style: _, color: _, shields: _, hp: _} => {
                        commands.entity(e).insert(object_data.object);
                        let states_and_statuses = object_data.states_and_statuses.clone().unwrap();
                        if object_data.object.id == local_clients_data.get_by_client_id(transport.client_id()).object_id{
                            match states_and_statuses.0 {
                                ShipState::Dead { time } => {
                                    (*is_dead).0 = true;
                                    (*is_dead).2 = object_data.translation;
                                    (*is_dead).3 = time;
                                    commands.entity(e).insert(Visibility::Hidden);
                                }
                                _ => {
                                    (*is_dead).0 = false;
                                    commands.entity(e).insert(Visibility::Visible);
                                }
                            }
                        }
                        commands.entity(e).insert(states_and_statuses);
                    }
                    _ => {}
                }
                entities_to_keep.push(e);
            } else {
                // SPAWN NEW ENTITY
                let t = match object_data.object.object_type {
                    ObjectType::Asteroid { seed, hp:_ } => {
                        let e = spawn_asteroid(
                            seed, 
                            Velocity { linvel: object_data.linear_velocity, angvel: object_data.angular_velocity }, 
                            Transform::from_translation(object_data.translation).with_rotation(object_data.rotation), 
                            &mut meshes, 
                            &mut materials, 
                            &mut commands,
                            object_data.object.id,
                            cfg.get_asteroid_hp(seed),
                        );
                        Some((e, object_data.object.id))
                    },
                    ObjectType::Bullet { previous_position: _, spawn_time, owner, extra_damage } => {
                        let e = spawn_bullet(
                            object_data.linear_velocity,
                            extra_damage,
                            Transform::from_translation(object_data.translation).with_rotation(object_data.rotation), 
                            object_data.object.id, 
                            owner, 
                            spawn_time, 
                            &asset_server, 
                            &mut commands
                        );
                        Some((e, object_data.object.id))
                    },
                    ObjectType::Ship { style, color, shields, hp} => {
                        let client_op = local_clients_data.get_option_by_object_id(object_data.object.id);
                        if client_op.is_some(){
                            let clientdata = client_op.unwrap();
                            let name = &clientdata.name;
                            let e = spawn_ship(false, &mut meshes, &mut materials, &mut commands, clientdata, &mut cfg, &time);
                            //println!("SPAWNED SHIP FOR {} WITH ID {} -> E {:?}", client.client_id, client.object_id, e);
                            commands.entity(e).insert((
                                Name::new(format!("Player {}", name)),
                                Object{
                                    id: object_data.object.id,
                                    object_type: ObjectType::Ship{ style, color, shields, hp }
                                },
                                Transform::from_translation(object_data.translation).with_rotation(object_data.rotation)
                            ));
                            /*if death_time == 0. {
                                commands.entity(e).insert(Visibility::Inherited);
                            } else {
                                commands.entity(e).insert(Visibility::Hidden);
                            }*/
                            if object_data.object.id == local_clients_data.get_by_client_id(transport.client_id()).object_id{
                                commands.entity(e).insert(CameraFollow);
                            }
                            Some((e, object_data.object.id))
                        } else {
                            None
                        }
                    },
                    ObjectType::PickUP{pickup_type} => {
                        let e = spawn_powerup(pickup_type, object_data.translation, &mut commands, &mut meshes, &mut materials, &asset_server, object_data.object.id);
                        Some((e, object_data.object.id))
                    }
                };
                if t.is_some(){
                    let (e, id) = t.unwrap();
                    existing_objects.insert(id, e);
                    entities_to_keep.push(e);
                }
            }
        }
        for (_, e) in existing_objects.iter(){
            if !entities_to_keep.contains(e){ 
                commands.entity(*e).despawn_recursive();
            }
        }
    }

    if (*is_dead).0 != (*is_dead).1 { // spawn or despawn
        if (*is_dead).0 {
            let font = asset_server.load("../assets/fonts/VecTerminus12Medium.otf");
            let text_style = TextStyle {
                font: font.clone(),
                font_size: 26.0,
                color: Color::ORANGE_RED,
            };
            commands.spawn((
                Text2dBundle{
                    text: Text::from_sections([
                        TextSection{value: "U ARE DEAD! :D".into(), style: text_style.clone()},
                        TextSection{value: format!("\n{}", (cfg.respawn_time_secs - is_dead.3).round() as i32).into(), style: text_style}
                    ]).with_alignment(TextAlignment::Center),
                    transform: Transform::from_translation(is_dead.2.truncate().extend(100.)),
                    ..default()
                },
                DeathLabel
            ));
        } else {
                commands.entity(death_label_q.single().0).despawn();
        }
    } else {
        if (*is_dead).0 {
            let (_, mut text) = death_label_q.single_mut();
            text.sections.get_mut(1).unwrap().value = format!("\n{}", (cfg.respawn_time_secs - is_dead.3).round() as i32);
        }
    }
    (*is_dead).1 = (*is_dead).0;

    while let Some(message) = client.receive_message(ServerChannel::Garanteed) {
        let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
        match msg {
            Message::OnConnect{clients_data, config, ship_object_id} => {
                *local_clients_data = clients_data;
                *cfg = config;
                for x in -1..(cfg.map_size_chunks.x as i32 + 1){ // include shadow chunks
                    for y in -1..(cfg.map_size_chunks.y as i32 + 1){
                        loaded_chunks.chunks.push(Chunk { pos: Vec2::from((x as f32, y as f32)) });
                    }
                }
            },
            Message::NewConnection { client_data } => {
                local_clients_data.add(client_data)
            }
            Message::NewDisconnection { id } => {
                local_clients_data.remove_by_client_id(id)
            }
            Message::Greeteng {  } => {
                send_message(
                    &mut client, 
                    ClientChannel::Garanteed, 
                    Message::RegisterClient {
                        style: settings.style,
                        color: Color::from(settings.color),
                        name: settings.name.clone() as String
                    }
                );
            },
            _msg_type => {
                warn!("Unhandled message recived on client!");
            }
        }
    }
}
// println!("{}", String::from_utf8(message.to_vec()).unwrap());
// Send a text message to the server
//client.send_message(DefaultChannel::ReliableOrdered, "HI FROM CLIENT!".as_bytes().to_vec());


const STARFIELD_STARS : usize = 5000;
const DUST : usize = 1000;

fn distance_distribution(x: f32) -> f32{
    if x < 0.5{
        0.1 + x / 5.
    } else{
        (1. - (1. - x.powi(10)).powi(2)) * 0.9 + 0.2
    } 
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


// todo: add shooting stars
// todo: add many values to settings!
// todo: add handle of screen resize
fn _starfield_update(
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
    let p = player.get_single();
    let (player_transform, player_velocity) = if p.is_err(){return;} else {p.unwrap()};
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
    }
    if keys.just_pressed(KeyCode::P){
        commands.entity(star_layer_q.single().1).despawn_recursive();
    }
}


// todo: this is dust..
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
    mut max_dist_squared: Local<f32>,
    mut prev_pos: Local<Vec2>,
    cfg: Res<GlobalConfig>
){
    if camera.is_empty(){return;}
    let (camera, camera_global_transform) = camera.single_mut();
    let camera_global_transform = camera_global_transform.compute_transform();
    let padding = 10.;
    let mut reader = resize_event.get_reader();
    if reader.read(&resize_event).len() > 0 || *max_dist < 1. || keys.just_pressed(KeyCode::P){ // todo: fix bug with first frame; after using it in client it might fix itself.
        let window_size = camera.ndc_to_world(
            &GlobalTransform::from(camera_global_transform.with_rotation(Quat::from_axis_angle(Vec3::Z, 0.)).with_translation(Vec3::ZERO)),
            Vec3::ONE
        ).unwrap();
        let max_size = window_size.x.round().max(window_size.y.round());
        *max_dist_squared = 2. * (max_size.powi(2));
        *max_dist = max_dist_squared.sqrt();
    }
    let p = player.get_single();
    let (player_transform, player_velocity) = if p.is_err(){return;} else {p.unwrap()};
    if star_layer_q.get_single().is_ok(){
        star_layer_q.single_mut().0.translation = player_transform.translation;
    }
    if *prev_pos != Vec2::ZERO{
        let map_size = cfg.single_chunk_size * cfg.map_size_chunks;
        let delta1 = *prev_pos - player_transform.translation.truncate();
        let mut delta2 = (*prev_pos + map_size + map_size / 2.) % map_size - (player_transform.translation.truncate() + map_size + map_size / 2.) % map_size;
        let delta = if delta1.length_squared() < delta2.length_squared(){delta1}else{delta2}; // todo: REWRITE!!!! 
        
        for star_data in star_q.iter_mut(){
            let (mut transform, mut sprite, star, e) = star_data;
            let star_transform =  transform.translation;
            if star_transform.truncate().length_squared() < *max_dist_squared + padding{ // inside "keep" circle
                transform.translation += delta.extend(0.) * time.delta_seconds() * (star.depth + 10.) * 6.5;//
            } else {
                if rand::random::<f32>() < 0.1 { // some random
                    commands.entity(e).remove_parent();
                    commands.entity(e).despawn();
                }
            }
        }
    }
    *prev_pos = player_transform.translation.truncate();
    // todo: add stars back
    let curr_stars_count = star_q.into_iter().len();
    let mut rng = rand::thread_rng();
    let texture_path = [
        "dust.png" // todo: upper dust layer might move faster than player!
    ];
    


    if curr_stars_count < DUST{ // todo: move to init and add varables to settings
        let layer = star_layer_q.get_single();
        let layer = if layer.is_ok(){
            layer.unwrap().1
        } else {
            commands.spawn((
                StarsLayer,
                TransformBundle::default(),
                VisibilityBundle::default(),
                Name::new("Dust Layer")
            )).id()
        };
        
        let diff = DUST - curr_stars_count;
        let init_spawn = curr_stars_count == 0;
        for _ in 0..diff{
            let depth_range = (-3., 3.);
            let depth = rand::random::<f32>() * (depth_range.1 - depth_range.0) - depth_range.1;
            
            let size = 0.35 + (depth + depth_range.1) * 0.1;// 0.3;

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
            new_pos.z = depth;
            commands.spawn((
                SpriteBundle {
                    transform: Transform::from_translation(new_pos)
                        .with_rotation(Quat::from_axis_angle(Vec3::Z, PI / 2. * rand::random::<f32>()))
                        .with_scale(Vec3::splat(size)),//0.11 + depth * size * 0.8
                        // 0.2 -> 0.65
                    texture: asset_server.load(texture_path[rng.gen_range(0..texture_path.len())]),
                    sprite: Sprite { color: Color::from([0.05, 0.05, 0.05, 1.]), ..default() }, // ADD RANDOM COLORS
                    ..default()
                },
                Star{depth: depth},
                Name::new("Dust")
            )).set_parent(layer);
        }
    }
    if keys.just_pressed(KeyCode::P){
        commands.entity(star_layer_q.single().1).despawn_recursive();
    }
}







#[derive(Component)]
struct StatusBar{binds: HashMap<PowerUPType, Entity>}

impl Default for StatusBar {
    fn default() -> Self {
        StatusBar{binds: HashMap::new()}
    }
}

#[derive(Component)]
struct ShipLabel{entity_id: u64}

#[derive(Component)]
struct ShieldBar;

#[derive(Component)]
struct HPBar;

#[derive(Clone)]
struct CachedMeshes{
    hp: Mesh,
    shield: Mesh,
    outline: Mesh,
    bg: Mesh
}

#[derive(Clone)]
struct ShipData{
    name: String,
    shields: f32,
    hp: f32,
    statuses: ShipStatuses
}

fn ship_labels( // todo: maybe add it as childs to ships? // add handle for every puppet
    cfg: Res<GlobalConfig>,
    mut commands: Commands,
    ships_q: Query<(&Object, &mut Transform, &ShipState, &ShipStatuses, Entity), (With<Ship>, Without<ShipLabel>, Without<Puppet>, Without<HPBar>, Without<ShieldBar>)>,
    ships_puppets_q: Query<(&Object, &mut Transform, &ShipState, &ShipStatuses, Entity), (With<Ship>, Without<ShipLabel>, With<Puppet>, Without<HPBar>, Without<ShieldBar>)>,
    mut labels_q: Query<(&ShipLabel, &mut Transform, Entity, &Children), (With<ShipLabel>, Without<Ship>, Without<HPBar>, Without<ShieldBar>)>,  
    mut hpbar_q: Query<&mut Transform, (With<HPBar>, Without<ShipLabel>, Without<Ship>, Without<ShieldBar>)>,  
    mut shieldbar_q: Query<&mut Transform, (With<ShieldBar>, Without<ShipLabel>, Without<Ship>, Without<HPBar>)>,
    mut statusbar_q: Query<(Entity, &mut StatusBar, Option<&Children>), With<StatusBar>>,
    mut status_q: Query<(Entity, &PowerUPType, Option<&Children>)>,
    mut text_q: Query<&mut Text>,
    asset_server: Res<AssetServer>, 
    clients_data: Res<ClientsData>,
    time: Res<Time>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut cached_meshes: Local<Option<CachedMeshes>>
){
    let font = asset_server.load("../assets/fonts/VecTerminus12Medium.otf");
    let text_style = TextStyle {
        font: font.clone(),
        font_size: 14.0,
        color: Color::WHITE,
    };

    let size = Vec2{x: 50., y: 4.};
    let line_indices = vec![0, 1, 1, 2, 2, 3, 3, 0];
    let indices = vec![0, 1, 2, 1, 2, 3];
    let outline_vertices = vec![
        Vec3{x: (size.x / 2. + 1.), y: (size.y / 2. + 1.), z:10.},
        Vec3{x: (size.x / 2. + 1.), y:-(size.y / 2. + 1.), z:10.},
        Vec3{x:-(size.x / 2. + 1.), y:-(size.y / 2. + 1.), z:10.},
        Vec3{x:-(size.x / 2. + 1.), y: (size.y / 2. + 1.), z:10.}
    ];
    if cached_meshes.is_none(){
        let mut outline_mesh = Mesh::new(PrimitiveTopology::LineList);
        outline_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, outline_vertices.clone()); 
        outline_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::DARK_GRAY.as_rgba_f32(); 4]); 
        outline_mesh.set_indices(Some(Indices::U32(line_indices)));
        let mut outline_mesh_bg = Mesh::new(PrimitiveTopology::TriangleList);
        outline_mesh_bg.insert_attribute(Mesh::ATTRIBUTE_POSITION, outline_vertices); 
        outline_mesh_bg.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::BLACK.as_rgba_f32(); 4]); 
        outline_mesh_bg.set_indices(Some(Indices::U32(indices.clone())));

        let mesh_vertices = vec![
            Vec3{x: size.x / 2.,y: size.y / 2., z:10.},
            Vec3{x: size.x / 2.,y:-size.y / 2., z:10.},
            Vec3{x:-size.x / 2.,y: size.y / 2., z:10.},
            Vec3{x:-size.x / 2.,y:-size.y / 2., z:10.}
        ];
        let mut hp_mesh = Mesh::new(PrimitiveTopology::TriangleList);
        hp_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, mesh_vertices);
        hp_mesh.set_indices(Some(Indices::U32(indices)));
        hp_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::YELLOW_GREEN.as_rgba_f32(); 4]); 
        let mut shield_mesh = hp_mesh.clone();
        shield_mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::CYAN.as_rgba_f32(); 4]); 
        *cached_meshes = Some(CachedMeshes{
            hp: hp_mesh,
            shield: shield_mesh,
            outline: outline_mesh,
            bg: outline_mesh_bg
        });
    }
    let mut labels = HashMap::new();
    for (label, transform, entity, children) in labels_q.iter_mut(){
        labels.insert(label.entity_id, (label, transform, entity, children));
    }
    let mut used_labels = HashSet::new();
    // iterating trough ships, collect data about name, hp and shields; and after iter trough puppets. because puppets doesnt update their hp and shields, but we need to update its labels
    let mut data_about_ships: HashMap<u64, ShipData> = HashMap::new();
    for (object, transform, state, statuses, e) in ships_q.iter().chain(ships_puppets_q.iter()){ 
        match state {ShipState::Dead { time: _ } => {continue;} _ => {}}; // labels only for alive!

        let id = e.to_bits();
        let data = clients_data.get_option_by_object_id(object.id);
        if data.is_some(){
            let data = data.unwrap();
            match object.object_type {
                ObjectType::Ship { style:_ , color: _, shields, hp} => {
                    let ship_data = if ships_puppets_q.contains(e){
                        let res = data_about_ships.get(&object.id);
                        if res.is_none(){
                            continue;
                        }
                        res.unwrap().clone()
                    } else {
                        data_about_ships.insert(object.id, 
                            ShipData{
                                name: data.name.clone(),
                                shields,
                                hp,
                                statuses: statuses.clone()
                            });
                        ShipData{
                            name: data.name.clone(),
                            shields,
                            hp,
                            statuses: statuses.clone()
                        }
                    };
                    if labels.contains_key(&id){
                        labels.get_mut(&id).unwrap().1.translation = transform.translation + Vec3::Y * -28. + Vec3::Z * 10.;
                        /* UPDATE */
                        let children = labels.get(&id).unwrap().3;
                        
                        
                        for e in children.iter(){
                            commands.entity(*e);
                            let hpbar = hpbar_q.get_mut(*e);
                            if hpbar.is_ok(){
                                hpbar.unwrap().scale.x = ship_data.hp / cfg.player_hp;
                            }


                            let shbar = shieldbar_q.get_mut(*e);
                            if shbar.is_ok(){
                                shbar.unwrap().scale.x = ship_data.shields / cfg.player_shields;
                            }

                            //let empty_statusbar = empty_statusbar_q.get_mut(*e);
                            let statusbar = statusbar_q.get_mut(*e);


                            if statusbar.is_ok(){
                                
                                let (e, mut status_bar, children) = statusbar.unwrap();

                                let mut iterated = HashSet::new();
                                let mut existing = HashSet::new();
                                //let mut to_spawn = HashSet::new();
                                //let mut to_rem = HashSet::new();
                                if children.is_some() {
                                    for c in children.unwrap().iter(){
                                        let status = status_q.get(*c);
                                        if status.is_ok(){
                                            let (new_e, existing_status, children_op) = status.unwrap();
                                            status_bar.binds.insert(*existing_status, new_e);
                                            existing.insert(existing_status);
                                        }
                                    }
                                }
                                //println!("ex {}", existing.len());
                                
                                let scale = 1.2;
                                for (status_type, effect) in ship_data.statuses.current.iter(){
                                    iterated.insert(status_type);
                                    if !existing.contains(status_type){
                                        // spawn
                                        let new_e = commands.spawn((
                                            SpriteBundle {
                                                texture: asset_server.load(status_type.texture_path()),
                                                transform: Transform::from_scale(Vec3::splat(scale)),
                                                ..default()
                                            },
                                            status_type.clone(),
                                            Name::new("ICON"),
                                        )).set_parent(e).id();

                                        status_bar.binds.insert(*status_type, new_e);

                                        commands.spawn(Text2dBundle{
                                            text: Text::from_section(
                                            format!("{}", effect.get_val_to_show(status_type)),
                                            TextStyle {
                                                font: font.clone(),
                                                font_size: 10.0,
                                                color: Color::WHITE,
                                            }),
                                            transform: Transform::from_translation(Vec3::Y * -10. - Vec3::X),
                                            ..default()
                                        }).set_parent(new_e);
                                    }
                                }

                                let max_len = 
                                    (format!("{}", cfg.effects_extradamage_secs).len()).max(
                                    (format!("{}", cfg.effects_haste_secs).len()).max(
                                    (format!("{}", cfg.effects_invisibility_secs).len()).max(
                                     format!("{}" , cfg.effects_supershield_amount).len()))) as f32;
                                
                                let margin = 2. * max_len + 2.; // 2 * len + 2
                                let single_size = 12.;
                                let half_size = (margin + single_size) / 2. * (iterated.len() as f32 - 1.);
                                let mut i = 0;
                                for (status_type, effect) in ship_data.statuses.current.iter(){ // update pos and value
                                    let val = effect.get_val_to_show(status_type);
                                    let icon_entity = status_bar.binds.get(status_type).unwrap();
                                    commands.entity(*icon_entity).insert(Transform::from_translation(Vec3::X * (i as f32 * (single_size + margin) - half_size + 0.5)).with_scale(Vec3::splat(scale)));


                                    let option = status_q.get(*icon_entity);
                                    
                                    
                                    if option.is_ok() {
                                        let (_, _, children) = option.unwrap();
                                        if children.is_some() {
                                            text_q.get_mut(*children.unwrap().first().unwrap()).unwrap().sections.get_mut(0).unwrap().value = format!("{}", val.round());
                                        }
                                    }
                                    i += 1;
                                }
                                
                                for status in existing {
                                    if !iterated.contains(status){
                                        let e = status_bar.binds.get(status);
                                        if e.is_some() {
                                            commands.entity(*e.unwrap()).despawn_recursive();
                                        } else {
                                            warn!("Statusbar doesnt have bind to \"need_to_despawn\" entity")
                                        }
                                    }
                                }


                                    //cfg.get_power_up_effect(status_type);
                                
                            };
                        }
                        
                    } else {
                        let label_e = commands.spawn((
                            Text2dBundle {
                                text: Text::from_section(
                                        format!("{}\n", ship_data.name),
                                        text_style.clone(),
                                    ),
                                ..default()
                            },
                            ShipLabel{
                                entity_id: id
                            },
                        )).insert(Transform::from_translation(transform.translation + Vec3::Y * -28. + Vec3::Z * 10.)).id();
                        let cached_meshes = cached_meshes.clone().unwrap();

                        commands.spawn(MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(meshes.add(cached_meshes.hp.clone())),
                            transform: Transform::from_translation(Vec3::Y * -10. + Vec3::Z),
                            material: materials.add(ColorMaterial::default()),
                            ..default()
                        }).insert(HPBar).set_parent(label_e);
                        commands.spawn(MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(meshes.add(cached_meshes.outline.clone())),
                            transform: Transform::from_translation(Vec3::Y * -10.),
                            material: materials.add(ColorMaterial::default()),
                            ..default()
                        }).set_parent(label_e);
                        commands.spawn(MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(meshes.add(cached_meshes.bg.clone())),
                            transform: Transform::from_translation(Vec3::Y * -10.),
                            material: materials.add(ColorMaterial::default()),
                            ..default()
                        }).set_parent(label_e);

                        commands.spawn(MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(meshes.add(cached_meshes.shield.clone())),
                            transform: Transform::from_translation(Vec3::Y * -16. + Vec3::Z),
                            material: materials.add(ColorMaterial::default()),
                            ..default()
                        }).insert(ShieldBar).set_parent(label_e);
                        commands.spawn(MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(meshes.add(cached_meshes.outline.clone())),
                            transform: Transform::from_translation(Vec3::Y * -16.),
                            material: materials.add(ColorMaterial::default()),
                            ..default()
                        }).set_parent(label_e);
                        commands.spawn(MaterialMesh2dBundle {
                            mesh: Mesh2dHandle(meshes.add(cached_meshes.bg.clone())),
                            transform: Transform::from_translation(Vec3::Y * -16.),
                            material: materials.add(ColorMaterial::default()),
                            ..default()
                        }).set_parent(label_e);

                        commands.spawn((
                            StatusBar::default(),
                            TransformBundle::default(),
                            VisibilityBundle::default(),
                            Name::new("ICONBAR"),
                        )).insert(Transform::from_translation(Vec3::Y * 74.)).set_parent(label_e);
                    }
                    used_labels.insert(id);
                },
                _ => {}
            }
        }
    }
    
    for (id, (_, _, e, _)) in labels.iter(){
        if !used_labels.contains(id){commands.entity(*e).despawn_recursive()};
    }
}