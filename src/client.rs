use std::{net::UdpSocket, time::SystemTime, f32::consts::PI};

use bevy::{prelude::*, DefaultPlugins, utils::HashMap, transform::commands, window::WindowResized};

use bevy_inspector_egui::{quick::WorldInspectorPlugin, bevy_egui::EguiPlugin};
use bevy_rapier2d::prelude::Velocity;
use bevy_renet::{renet::{*, transport::*}, transport::NetcodeClientPlugin, RenetClientPlugin};
use rand::Rng;
use renet_visualizer::RenetServerVisualizer;

#[path = "client_menu.rs"] mod client_menu;
use client_menu::*;
#[path = "game.rs"] mod game;
use game::*;
use game::components::*;
use weighted_rand::builder::*;

//InintClient


#[derive(Resource)]
struct ConnectProperties{
    pub adress: String
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
enum ClientState {
    #[default]
    Menu,
    InGame
}

#[derive(Event)]
struct InitClient;

fn main(){
    let mut app = App::new();

    //let default_settings = settings::GameSettings::init();

    //app.insert_resource(default_settings);
    app.init_resource::<GameSettings>();
    app.add_state::<ClientState>();
    // todo: USE RAPIER PHYSICS ON CLIENT???
    app.add_plugins(RenetClientPlugin);
    app.add_plugins(NetcodeClientPlugin);
    app.insert_resource(RenetServerVisualizer::<200>::default());
    let client = RenetClient::new(ConnectionConfig::default());
    app.insert_resource(client);
    app.insert_resource(GlobalConfig::default());
    app.insert_resource(ClientsData::default());
    app.insert_resource(LoadedChunks{chunks: vec![]});
    app.add_plugins((DefaultPlugins.set(
        ImagePlugin::default_nearest()
        ),
        EguiPlugin,
        WorldInspectorPlugin::new()
    ));


    app.add_systems(
        OnEnter(ClientState::Menu), 
        (
            setup_splash_and_beams,
            setup_preview_camera
    ));
    app.add_systems(
        Update, 
        (
            update_menu,
            spawn_beam,
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
            


            (receive_message_system, snap_objects, update_chunks_around, starfield_update, camera_follow).chain(),
            handle_inputs_system,
            
    ).run_if(in_state(ClientState::InGame)));
    app.add_systems(
        OnExit(ClientState::InGame), 
        (
            despawn_game_components,
    ));



    app.insert_resource(ConnectProperties{adress: "".into()});

    app.add_event::<SpawnMenuBeam>();
    app.add_event::<InitClient>();


    game::init_pixel_camera(&mut app);

    app.run()
}

fn despawn_game_components(
    mut commands: Commands,
    objects_q: Query<Entity, With<Object>>,
    debug_chuncs_q: Query<Entity, With<Chunk>>,
    mut clients_data: ResMut<ClientsData>,
    mut star_layer_q: Query<Entity, With<StarsLayer>>,
){
    for e in objects_q.iter(){
        commands.entity(e).despawn();
    }
    for e in debug_chuncs_q.iter(){
        commands.entity(e).despawn();
    }
    commands.entity(star_layer_q.single()).despawn_recursive();
    clients_data.clean_exclude_me();
}

fn init_client(
    mut reader: EventReader<InitClient>,
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut commands: Commands,
    mut connect_properties: ResMut<ConnectProperties>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    // todo: FOR DEBUG => REMOVE IT
    mut clients_data: ResMut<ClientsData>,
){  
    for e in reader.read(){
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
    let player_data = player_data.get_single_mut();
    if player_data.is_err(){
        return;
    };
    let (mut vel, transform, object) = player_data.unwrap();


    if keys.pressed(KeyCode::W){inp.up = true} //  || buttons.pressed(MouseButton::Right
    if keys.pressed(KeyCode::S){inp.down = true}
    if keys.pressed(KeyCode::A){inp.left = true}
    if keys.pressed(KeyCode::D){inp.right = true}

    
    
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
    mut followed_q: Query<(Entity, &Object), With<CameraFollow>>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    mut cached_entities: Local<HashMap<u64, Entity>>, // object_id => entity
){
    if client.is_disconnected(){
        next_state.set(ClientState::Menu);
    }

    /*let self_data = local_clients_data.get_option_by_client_id(transport.client_id());
    
    let follow_object /* self */ = followed_q.get_single();
    if follow_object.is_err() && self_data.is_some(){
        for object in objects_q.iter(){
            let (entity, object, _, _) = object;
            if object.id == self_data.unwrap().object_id{
                cached_entities.insert(object.id, entity);
                commands.entity(entity).insert(CameraFollow);
            }
        }
    }*/
    /*for object in objects_q.iter(){
        let (entity, object, _, _) = object;
        let object_id = object.id;
        if !cached_entities.contains_key(&object_id){
            cached_entities.insert(object_id, entity);
        }
    }*/
    //println!("{:?}", cached_entities);
    while let Some(message) = client.receive_message(ServerChannel::Fast) {
        let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
        match msg {
            Message::Update { data } => {
                let mut iterated_entities: Vec<Entity> = vec![];
                for object_data in data.iter(){
                    //println!("target {:?} my {:?} vel {:?}", object_data.object.id, my_id, object_data.linear_velocity);
                    if cached_entities.contains_key(&object_data.object.id){
                        // UPDATE ENTITY
                        let object_r = objects_q.get_mut(*cached_entities.get(&object_data.object.id).unwrap());
                        if object_r.is_ok(){
                            let (e, _, mut velocity, mut transform) = object_r.unwrap();
                            velocity.angvel = object_data.angular_velocity;
                            velocity.linvel = object_data.linear_velocity;
                            transform.translation = object_data.translation;
                            transform.rotation = object_data.rotation;
                            iterated_entities.push(e);
                        } else {
                            //cached_entities.remove(&object_data.object.id);
                            println!("{} need to be removed!", &object_data.object.id)
                        }
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
                            ObjectType::Bullet => {
                                None
                            },
                            ObjectType::Ship => {
                                let client_op = local_clients_data.get_option_by_object_id(object_data.object.id);
                                if client_op.is_some(){
                                    let client = client_op.unwrap();
                                    let name = &client.name;
                                    let e = spawn_ship(false, &mut meshes, &mut materials, &mut commands, client);
                                    println!("SPAWNED SHIP FOR {} WITH ID {} HIS E IS {:?}", client.client_id, client.object_id, e);
                                    commands.entity(e).insert((
                                        Name::new(format!("Player {}", name)),
                                        Object{
                                            id: object_data.object.id,
                                            object_type: ObjectType::Ship
                                        }
                                    ));
                                    Some((e, object_data.object.id))
                                } else {
                                    None
                                }
                            }
                        };
                        if t.is_some(){
                            let (e, id) = t.unwrap();
                            cached_entities.insert(id, e);
                            iterated_entities.push(e);
                            println!("ADDED BIND FOR WITH ID {} -> E {:?}", id, e);
                        }
                    }
                }
                let mut to_remove = vec![];
                let ce = cached_entities.clone();
                for (k, e) in ce.iter(){
                    if !iterated_entities.contains(e){
                        commands.entity(*e).despawn_recursive();
                        println!("DESPAWNED ID {} -> E {:?}", k, e);
                        to_remove.push(k);
                    }
                }
                for k in to_remove.iter(){
                    cached_entities.remove(*k);
                }
            }
            msg_type => {
                warn!("Unhandled message recived on client!");
            }
        }
        
    }
    while let Some(message) = client.receive_message(ServerChannel::Garanteed) {
        let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
        match msg {
            Message::OnConnect{clients_data, config, ship_object_id} => {
                *local_clients_data = clients_data;
                *cfg = config;
                println!("spawned new ship!");
                let player_data = local_clients_data.get_by_client_id(transport.client_id());
            
                let entity = spawn_ship(false, &mut meshes, &mut materials, &mut commands, player_data);
                println!("<init> SPAWNED SHIP FOR {} WITH ID {}", player_data.client_id, player_data.object_id);
                //commands.entity(*cached_entities.get(&0).unwrap()).insert(Object{id: ship_object_id, object_type: ObjectType::Ship});
                commands.entity(entity).insert((
                    CameraFollow,
                    Object{
                        id: ship_object_id,
                        object_type: ObjectType::Ship
                    },
                ));
                
                cached_entities.insert(ship_object_id, entity);

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
// todo: add some values to settings!
// todo: add handle of screen resize
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