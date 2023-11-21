use std::{net::UdpSocket, time::SystemTime};

use bevy::{prelude::*, DefaultPlugins, utils::HashMap, transform::commands};

use bevy_inspector_egui::{quick::WorldInspectorPlugin, bevy_egui::EguiPlugin};
use bevy_rapier2d::prelude::Velocity;
use bevy_renet::{renet::{*, transport::*}, transport::NetcodeClientPlugin, RenetClientPlugin};
use renet_visualizer::RenetServerVisualizer;

#[path = "client_menu.rs"] mod client_menu;
use client_menu::*;
#[path = "game.rs"] mod game;
use game::*;
use game::components::*;

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
struct InitClient{pub style: u8}

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
            


            (receive_message_system, snap_objects, update_chunks_around, camera_follow).chain(),
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
){
    for e in objects_q.iter(){
        commands.entity(e).despawn();
    }
    for e in debug_chuncs_q.iter(){
        commands.entity(e).despawn();
    }
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
        let mut data: Vec<u8> = vec![];

        // COLOR
        let color = settings.color;
        data.push((color[0] * 255.) as u8);
        data.push((color[1] * 255.) as u8);
        data.push((color[2] * 255.) as u8);

        // STYLE
        data.push(e.style);


        let name = settings.name.as_bytes();
        for char in name.iter(){
            data.push(*char)
        }

        let mut new_data: [u8; 256] = [0; 256];
        for i in 0..data.len(){
            new_data[i] = data[i];
        }

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
                user_data: Some(new_data)
            }, 
            socket
        ).unwrap();

        
        
        let for_spawn_cl_data = ClientData::for_spawn(e.style, color, 0);

        
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
        


        commands.insert_resource(RenetClient::new(connection_config()));
        commands.insert_resource(transport);
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
    let player_data = player_data.get_single_mut();
    if player_data.is_err(){
        return;
    };
    let (mut vel, transform, object) = player_data.unwrap();

    let mut up = false;
    let mut down = false;
    let mut right = false;
    let mut left = false;

    if keys.pressed(KeyCode::W){up = true} //  || buttons.pressed(MouseButton::Right
    if keys.pressed(KeyCode::S){down = true}
    if keys.pressed(KeyCode::A){left = true}
    if keys.pressed(KeyCode::D){right = true}

    
    
    let mut target_angular_vel: f32 = 0.;
    let window = window.single();
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
                    target_angular_vel = -target_angle;
                }
            }
        }
    }
    let pressed_keys = PressedKeys{ up, down, right, left };

    send_message(&mut renet_client, ClientChannel::Fast, Message::Inputs { keys: pressed_keys, rotation_direction: target_angular_vel });

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
    transport: Res<NetcodeClientTransport>,
    mut local_clients_data: ResMut<ClientsData>,
    mut commands: Commands,
    mut objects_q: Query<(Entity, &Object, &mut Velocity, &mut Transform), (With<Object>, Without<Puppet>)>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    mut cached_entities: Local<HashMap<u64, Entity>> // object_id => entity
) {
    for object in objects_q.iter(){
        let (entity, object, _, _) = object;
        let object_id = object.id;
        if !cached_entities.contains_key(&object_id){
            cached_entities.insert(object_id, entity);
        }
    }

    if client.is_disconnected(){
        next_state.set(ClientState::Menu);
    }

    while let Some(message) = client.receive_message(ServerChannel::Fast) {
        let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
        match msg {
            Message::Update { data } => {
                for object_data in data.iter(){
                    //println!("target {:?} my {:?} vel {:?}", object_data.object.id, my_id, object_data.linear_velocity);
                    if cached_entities.contains_key(&object_data.object.id){
                        // UPDATE ENTITY
                        let (_, _, mut velocity, mut transform) = objects_q.get_mut(*cached_entities.get(&object_data.object.id).unwrap()).unwrap();
                        velocity.angvel = object_data.angular_velocity;
                        velocity.linvel = object_data.linear_velocity;
                        transform.translation = object_data.translation;
                        transform.rotation = object_data.rotation;

                    } else {
                        // SPAWN NEW ENTITY
                        match object_data.object.object_type {
                            ObjectType::Asteroid { seed, hp:_ } => {
                                spawn_asteroid(
                                    seed, 
                                    Velocity { linvel: object_data.linear_velocity, angvel: object_data.angular_velocity }, 
                                    Transform::from_translation(object_data.translation).with_rotation(object_data.rotation), 
                                    &mut meshes, 
                                    &mut materials, 
                                    &mut commands,
                                    object_data.object.id,
                                    cfg.get_asteroid_hp(seed),
                                );
                            },
                            ObjectType::Bullet => {

                            },
                            ObjectType::Ship => {
                                let client_op = local_clients_data.get_option_by_object_id(object_data.object.id);
                                if client_op.is_some(){
                                    let client = client_op.unwrap();
                                    let name = &client.name;
                                    let e = spawn_ship(false, &mut meshes, &mut materials, &mut commands, client);
                                    commands.entity(e).insert((
                                        Name::new(format!("Player {}", name)),
                                        Object{
                                            id: object_data.object.id,
                                            object_type: ObjectType::Ship
                                        }
                                        
                                    ));
                                }
                            }
                        }
                    }
                }
            }
            msg_type => {
                warn!("Unhandled message with id {} recived on client!", u8::from(msg_type));
            }
        }
        
    }
    while let Some(message) = client.receive_message(ServerChannel::Garanteed) {
        let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
        match msg {
            Message::OnConnect{clients_data, config, ship_object_id} => {
                *local_clients_data = clients_data;
                *cfg = config;
                let player_data = local_clients_data.get_by_client_id(transport.client_id());
                let entity = spawn_ship(false, &mut meshes, &mut materials, &mut commands, player_data);

                //commands.entity(*cached_entities.get(&0).unwrap()).insert(Object{id: ship_object_id, object_type: ObjectType::Ship});

                commands.entity(entity).insert((
                    CameraFollow,
                    Object{
                        id: ship_object_id,
                        object_type: ObjectType::Ship
                    },
                ));
                for x in -1..(cfg.map_size_chunks.x as i32 + 1){ // include shadow chunks
                    for y in -1..(cfg.map_size_chunks.y as i32 + 1){
                        loaded_chunks.chunks.push(Chunk { pos: Vec2::from((x as f32, y as f32)) });
                    }
                }
            },
            Message::NewConnection { client_data } => {
                local_clients_data.add(client_data)
            }
            msg_type => {
                warn!("Unhandled message with id {} recived on client!", u8::from(msg_type));
            }
        }
    }
}
// println!("{}", String::from_utf8(message.to_vec()).unwrap());
// Send a text message to the server
//client.send_message(DefaultChannel::ReliableOrdered, "HI FROM CLIENT!".as_bytes().to_vec());