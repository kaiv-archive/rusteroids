//#![windows_subsystem = "windows"]
use std::{net::{UdpSocket, SocketAddr}, time::SystemTime, path::Path, f32::consts::PI, collections::BTreeMap, ops::RangeInclusive, any};

use bevy::{render::{view::window, mesh::Indices, render_resource::PrimitiveTopology, color::SrgbColorSpace}, window::WindowResized, asset::FileAssetIo, utils::label, sprite::{MaterialMesh2dBundle, Mesh2dHandle}, input::keyboard::KeyboardInput, prelude::*, DefaultPlugins, app::AppExit, core_pipeline::{tonemapping::{Tonemapping, DebandDither}, bloom::BloomCompositeMode}};
use bevy_egui::{egui::{self, Style, Visuals, epaint::{Shadow, self, Vertex, Hsva}, Color32, Rounding, FontDefinitions, Align, Stroke, FontId, WidgetInfo, Frame, emath, Pos2, vec2}, EguiContexts};
use bevy_inspector_egui::{quick::WorldInspectorPlugin, bevy_egui::{EguiPlugin, EguiContext}};
use bevy_rapier2d::na::{Translation, U4};
use bevy_renet::{renet::{*, transport::*}, RenetServerPlugin, transport::{NetcodeServerPlugin, NetcodeClientPlugin}, RenetClientPlugin};
use rand::random;
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

    app.add_plugins(RenetClientPlugin);
    app.add_plugins(NetcodeClientPlugin);
    app.insert_resource(RenetServerVisualizer::<200>::default());
    let client = RenetClient::new(ConnectionConfig::default());
    app.insert_resource(client);
    app.insert_resource(MapSettings{
        last_id: 0,
        max_size: Vec2{x: 5., y: 5.},
        single_chunk_size: Vec2{x: 500., y: 500.},
        debug_render: true,
    });
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
            (egui_based_menu, update_preview_ship).chain(),
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
                    
            (snap_objects, update_chunks_around).chain(),
            spawn_asteroid,


            receive_message_system,
            send_message_system,
    ).run_if(in_state(ClientState::InGame)));
   

    app.add_systems(Update, spawn_ship,);


    app.insert_resource(ConnectProperties{adress: "".into()});

    app.add_event::<SpawnMenuBeam>();
    app.add_event::<InitClient>();

    app.add_event::<SpawnShip>();
    app.add_event::<SpawnAsteroid>();

    app = game::init_pixel_camera(app);

    app.run()
}




fn init_client(
    mut reader: EventReader<InitClient>,
    time: Res<Time>,
    settings: Res<GameSettings>,
    mut commands: Commands,
    mut connect_properties: ResMut<ConnectProperties>,
    
    // todo: FOR DEBUG => REMOVE IT
    mut clients_data: ResMut<ClientsData>,
    mut writer: EventWriter<SpawnShip>,
){  
    for e in reader.iter(){
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

        clients_data.add(
            ClientData {
                client_id: 0,
                object_id: 1,
                style: e.style,
                color: color,
                name: "CURSED".into()
            }
        );
        writer.send(SpawnShip { id: 0, for_preview: false });


        commands.insert_resource(RenetClient::new(ConnectionConfig::default()));
        commands.insert_resource(transport);
    }
}



fn send_message_system(
    mut client: ResMut<RenetClient>
){
    // Send a text message to the server
    //client.send_message(DefaultChannel::ReliableOrdered, "HI FROM CLIENT!".as_bytes().to_vec());
}

fn receive_message_system(
    mut client: ResMut<RenetClient>,
    mut map: ResMut<MapSettings>
) {
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        let msg: MessageType = bincode::deserialize::<MessageType>(&message).unwrap();
        match msg {
            MessageType::OnConnect{ clients_data, max_size, single_chunk_size } => {
                map.max_size = max_size;
                map.single_chunk_size = single_chunk_size;
            },
            _ => {}
        }

        // println!("{}", String::from_utf8(message.to_vec()).unwrap());
    }
    while let Some(_message) = client.receive_message(DefaultChannel::ReliableUnordered) {
         // println!("{}", String::from_utf8(message.to_vec()).unwrap());
    }
    while let Some(_message) = client.receive_message(DefaultChannel::Unreliable) {
         // println!("{}", String::from_utf8(message.to_vec()).unwrap());
    }
    // Send a text message to the server
    //client.send_message(DefaultChannel::ReliableOrdered, "HI FROM CLIENT!".as_bytes().to_vec());
}
