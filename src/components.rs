use std::{collections::HashMap, time::Duration};
use bevy::{prelude::{Component, Resource, Event, Vec2, Vec3, Transform, Entity, Quat}, render::color::Color, ecs::schedule::States};
use bevy_rapier2d::prelude::Velocity;
use bevy_renet::renet::{ChannelConfig, SendType, ConnectionConfig};
use rand::{SeedableRng, Rng};
use rand_chacha::ChaCha8Rng;
use serde::{Serialize, Deserialize};

#[derive(Resource)]
pub struct ConnectProperties{
    pub adress: String
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq, Hash, States)]
pub enum ClientState {
    #[default]
    Menu,
    InGame
}

#[derive(Serialize, Deserialize)]
pub enum Message{
    Greeteng{

    },
    RegisterClient{
        style: u8,
        color: Color,
        name: String
    },
    OnConnect{ // MAP AND CLIENT DATA
        clients_data: ClientsData,
        ship_object_id: u64,
        config: GlobalConfig
    }, 
    Update{ // DATA ABOUT CHUNKS AROUND
        data: Vec<ObjectData>
    }, 
    Inputs{ // CLIENT INPUTS
        inputs: InputKeys,
    }, 
    ChatMessage{ // SENDER, MESSAGE
        sender_id: u64,
        message: String,
    }, 
    NewConnection{ // CLIENT DATA
        client_data: ClientData
    }, 
    NewDisconnection{ // CLIENT ID
        id: u64
    }, 
    Kick{ // REASON
        reason: String
    }, 
    ERR,
}

#[derive(Serialize, Deserialize)]
#[derive(Clone)]
pub struct ObjectData{
    pub object: Object,
    pub angular_velocity: f32,
    pub linear_velocity: Vec2,
    pub translation: Vec3,
    pub rotation: Quat,
}


#[derive(Serialize, Deserialize)]
pub struct MyData{
    pub color: [f32; 3],
    pub style: u8,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
#[derive(Clone)]
#[derive(Resource)]
pub struct ClientsData{
    binds: HashMap<u64, u64>, // object_id -> client_id
    data: HashMap<u64, ClientData> // client_id -> data
}




#[derive(Serialize, Deserialize)]
#[derive(Clone)]
#[derive(Component)]
pub struct ClientData{ 
    pub client_id: u64,
    pub object_id: u64,
    pub entity: Entity,
    pub style: u8,
    pub color: Color,
    pub name: String,
}
impl Default for ClientsData {
    fn default() -> Self {
        ClientsData{
            binds: HashMap::new(),
            data: HashMap::new()
        }
    }
}

impl ClientData{
    pub fn for_spawn(
        style: u8,
        color: Color,
        object_id: u64,
    ) -> Self{
        ClientData {
            client_id: 0,
            object_id: object_id,
            style: style,
            entity: Entity::PLACEHOLDER,
            color: color,
            name: "PLACEHOLDER".into()
        }
    }
}


impl ClientsData{
    pub fn clean_exclude_me(&mut self){
        let to_save = self.get_by_client_id(0).clone();
        self.binds = HashMap::new();
        self.data = HashMap::new();
        self.add(to_save);
    }
    pub fn get_option_by_object_id(&self, key: u64) -> Option<&ClientData>{
        let key = self.binds.get(&key);
        if key.is_some(){
            return self.data.get(key.unwrap());
        }
        return None;
    }
    pub fn get_by_object_id(&self, key: u64) -> &ClientData{
        self.data.get(self.binds.get(&key).unwrap()).unwrap()
    }
    pub fn get_by_client_id(&self, key: u64) -> &ClientData{
        self.data.get(&key).unwrap()
    }
    pub fn get_option_by_client_id(&self, key: u64) -> Option<&ClientData>{
        self.data.get(&key)
    }
    pub fn get_mut_by_object_id(&mut self, key: u64) -> &mut ClientData{
        self.data.get_mut(self.binds.get(&key).unwrap()).unwrap()
    }
    pub fn get_mut_by_client_id(&mut self, key: u64) -> &mut ClientData{
        self.data.get_mut(&key).unwrap()
    }
    pub fn add(&mut self, data: ClientData){
        self.binds.insert(data.object_id, data.client_id);
        self.data.insert(data.client_id, data);
    }
    pub fn remove_by_object_id(&mut self, key: u64){
        let k = self.binds.get(&key).unwrap();
        self.data.remove(k);
        self.binds.remove(&key);
    }
    pub fn remove_by_client_id(&mut self, key: u64){
        self.data.remove(&key);
        self.binds.remove(&key);
    }
}


#[derive(Component)]
pub struct CameraCanvas;

#[derive(Component)]
pub struct PixelCamera;

#[derive(Event)]
pub enum ApplyCameraSettings{
    Tonemapping,
    BloomCompositeMode,
    Intensity,
    LowFrequencyBoost,
    LowFrequencyBoostCurvature,
    HighPassFrequency,
    Threshold,
    ThresholdSoftness,
    DebandDither,
}

#[derive(Serialize, Deserialize)]
#[derive (Clone)]
#[derive (Resource)]
pub struct GlobalConfig{
    // MAP
    pub last_id: u64,

