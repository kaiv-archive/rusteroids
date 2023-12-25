use std::{net::{UdpSocket, SocketAddr}, time::SystemTime, f32::consts::PI, collections::{HashMap, HashSet}};

use bevy::{prelude::*, core_pipeline::clear_color::ClearColorConfig, window::WindowResized};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::{prelude::{RapierPhysicsPlugin, NoUserData, Velocity}, render::RapierDebugRenderPlugin};
use bevy_renet::{renet::{*, transport::*}, RenetServerPlugin, transport::NetcodeServerPlugin};
use renet_visualizer::RenetServerVisualizer;
use bevy_egui::{egui::{self, Style, Visuals, epaint::Shadow, Color32, Rounding, Align, Stroke, FontId}, EguiContexts, EguiPlugin};


//#[path = "settings.rs"] mod settings;
#[path = "console.rs"] mod console;
use console::*;
#[path = "game.rs"] mod game;
use game::*;
use game::components::*;
//#[path = "components.rs"] mod components;
//use components::*;


#[derive(Component)]
pub struct GameLabel;

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum ServerState {
    #[default]
    PreInit,
    Running
}

#[derive(Resource)]
pub struct ServerSettings{
    pub port: i16,
    pub max_clients: usize,
}


fn main(){
    let mut app = App::new();
    app.add_plugins((
        DefaultPlugins,
        EguiPlugin,
        WorldInspectorPlugin::new(),
        RenetServerPlugin,
        NetcodeServerPlugin,
        RapierPhysicsPlugin::<NoUserData>::pixels_per_meter(100.0), // ::<NoUserData>::pixels_per_meter(15.0)
        RapierDebugRenderPlugin{enabled: false, ..default()}
    ));
    app.add_state::<ServerState>();

    app.insert_resource(ClientsData::default());
    app.insert_resource(LoadedChunks{chunks: vec![]});
    app.insert_resource(GlobalConfig{
        map_size_chunks: Vec2{
            x: 3.,
            y: 3.
        },
        single_chunk_size: Vec2{
            x: 1000.,
            y: 1000.,
        },
        ..default()
    });
    app.insert_resource(ServerSettings{
        port: 8567,
        max_clients: 16,
    });

    //app.add_systems(OnEnter(ServerState::PreInit), setup_menu);
    app.add_systems(Update, menu.run_if(in_state(ServerState::PreInit)));
    //app.add_systems(OnExit(ServerState::PreInit), cleanup_menu);

    app.add_systems(OnEnter(ServerState::Running), (
        setup_game,
    ));
    app.add_systems(Update, (
        debug_chunk_render,
        resize_server_camera,

        check_bullet_collisions_and_lifetime,
        check_ship_collisions_and_lifetime,
        /*asteroids_refiller,*/

        (snap_objects, update_chunks_around, send_message_system).chain(),

        receive_message_system,
        handle_events_system,
        
        console_renderer,
        command_executer
        //check_bullet_collisions_and_lifetime
    ).run_if(in_state(ServerState::Running)));
    //app.add_systems(OnExit(ServerState::Running), cleanup_menu)

    app.add_event::<ServerEvent>();
    
    setup_commands_executer(&mut app, true);

    app.run();
}


