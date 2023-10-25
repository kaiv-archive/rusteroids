use std::{net::UdpSocket, time::SystemTime, collections::HashMap};

use bevy::{prelude::*, core_pipeline::clear_color::ClearColorConfig, window::WindowResized, transform::commands};
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::{prelude::{Velocity, RapierPhysicsPlugin, NoUserData}, render::RapierDebugRenderPlugin};
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
    app.insert_resource(MapSettings{
        last_id: 0,
        max_size: Vec2{x: 5., y: 2.},
        single_chunk_size: Vec2{x: 500., y: 500.},
        debug_render: true,
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
        spawn_asteroid,
        spawn_ship,
        // multiplayer connection systems
        send_message_system,
        receive_message_system,
        handle_events_system
    ).run_if(in_state(ServerState::Running)));
    //app.add_systems(OnExit(ServerState::Running), cleanup_menu)

    app.add_event::<SpawnShip>();
    app.add_event::<SpawnAsteroid>();
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
    map: ResMut<MapSettings>,
    mut window: Query<&mut Window>,
    mut asteroid_event: EventWriter<SpawnAsteroid>,
    mut loaded_chunks: ResMut<LoadedChunks>,
){
    // INIT SERVER   

    let server = RenetServer::new(ConnectionConfig::default());
    commands.insert_resource(server);
    
    println!("PORT IS {}", settings.port);

    let server_addr = format!("127.0.0.1:{}", settings.port).parse().unwrap();

    commands.insert_resource(RenetServerVisualizer::<200>::default());
    let socket = UdpSocket::bind(server_addr).unwrap();

    const GAME_PROTOCOL_ID: u64 = 0;
    let server_config = ServerConfig {
        max_clients: settings.max_clients,
        protocol_id: GAME_PROTOCOL_ID,
        public_addr: server_addr,
        authentication: ServerAuthentication::Unsecure
    };

    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();
    let transport = NetcodeServerTransport::new(current_time, server_config, socket).unwrap();
    commands.insert_resource(transport);
    println!("SERVER STARTED!!!!");
    let size = (map.max_size  + Vec2::from((2., 2.))) * map.single_chunk_size;
    let mid = map.max_size * map.single_chunk_size / 2.;
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
    for x in -1..(map.max_size.x as i32 + 1){ // include shadow chunks
        for y in -1..(map.max_size.y as i32 + 1){
            loaded_chunks.chunks.push(Chunk { pos: Vec2::from((x as f32, y as f32)) });
        }
    }
    // SPAWN ASTEROIDS
    //for x in 0..(map.max_size.x as i32){
    //    for y in 0..(map.max_size.y as i32){
    //        for _ in 0..((rand::random::<f32>() * 4.) as i32){
    //            let x = x as f32;
    //            let y = y as f32;
    //            asteroid_event.send(SpawnAsteroid {
    //                transform: Transform::from_xyz(
    //                    x * map.single_chunk_size.x + map.single_chunk_size.x * rand::random::<f32>(), 
    //                    y * map.single_chunk_size.y + map.single_chunk_size.y * rand::random::<f32>(), 
    //                    0.),
    //                velocity: Velocity {
    //                    linvel: Vec2::from((
    //                        rand::random::<f32>() - 0.5,
    //                        rand::random::<f32>() - 0.5
    //                    )) * 1000.,
    //                    angvel: rand::random::<f32>() - 0.5
    //                },
    //                seed: rand::random::<u64>()
    //            });
    //        }
    //    }
    //}
    //app.add_systems(Update, send_message_system);
    //app.add_systems(Update, receive_message_system);
    //app.add_systems(Update, handle_events_system);
    // INIT GAME
}


fn resize_server_camera(
    resize_event: Res<Events<WindowResized>>,
    map: ResMut<MapSettings>,
    mut camera_transform_q: Query<&mut Transform, With<Camera2d>>
){
    let mut reader = resize_event.get_reader();
    for e in reader.iter(&resize_event) {
        let window_size = Vec2::from((e.width, e.height));
        let size = (map.max_size  + Vec2::from((2., 2.))) * map.single_chunk_size;
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
fn send_message_system(mut server: ResMut<RenetServer>) {
    // let channel_id = 0;
    // Send a text message for all clients
    // The enum DefaultChannel describe the channels used by the default configuration
    //server.broadcast_message(DefaultChannel::ReliableOrdered, "server message".as_bytes().to_vec());
}


fn receive_message_system(mut server: ResMut<RenetServer>) {
     // Send a text message for all clients
    for client_id in server.clients_id().into_iter() {
        while let Some(_message) = server.receive_message(client_id, DefaultChannel::ReliableOrdered) {
           // println!("{}", String::from_utf8(message.to_vec()).unwrap());
        }
        while let Some(_message) = server.receive_message(client_id, DefaultChannel::ReliableUnordered) {
            // println!("{}", String::from_utf8(message.to_vec()).unwrap());
        }
        while let Some(_message) = server.receive_message(client_id, DefaultChannel::Unreliable) {
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
    mut map: ResMut<MapSettings>,
    transport: Res<NetcodeServerTransport>,
    mut spawn_ship: EventWriter<SpawnShip>,
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
                let object_id = map.new_id();
                spawn_ship.send(SpawnShip { id: *client_id, for_preview: false });

                clients_data.add(
                    ClientData { 
                        client_id:*client_id,
                        object_id: object_id,
                        style: data[3],
                        color: [data[0] as f32 / 255., data[1] as f32 / 255., data[2] as f32 / 255.], 
                        name: name.to_string() 
                });

                
                // SEND DATA TO CONNECTED PLAYER
                let msg = MessageType::OnConnect{
                    clients_data: ClientsData::default(),
                    max_size: map.max_size,
                    single_chunk_size: map.single_chunk_size,
                };
                let encoded: Vec<u8> = bincode::serialize(&msg).unwrap();
                server.send_message(*client_id, DefaultChannel::ReliableOrdered, encoded);

                // SEND CONNECTION MESSAGE TO ALL
                //server.broadcast_message(DefaultChannel::ReliableOrdered, message)
            }
            ServerEvent::ClientDisconnected { client_id, reason } => {
                visualizer.remove_client(*client_id);
                println!("Client {client_id} disconnected: {reason}");
            }
        }
    }
}