    pub map_size_chunks: Vec2, //   !!!MUST BE INTEGER!!!
    pub single_chunk_size: Vec2, // !!!MUST BE INTEGER!!!
    pub asteroids_per_chunk: f32,

    pub debug_render: bool,
    // OBJECTS
    pub asteroid_hp: [i8; 3],
    pub player_hp: f32,
    pub player_shields: f32,
    pub bullet_damage: f32,
    // TIMERS
    pub dash_cd_secs: f32,
    pub shoot_cd_secs: f32,
    pub bullet_lifetime_secs: f32,
    pub respawn_time_secs: f32,

}
impl Default for GlobalConfig {
    fn default() -> Self {
        GlobalConfig{
            last_id: 0,
            map_size_chunks: Vec2{x: 5., y: 5.},
            single_chunk_size: Vec2{x: 500., y: 500.},
            asteroids_per_chunk: 1.,
            debug_render: false,
            asteroid_hp: [1, 1, 1],
            player_hp: 100.,
            player_shields: 100.,
            bullet_damage: 200.,
            dash_cd_secs: 3.,
            shoot_cd_secs: 0.5,
            bullet_lifetime_secs: 10.,
            respawn_time_secs: 5.
        }
    }
}

pub fn get_asteroid_size(seed: u64) -> u8{
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
     match rng.gen_range(0..16) {
        0..=6 => 1,
        7..=14 => 2,
        15..=16 => 3,
        e => {println!("{}", e); 1}
    }
}

impl GlobalConfig{
    pub fn get_asteroid_hp(&mut self, seed: u64) -> u8{
        *self.asteroid_hp.get(get_asteroid_size(seed) as usize - 1).unwrap() as u8
    }
    pub fn new_id(&mut self) -> u64{ // ID 0 IS EMPTY!!!!
        self.last_id += 1;
        return self.last_id;
    }
    pub fn pos_to_chunk(&self, pos: &Vec3) -> Vec2{
        Vec2{x: (pos.x / self.single_chunk_size.x).floor(), y: (pos.y / self.single_chunk_size.y).floor()}
    }
    
    pub fn pos_to_real_chunk(&self, pos: &Vec3) -> Vec2{
        let chunk = self.pos_to_chunk(pos);
        Vec2{x: chunk.x.rem_euclid(self.map_size_chunks.x), y: chunk.y.rem_euclid(self.map_size_chunks.y)}
    }
    pub fn pos_to_chunk_v2(&self, pos: &Vec2) -> Vec2{
        Vec2{x: (pos.x / self.single_chunk_size.x).floor(), y: (pos.y / self.single_chunk_size.y).floor()}
    }
    
    pub fn pos_to_real_chunk_v2(&self, pos: &Vec2) -> Vec2{
        let chunk = self.pos_to_chunk_v2(pos);
        Vec2{x: chunk.x.rem_euclid(self.map_size_chunks.x), y: chunk.y.rem_euclid(self.map_size_chunks.y)}
    }

    pub fn chunk_to_real_chunk_v2(&self, chunk: &Vec2) -> Vec2{
        Vec2{x: chunk.x.rem_euclid(self.map_size_chunks.x), y: chunk.y.rem_euclid(self.map_size_chunks.y)}
    }