fn menu(
    mut egui_context: EguiContexts,
    mut settings: ResMut<ServerSettings>,
    mut port_preview: Local<String>,
    mut next_state: ResMut<NextState<ServerState>>,
){
    let ctx = egui_context.ctx_mut();
    let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert("Font".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/VecTerminus12Medium.otf") )
    );
    
    fonts.families.insert(egui::FontFamily::Name("Font".into()), vec!["Font".to_owned()]);
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
            .insert(0, "Font".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
        .insert(0, "Font".to_owned());

    ctx.set_fonts(fonts);

    let style = Style{
        visuals: Visuals{
            window_rounding: Rounding::ZERO,
            window_shadow: Shadow::NONE,
            window_fill: Color32::from_rgba_unmultiplied(0, 0, 0, 230),
            window_stroke: Stroke{
                width: 1.,
                color: Color32::from_rgba_unmultiplied(255, 255, 255, 255)
            },
            button_frame: false,
            ..default()
        },
        animation_time: 0.,
        ..default()
    };
    ctx.set_style(style.clone());


    egui::Window::new("MENU")
        .anchor(egui::Align2([Align::Center, Align::Center]), [0., 100.])
        
        //.constrain(true)
        .resizable(false)
        //.default_height(100.0)
        .default_width(200.)
        
        .title_bar(false)
        .collapsible(false)
        
        .vscroll(false)
        .hscroll(false)
        
        //.fixed_size(bevy_egui::egui::Vec2{x: 100., y: 100.})
        .show(ctx, |ui|{
            ui.set_style(style.clone());

            let mut newstyle = (*ctx.style()).clone();
            newstyle.text_styles = [
                (egui::TextStyle::Button, FontId::new(34.0, egui::FontFamily::Monospace)),
                (egui::TextStyle::Body, FontId::new(34.0, egui::FontFamily::Monospace))
                ].into();
            ui.style_mut().text_styles = newstyle.text_styles;
            ui.add(egui::TextEdit::singleline(&mut *port_preview).char_limit(5).hint_text("PORT (8567)"));
            if let Ok(_) = (*port_preview).parse::<i32>(){
            } else if port_preview.len() != 0 {
                ui.add(egui::Label::new(egui::RichText::new("PORT IS INVALID!").color(Color32::RED)));
            }
            ui.add(egui::Slider::new(&mut settings.max_clients, 1..=64).suffix(" MAX PLAYERS"));
            let start_btn = ui.add(egui::Button::new("START SERVER")).clicked();
            if start_btn{
                // CHECK ALL, INIT SERVER AND CHANGE STATE
                next_state.set(ServerState::Running);
            }
        });
}


fn setup_game(
    mut commands: Commands,
    settings: Res<ServerSettings>,
    mut cfg: ResMut<GlobalConfig>,
    mut window: Query<&mut Window>,
    mut loaded_chunks: ResMut<LoadedChunks>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut time: Res<Time>,
    
){
    // INIT SERVER   
    let server = RenetServer::new(connection_config());
    commands.insert_resource(server);
    
    println!("PORT IS {}", settings.port);

    let server_addr = vec![format!("127.0.0.1:{}", settings.port).parse::<SocketAddr>().unwrap()];//format!("127.0.0.1:{}", settings.port).parse().unwrap(); SocketAddr::from("127.0.0.1:{}");
    
    commands.insert_resource(RenetServerVisualizer::<200>::default());
    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let socket = UdpSocket::bind(server_addr[0]).unwrap();
    const GAME_PROTOCOL_ID: u64 = 0;
    let server_config = ServerConfig {
        max_clients: settings.max_clients,
        protocol_id: GAME_PROTOCOL_ID,
        public_addresses: server_addr,
        current_time: current_time,
        authentication: ServerAuthentication::Unsecure // todo: change to secure
    };
    
    let transport = NetcodeServerTransport::new(server_config, socket).unwrap();
    commands.insert_resource(transport);
    println!("SERVER STARTED!!!!");
    let size = (cfg.map_size_chunks  + Vec2::from((2., 2.))) * cfg.single_chunk_size;
    let mid = cfg.map_size_chunks * cfg.single_chunk_size / 2.;
    let window_size = Vec2::from((window.single_mut().width(), window.single_mut().height()));
    let target_scale = if window_size.x / window_size.y < size.x / size.y{
        size.x / window_size.x
    } else {
        size.y / window_size.y
    };
    
    commands.spawn(
            Camera2dBundle{
                camera_2d: Camera2d {
                    clear_color: ClearColorConfig::Custom(Color::Rgba { red: 0., green: 0., blue: 0., alpha: 1. }),
                    ..default()
                },
                camera: Camera{
                    hdr: true,
                    ..default()
                },
                transform: Transform::from_xyz(mid.x, mid.y, 0.).with_scale(Vec3::splat(target_scale)),
                ..default()
            },
    );
    // INIT CHUNKS
    for x in -1..(cfg.map_size_chunks.x as i32 + 1){ // include shadow chunks
        for y in -1..(cfg.map_size_chunks.y as i32 + 1){
            loaded_chunks.chunks.push(Chunk { pos: Vec2::from((x as f32, y as f32)) });
        }
    }

    commands.insert_resource(ObjectsDistribution{data: HashMap::new()});
    
    // SPAWN ASTEROIDS
    for x in 0..cfg.map_size_chunks.x as u32{
        for y in 0..cfg.map_size_chunks.y as u32{
            let vel = Velocity{
                linvel: Vec2 { 
                    x: (rand::random::<f32>() - 0.5), 
                    y: (rand::random::<f32>() - 0.5) 
                } * 0./*500.*/, 
                angvel: (rand::random::<f32>() - 0.5) * 5.
            };
            let position = Transform::from_translation(
                Vec3::from([
                    (x as f32 + rand::random::<f32>()) * cfg.single_chunk_size.x,
                    (y as f32 + rand::random::<f32>()) * cfg.single_chunk_size.y,
                    0.
                ]));
            let seed = rand::random::<u64>();
            spawn_asteroid(seed, vel, position, &mut meshes, &mut materials, &mut commands, cfg.new_id(), cfg.get_asteroid_hp(seed));
        }
    }

    // INIT GAME
}


fn resize_server_camera(
    resize_event: Res<Events<WindowResized>>,
    cfg: ResMut<GlobalConfig>,
    mut camera_transform_q: Query<&mut Transform, With<Camera2d>>
){
    let mut reader = resize_event.get_reader();
    for e in reader.read(&resize_event) {
        let window_size = Vec2::from((e.width, e.height));
        let size = (cfg.map_size_chunks  + Vec2::from((2., 2.))) * cfg.single_chunk_size;
        let target_scale = if window_size.x / window_size.y < size.x / size.y{
            size.x / window_size.x
        } else {
            size.y / window_size.y
        };
        camera_transform_q.single_mut().scale = Vec3::splat(target_scale);
        //println!("width = {} height = {}", e.width, e.height);
    }
}

// Systems
fn send_message_system(
    mut server: ResMut<RenetServer>,
    clients_data: Res<ClientsData>,
    mut commands: Commands,
    mut objects_q: Query<(&Object, &Velocity, &Transform), (With<Object>, Without<Puppet>)>,
    mut objects_distribution: ResMut<ObjectsDistribution>, 
    cfg: ResMut<GlobalConfig>,
) {

    objects_distribution.data = HashMap::new();

    let mut chunk_to_objects: HashMap<(u32, u32), Vec<ObjectData>> = HashMap::new();

    for object in objects_q.iter(){
        let (object, velocity, transform) = object;

        

        let object_data = ObjectData{
            object: object.clone(),
            angular_velocity: velocity.angvel,
            linear_velocity: velocity.linvel,
            translation: transform.translation,
            rotation: transform.rotation,
        };

        let chunk_pos = cfg.pos_to_chunk(&transform.translation);
        let key = (chunk_pos.x as u32, chunk_pos.y as u32);

        if chunk_to_objects.contains_key(&key){
            let v = chunk_to_objects.get_mut(&key).unwrap();
            v.push(object_data);
        } else {
            chunk_to_objects.insert((chunk_pos.x as u32, chunk_pos.y as u32), vec![object_data]);
        }
        let is_player = match object.object_type{
            ObjectType::Ship { style: _, color: _, shields: _, hp: _, death_time: _ } => {true},
            _ => {false}
        };
        if objects_distribution.data.contains_key(&key){
            let (n, has_player, mut vec) = objects_distribution.data.get(&key).unwrap().clone();
            vec.push(transform.translation.truncate());
            
            objects_distribution.data.insert(key, (n + 1, is_player || has_player, vec));
        } else {
            objects_distribution.data.insert(key, (1, is_player, vec![transform.translation.truncate()]));
        }
    }


    for client_id in server.clients_id().into_iter() {
        let cd = clients_data.get_option_by_client_id(client_id.raw());
        if cd.is_some(){
            let e = cd.unwrap().entity;
            let obj = objects_q.get(e);
            if obj.is_ok(){
                let (iterated_ship, _, t) = obj.unwrap();

                
                let chunk = cfg.pos_to_chunk(&t.translation);
                let mut included_chunks = HashSet::new(); // exclude overlapping chunks if map is small size of (1; 1) -> 8*(1; 1) same chunks with same objects
                let mut personalised_data: Vec<ObjectData> = vec![];
                for x in (chunk.x as i32) - 1 .. chunk.x as i32 + 2 {
                    for y in (chunk.y as i32) - 1 .. chunk.y as i32 + 2{
                        let real_chunk = cfg.chunk_to_real_chunk_v2(&Vec2{x: x as f32, y: y as f32});
                        
                        let objects_in_chunk = chunk_to_objects.get(&(real_chunk.x as u32, real_chunk.y as u32));
                        if objects_in_chunk.is_some() && !included_chunks.contains(&(real_chunk.x as u32, real_chunk.y as u32)){
                            for object_data in objects_in_chunk.unwrap().iter(){
                                match object_data.object.object_type{
                                    ObjectType::Ship { style: _, color: _, shields: _, hp: _, death_time } => { // todo: invisibility power up
                                        if death_time == 0. || object_data.object.id == iterated_ship.id{
                                            personalised_data.push(object_data.clone());
                                        }
                                    },
                                    _ => {
                                        personalised_data.push(object_data.clone());
                                    }
                                }
                            }
                            included_chunks.insert((real_chunk.x as u32, real_chunk.y as u32));
                        }
                    }
                }
                let msg = Message::Update {
                    data: personalised_data
                };
                let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                server.send_message(client_id, ServerChannel::Fast, encoded);
            }
        }
    }
}


struct ServerSideVarables{
    shooting_cds: HashMap<u64, f32>, // client_id -> latest shoot time
}

impl Default for ServerSideVarables{
    fn default() -> Self {
        ServerSideVarables{
            shooting_cds: HashMap::new()
        }
    }
}

fn receive_message_system(
    mut server: ResMut<RenetServer>,
    mut clients_data: ResMut<ClientsData>,
    mut commands: Commands,
    mut objects_distribution: ResMut<ObjectsDistribution>,
    mut cfg: ResMut<GlobalConfig>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    transport: Res<NetcodeServerTransport>,
    mut server_side_varables: Local<ServerSideVarables>,
    mut ships_q: Query<(&mut Velocity, &Transform), (With<Ship>, Without<Puppet>)>,
    time: Res<Time>,
    asset_server: Res<AssetServer>,
) {
     // Send a text message for all clients
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::Fast) {
            let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
            match msg {
                Message::Inputs{ inputs } => {
                    let client_data_op = clients_data.get_option_by_client_id(client_id.raw());
                    if client_data_op.is_some() {
                        let client_data = client_data_op.unwrap();
                        let res = ships_q.get_mut(client_data.entity);

                        if res.is_ok(){
                            let (mut velocity, transform) = res.unwrap();

                            // MOVES
                            let mut target_direction = Vec2::ZERO;
                            if inputs.up    {target_direction.y += 1.5;} //  || buttons.pressed(MouseButton::Right
                            if inputs.down  {target_direction.y -= 0.75;}
                            if inputs.right {target_direction.x += 1.0;}
                            if inputs.left  {target_direction.x -= 1.0;}
                            
                            // let it be...
                            let target_angle = transform.up().truncate().angle_between(inputs.rotation_target);
                            if !target_angle.is_nan(){
                                velocity.angvel += ((target_angle * 180. / PI - velocity.angvel) * 1.).clamp(-90., 90.);//.clamp(-1.5, 1.5);
                            }
                            velocity.linvel += target_direction;

                            // SHOOTING
                            if inputs.shoot{
                                let exist = server_side_varables.shooting_cds.contains_key(&client_id.raw());
                                let current_time = time.elapsed().as_secs_f32();
                                if exist {
                                    let last_time = server_side_varables.shooting_cds.get(&client_id.raw()).unwrap().clone();
                                    if time.elapsed().as_secs_f32() - last_time > cfg.shoot_cd_secs{
                                        spawn_bullet(
                                            velocity.linvel + transform.up().truncate() * 1000., 
                                            *transform, 
                                            cfg.new_id(), 
                                            client_data.object_id, 
                                            current_time, 
                                            &asset_server, 
                                            &mut commands
                                        );
                                        
                                        server_side_varables.shooting_cds.insert(client_id.raw(), current_time);
                                    }
                                } else {
                                    spawn_bullet(
                                        velocity.linvel + transform.up().truncate() * 1000., 
                                        *transform, 
                                        cfg.new_id(), 
                                        client_data.object_id, 
                                        current_time, 
                                        &asset_server, 
                                        &mut commands
                                    );
                                    server_side_varables.shooting_cds.insert(client_id.raw(), current_time);
                                }
                            }
                        }
                    }
                }
                msg_type => {
                    warn!("Unhandled message recived on server!");
                }
            }
           // println!("{}", String::from_utf8(message.to_vec()).unwrap());
        }
        while let Some(message) = server.receive_message(client_id, ClientChannel::Garanteed) {
            // println!("{}", String::from_utf8(message.to_vec()).unwrap());
            let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
            match msg {
                Message::RegisterClient { style, color, name } => {
                    
                    // todo: check color!
                    
                    /* SPAWN */
                    let object_id = cfg.new_id();

                    let for_spawn_cl_data = ClientData::for_spawn(style, color, object_id);

                    let pos = get_pos_to_spawn(&mut objects_distribution, &mut cfg).extend(0.);
                    
                    let entity = spawn_ship(false, pos, &mut meshes, &mut materials, &mut commands, &for_spawn_cl_data, &mut cfg);

                    let new_client_data = ClientData { 
                        client_id: client_id.raw(),
                        object_id: object_id,
                        entity: entity,
                        style: style,
                        color: color, 
                        name: name.to_string() 
                    };
                    clients_data.add(new_client_data.clone());
                    println!("register new client with id {}", client_id);

                    // SEND DATA TO CONNECTED PLAYER
                    let mut cfg_clone = cfg.clone();
                    cfg_clone.debug_render = false;
                    let msg = Message::OnConnect{
                        clients_data: clients_data.clone(),
                        ship_object_id: object_id,
                        config: cfg_clone
                    };
                    let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                    server.send_message(client_id, ServerChannel::Garanteed, encoded);

                    // SEND CONNECTION MESSAGE TO ALL
                    let msg = Message::NewConnection {client_data: new_client_data};
                    let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                    server.broadcast_message(ServerChannel::Garanteed, encoded);
                    
                }
                msg_type => {
                    warn!("Unhandled message recived on server!");
                }
            }
        }
    }
}


