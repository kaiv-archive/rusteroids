use std::{net::UdpSocket, time::{SystemTime, Duration}};

use bevy::prelude::*;
use bevy_renet::{renet::{*, transport::*}, RenetServerPlugin, transport::{NetcodeServerPlugin, NetcodeClientPlugin}};
use bevy_renet::*;
use renet_visualizer::*;
pub fn main(
    
){
    //println!()
    let mut app = App::new();
    app.add_plugins(DefaultPlugins); 

    app.add_plugins(RenetClientPlugin);

    let client = RenetClient::new(ConnectionConfig::default());
    app.insert_resource(client);
    
    // Setup the transport layer
    app.add_plugins(NetcodeClientPlugin);
    app.insert_resource(RenetServerVisualizer::<200>::default());
    
    app.add_systems(Startup, init_client);
    app.add_systems(Update, (send_message_system, receive_message_system));
    app.run();
}



fn init_client(
    time: Res<Time>,
    mut commands: Commands,
){
    let server_addr = "127.0.0.1:5000".parse().unwrap();
    let public_addr = "127.0.0.1:0";
    let socket = UdpSocket::bind(public_addr).unwrap();
    //UdpSocket::
    const GAME_PROTOCOL_ID: u64 = 0;


    let current_time = SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap();

    let transport = NetcodeClientTransport::new(
        current_time, 
        ClientAuthentication::Unsecure {
             protocol_id: GAME_PROTOCOL_ID,
             client_id: current_time.as_millis() as u64,
             server_addr: server_addr,
             user_data: Some([0 as u8; 256])
        }, 
        socket
    ).unwrap();

    commands.insert_resource(RenetClient::new(ConnectionConfig::default()));
    println!("{}", transport.is_connecting());
    commands.insert_resource(transport);
    

}



// Systems


fn send_message_system(mut client: ResMut<RenetClient>) {
    // Send a text message to the server
    client.send_message(DefaultChannel::ReliableOrdered, "server message".as_bytes().to_vec());
}

fn receive_message_system(mut client: ResMut<RenetClient>) {
    while let Some(message) = client.receive_message(DefaultChannel::ReliableOrdered) {
        // Handle received message
    }
}