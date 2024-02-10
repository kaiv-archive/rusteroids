use std::{time::Duration};
use bevy::{prelude::{Component, Resource, Event, Vec2, Vec3, Transform, Entity, Quat}, render::color::Color, ecs::schedule::States, utils::HashMap};
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
    pub states_and_statuses: Option<(ShipState, ShipStatuses)>,
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


#[derive(Component)]
pub struct LastDamageTaken{pub time: f32} 


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
    pub fn get_by_client_id(&self, key: u64) -> &ClientData{
        self.data.get(&key).unwrap()
    }
    pub fn get_option_by_client_id(&self, key: &u64) -> Option<&ClientData>{
        self.data.get(key)
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

    pub debug_render: bool, // todo: move to ?
    // OBJECTS
    pub asteroid_hp: [i8; 3], // why i8? oh... number of hits
    pub player_hp: f32,
    pub player_shields: f32,
    pub shield_recharge_per_sec: f32,
    pub bullet_damage: f32,
    pub powerup_drop_chances: f32,
    // TIMERS
    pub dash_cd_secs: f32,
    pub dash_time: f32,
    pub dash_impulse: f32,
    pub shoot_cd_secs: f32,
    pub bullet_lifetime_secs: f32,
    pub shield_recharge_delay: f32,
    pub spawn_immunity_time: f32,
    pub respawn_time_secs: f32,

    pub effects_repair_amount: f32,
    pub effects_extradamage_secs: f32,
    pub effects_extradamage_amount: f32,
    pub effects_haste_secs: f32,
    pub effects_haste_amount: f32,
    pub effects_supershield_amount: f32,
    pub effects_invisibility_secs: f32,
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
            shield_recharge_per_sec: 10.,
            shield_recharge_delay: 5.,
            bullet_damage: 50.,
            powerup_drop_chances: 1.0,
            dash_cd_secs: 0.5, // todo: gui cd
            dash_time: 0.12,
            dash_impulse: 2000.,
            shoot_cd_secs: 0.5,
            bullet_lifetime_secs: 10.,
            spawn_immunity_time: 2.,
            respawn_time_secs: 5.,

            effects_repair_amount: 100.,
            effects_extradamage_secs: 3., // 15.,
            effects_extradamage_amount: 0.5,
            effects_haste_secs: 3., // 15.,
            effects_haste_amount: 2.,
            effects_supershield_amount: 50.,
            effects_invisibility_secs: 3., // 20.,
        }
    }
}

impl GlobalConfig {
    pub fn get_power_up_effect(&self, poweup_type: PowerUPType) -> PowerUPEffect {
        match poweup_type {
            PowerUPType::ExtraDamage => {
                PowerUPEffect{seconds: self.effects_extradamage_secs, value: self.effects_extradamage_amount}
            },
            PowerUPType::Haste => {
                PowerUPEffect{seconds: self.effects_haste_secs, value: self.effects_haste_amount}
            },
            PowerUPType::Invisibility => {
                PowerUPEffect{seconds: self.effects_invisibility_secs, value: f32::INFINITY}
            },
            PowerUPType::Repair => {
                PowerUPEffect{seconds: 0., value: self.effects_repair_amount}
            },
            PowerUPType::SuperShield => {
                PowerUPEffect{seconds: f32::INFINITY, value: self.effects_supershield_amount}
            }
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
pub struct Chunk{pub pos: Vec2} // todo: do smt

#[derive (Component)]
pub struct Bullet;

#[derive (Component)]
pub struct Asteroid;

#[derive (Component)]
pub struct Ship;

#[derive(Component)]
pub struct PowerUP;

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
    Bullet{previous_position: Transform, spawn_time: f32, owner: u64, extra_damage: bool},
    Ship{style: u8, color: Color, shields: f32, hp: f32},
    PickUP{pickup_type: PowerUPType},
}
/*
NEVER DO LIKE THAT ^^^^^ (ENUM COMPONENT STRUCTS WITH BEVY) WITH MUT PARAMETERS. LOOK AT MY CODE AND DONT DO LIKE THAT.

/ for example, changing one parameter (hp) looks like that:

fn heal(
    mut ship_q: Query<&mut Object>,
    cfg: Res<GlobalConfig>,
){
    let to_heal_object = ship_q.last().unwrap(); 

    let mut object_clone = object.clone();
    match object.object_type {
        ObjectType::Ship { style, color, shields, hp } => {
            object_clone.object_type = ObjectType::Ship { style, color, shields, hp: (hp + cfg.effects_repair_amount).clamp(0., cfg.player_hp)};
            *object = object_clone;
        }
        _ => {}
    }
}
*/


#[derive(Component)]
#[derive(Serialize, Deserialize)]
#[derive(Clone, Copy)]
#[derive(Hash, PartialEq, Eq)]
pub enum PowerUPType{
    Repair,
    ExtraDamage,
    Haste,
    SuperShield,
    Invisibility,
}

impl PowerUPType {
    pub fn texture_path(&self) -> String {
        match self {
            PowerUPType::ExtraDamage => { return "powerups/extradamage.png".to_owned()},
            PowerUPType::Haste => { return "powerups/haste.png".to_owned()},
            PowerUPType::Repair => { return "powerups/repair.png".to_owned()},
            PowerUPType::SuperShield => { return "powerups/supershield.png".to_owned()},
            PowerUPType::Invisibility => { return "powerups/invisibility.png".to_owned()},
        };
    }
}

#[derive (Component)]
#[derive(Serialize, Deserialize)]
#[derive (Clone, Copy)]
pub enum ShipState{
    Regular,
    Dash{start_time: f32, init_velocity: Vec2},
    Dead{time: f32},
}
#[derive (Component)]
#[derive(Serialize, Deserialize)]
#[derive (Clone)]
pub struct ShipStatuses{
    pub current: HashMap<PowerUPType, PowerUPEffect>
}

impl ShipStatuses {
    pub fn has_extra_damage(&self) -> bool{
        self.current.contains_key(&PowerUPType::ExtraDamage)
    }
    pub fn has_haste(&self) -> bool{
        self.current.contains_key(&PowerUPType::Haste)
    }
    pub fn has_super_shield(&self) -> bool{
        self.current.contains_key(&PowerUPType::SuperShield)
    }
    pub fn has_invisibility(&self) -> bool{
        self.current.contains_key(&PowerUPType::Invisibility)
    }
}

#[derive(Serialize, Deserialize)]
#[derive (Clone)]
pub struct PowerUPEffect{
    pub seconds: f32,
    pub value: f32
}
impl PowerUPEffect{
    pub fn get_val_to_show(&self, power_up_type: &PowerUPType) -> f32{
        match power_up_type {
            PowerUPType::Repair => {self.value}
            PowerUPType::ExtraDamage => {self.seconds}
            PowerUPType::Haste => {self.seconds}
            PowerUPType::SuperShield => {self.value}
            PowerUPType::Invisibility => {self.seconds}
        }
    }
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