fn handle_events_system(
    mut server_events: EventReader<ServerEvent>,
    mut server: ResMut<RenetServer>,
    mut visualizer: ResMut<RenetServerVisualizer<200>>,
    mut clients_data: ResMut<ClientsData>,
    mut commands: Commands,
    mut cfg: ResMut<GlobalConfig>,
    transport: Res<NetcodeServerTransport>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    for event in server_events.read() {
        //println!("{:?}", event);
        match event {
            ServerEvent::ClientConnected { client_id } => {
                // ADD CLIENT TO SERVER DB
                visualizer.add_client(*client_id);
                println!("New client with id {} connected", client_id);
                let encoded: Vec<u8> = bincode::serialize(&Message::Greeteng {}).unwrap();
                server.send_message(*client_id, ServerChannel::Garanteed, encoded);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                visualizer.remove_client(*client_id);
                println!("Client {client_id} disconnected: {reason}");
                let data = clients_data.get_option_by_client_id(client_id.raw());
                if data.is_some(){
                    commands.entity(data.unwrap().entity).despawn_recursive();
                }
                clients_data.remove_by_client_id(client_id.raw()); // todo: add reconnection (may be hard, but if use unique u64 for every client, possible)
                let msg = Message::NewDisconnection { id: client_id.raw()};
                let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                
                server.broadcast_message_except(*client_id, ServerChannel::Garanteed, encoded);
            }
        }
    }
}

