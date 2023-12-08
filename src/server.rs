use std::{net::{UdpSocket, SocketAddr}, time::SystemTime};

use bevy::{prelude::*, core_pipeline::clear_color::ClearColorConfig, window::WindowResized, transform::commands};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::{prelude::{RapierPhysicsPlugin, NoUserData, Velocity}, render::RapierDebugRenderPlugin};
use bevy_renet::{renet::{*, transport::*}, RenetServerPlugin, transport::NetcodeServerPlugin};
use renet_visualizer::RenetServerVisualizer;
use bevy_egui::{egui::{self, Style, Visuals, epaint::Shadow, Color32, Rounding, Align, Stroke, FontId}, EguiContexts, EguiPlugin};


//#[path = "settings.rs"] mod settings;
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
            x: 6.,
            y: 3.
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

        
        (snap_objects, update_chunks_around).chain(),
        // multiplayer connection systems
        send_message_system,
        receive_message_system,
        handle_events_system
    ).run_if(in_state(ServerState::Running)));
    //app.add_systems(OnExit(ServerState::Running), cleanup_menu)

    app.add_event::<ServerEvent>();
    

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
        fonts.font_data.insert("MinecraftRegular".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/F77MinecraftRegular-0VYv.ttf") )
    );
    
    fonts.families.insert(egui::FontFamily::Name("MinecraftRegular".into()), vec!["MinecraftRegular".to_owned()]);
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap()
            .insert(0, "MinecraftRegular".to_owned());
    fonts.families.get_mut(&egui::FontFamily::Monospace).unwrap()
        .insert(0, "MinecraftRegular".to_owned());

    ctx.set_fonts(fonts);

    let style = Style{
        visuals: Visuals{
            window_rounding: Rounding::none(),
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
        authentication: ServerAuthentication::Unsecure
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
    
    // SPAWN ASTEROIDS
    for x in 0..cfg.map_size_chunks.x as u32{
        for y in 0..cfg.map_size_chunks.y as u32{
            let vel = Velocity{
                linvel: Vec2 { 
                    x: (rand::random::<f32>() - 0.5), 
                    y: (rand::random::<f32>() - 0.5) 
                } * 500., 
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
    for e in reader.iter(&resize_event) {
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
    mut objects_q: Query<(&Object, &Velocity, &Transform), (With<Object>, Without<Puppet>)>
) {

    // todo: SEND ONLY 9 CHUNKS AROUND!!! (or no...)

    let mut data: Vec<ObjectData> = vec![];
    for object in objects_q.iter(){
        let (object, velocity, transform) = object;
        data.push(
            ObjectData{
                object: object.clone(),
                angular_velocity: velocity.angvel,
                linear_velocity: velocity.linvel,
                translation: transform.translation,
                rotation: transform.rotation,
            }
        )
    }


    for client_id in server.clients_id().into_iter() {
        

        let msg = Message::Update {
            data: data.clone()
        };
        let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
        server.send_message(client_id, ServerChannel::Fast, encoded);
    }
}


fn receive_message_system(
    mut server: ResMut<RenetServer>,
    clients_data: Res<ClientsData>,
    mut commands: Commands,
    mut ships_q: Query<(&mut Velocity, &Transform), (With<Ship>, Without<Puppet>)>
) {
     // Send a text message for all clients
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, ClientChannel::Fast) {
            let msg: Message = bincode::deserialize::<Message>(&message).unwrap();
            match msg {
                Message::Inputs{ keys, rotation_direction } => {
                    let client_data_op = clients_data.get_option_by_client_id(client_id.raw());
                    if client_data_op.is_some() {
                        let client_data = clients_data.get_by_client_id(client_id.raw());
                        let res = ships_q.get_mut(client_data.entity);

                        if res.is_ok(){
                            let (mut velocity, transform) = res.unwrap();
    
                            velocity.angvel += rotation_direction.clamp(-3., 3.) * 0.01;
        
                            let mut target_direction = Vec2::ZERO;
                            if keys.up    {target_direction.y += 1.5;} //  || buttons.pressed(MouseButton::Right
                            if keys.down  {target_direction.y -= 0.75;}
                            if keys.right {target_direction.x += 1.0;}
                            if keys.left  {target_direction.x -= 1.0;}
                            
                            velocity.linvel += Vec2::from((transform.up().x, transform.up().y)) * target_direction.y * 2.0;
                            velocity.linvel += Vec2::from((transform.right().x, transform.right().y)) * target_direction.x * 2.0;
                        }
                    }
                }
                msg_type => {
                    warn!("Unhandled message with id {} recived on client!", u8::from(msg_type));
                }
            }
           // println!("{}", String::from_utf8(message.to_vec()).unwrap());
        }
        while let Some(_message) = server.receive_message(client_id, ClientChannel::Garanteed) {
            // println!("{}", String::from_utf8(message.to_vec()).unwrap());
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
    for event in server_events.iter() {
        //println!("{:?}", event);
        match event {
            ServerEvent::ClientConnected { client_id } => {

                // ADD CLIENT TO SERVER DB
                visualizer.add_client(*client_id);
                let data = transport.user_data(*client_id).unwrap();
                let mut byte_seq:Vec<u8> = vec![];
                let mut firstbyte = false;
                for i in 0..(256 - 4){
                    let d = data[255 - i];
                    if d != 0 || firstbyte == true{
                        byte_seq.push(d);
                        firstbyte = true;
                    }
                }
                byte_seq.reverse();
                let name = match std::str::from_utf8(&byte_seq) {
                    Ok(s) => {s}
                    Err(_) => {todo!("KICK")}
                };
                
                /* SPAWN */
                let object_id = cfg.new_id();

                let for_spawn_cl_data = ClientData::for_spawn(data[3], [data[0] as f32 / 255., data[1] as f32 / 255., data[2] as f32 / 255.], object_id);
                let entity = spawn_ship(false, &mut meshes, &mut materials, &mut commands, &for_spawn_cl_data);



                let new_client_data = ClientData { 
                    client_id: client_id.raw(),
                    object_id: object_id,
                    entity: entity,
                    style: data[3],
                    color: [data[0] as f32 / 255., data[1] as f32 / 255., data[2] as f32 / 255.], 
                    name: name.to_string() 
                };
                clients_data.add(new_client_data.clone());
                println!("register new client with id {}", client_id);

                
                
                
                // SEND DATA TO CONNECTED PLAYER
                let cfg_clone = cfg.clone();
                cfg.debug_render = false;
                let msg = Message::OnConnect{
                    clients_data: clients_data.clone(),
                    ship_object_id: object_id,
                    config: cfg_clone
                };
                let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                server.send_message(*client_id, ServerChannel::Garanteed, encoded);




                // SEND CONNECTION MESSAGE TO ALL
                let msg = Message::NewConnection {client_data: new_client_data};
                let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                server.broadcast_message(ServerChannel::Garanteed, encoded);
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                visualizer.remove_client(*client_id);
                println!("Client {client_id} disconnected: {reason}");
            }
        }
    }
}