    pub fn chunk_to_offset(&self, chunk: &Vec2) -> Vec2{
        Vec2{x: chunk.x * self.single_chunk_size.x, y: chunk.y * self.single_chunk_size.y}
    }
}

#[derive(Resource)]
pub struct LoadedChunks{ pub chunks: Vec<Chunk> } // todo: fow what? (debug maybe)

#[derive(Resource)]
pub struct ObjectsDistribution{
    pub data: HashMap<(u32, u32), (u32, bool, Vec<Vec2>)>
}

#[derive(Event)]
pub struct BrokeAsteroid( pub Entity );

#[derive(Event)]
pub struct SpawnBullet{ 
    pub transform: Transform, 
    pub velocity: Velocity,
    pub owner: u64 
}

pub enum GameRenderLayers{
    _Main,
    PixelCamera,
    PreviewCamera,
}



#[derive(Serialize, Deserialize)]
#[derive (Component, Clone, Copy)]
pub struct Object{
    pub id: u64,
    pub object_type: ObjectType
}

#[derive (Component, Clone)]
pub struct Puppet{pub id: u64, pub binded_chunk: Chunk}
/*
impl Puppet{
    pub fn empty() -> Self{
        return Puppet{id:0, binded_chunk: Chunk { pos: Vec2::ZERO }}
    }
}
*/

#[derive (Component, Clone)]
pub struct Chunk{pub pos: Vec2}

#[derive (Component)]
pub struct Bullet;

#[derive (Component)]
pub struct Asteroid;

#[derive (Component)]
pub struct Ship;

#[derive(Component)]
pub struct PowerUPImage;

#[derive(Component)]
pub struct PowerUPCube;


#[derive (Component)]
pub struct Debug;

#[derive (Component)]
pub struct PuppetPlayer;


#[derive(Serialize, Deserialize)]
#[derive (Clone, Copy)]
pub enum ObjectType{
    Asteroid{seed: u64, hp: u8},
    Bullet{previous_position: Transform, spawn_time: f32, owner: u64},
    Ship{style: u8, color: Color, shields: f32, hp: f32},
    PickUP{pickup_type: PowerUPType},
}

#[derive(Serialize, Deserialize)]
#[derive (Clone, Copy)]
pub enum PowerUPType{
    Repair, // +--
    DoubleDamage, // +--
    Haste, //+--
    SuperShield, //+--
    Invisibility, //+--
}


#[derive(Serialize, Deserialize)]
pub enum ShipState{
    Regular{spawn_time: f32},
    Dash{start_time: f32},
    Dead{death_time: f32},
}


#[derive(Component)]
pub struct ShipPreview;



// ids and properties of fast and garanteed same for both client and server are the same. maybe use just one enum Channel?
pub enum ClientChannel { 
    Fast,
    Garanteed,
}

impl From<ClientChannel> for u8 {
    fn from(channel_id: ClientChannel) -> Self {
        match channel_id {
            ClientChannel::Fast => 0,
            ClientChannel::Garanteed => 1,
        }
    }
}

impl ClientChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::Fast.into(),
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::ZERO,
                },
            },
            ChannelConfig {
                channel_id: Self::Garanteed.into(),
                max_memory_usage_bytes: 5 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
        ]
    }
}

pub enum ServerChannel {
    Fast,
    Garanteed,
}

impl From<ServerChannel> for u8 {
    fn from(channel_id: ServerChannel) -> Self {
        match channel_id {
            ServerChannel::Fast => 0,
            ServerChannel::Garanteed => 1,
        }
    }
}

impl ServerChannel {
    pub fn channels_config() -> Vec<ChannelConfig> {
        vec![
            ChannelConfig {
                channel_id: Self::Fast.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::Unreliable,
            },
            ChannelConfig {
                channel_id: Self::Garanteed.into(),
                max_memory_usage_bytes: 10 * 1024 * 1024,
                send_type: SendType::ReliableOrdered {
                    resend_time: Duration::from_millis(200),
                },
            },
        ]
    }
}

pub fn connection_config() -> ConnectionConfig {
    ConnectionConfig {
        available_bytes_per_tick: 1024 * 1024,
        client_channels_config: ClientChannel::channels_config(),
        server_channels_config: ServerChannel::channels_config(),
    }
}

#[derive(Serialize, Deserialize)]
#[derive(Resource, PartialEq, Eq)]
pub enum InputType{
    Keyboard,
    Mouse
}
#[derive(Serialize, Deserialize)]
#[derive(Resource)]
pub struct InputKeys{
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub rotate_left: bool,
    pub rotate_right: bool,
    pub rotation_target: Vec2,
    pub stabilize: bool,
    pub shoot: bool,
    pub dash: bool,
    pub fixed_camera_z: bool,
    pub input_type: InputType,
}

impl Default for InputKeys{
    fn default() -> Self {
        InputKeys {
            up: false,
            down: false,
            left: false,
            right: false,
            rotate_left: false,
            rotate_right: false,
            rotation_target: Vec2::ZERO,
            stabilize: false,
            shoot: false,
            dash: false,
            fixed_camera_z: false,
            input_type: InputType::Mouse,
        }
    }
}