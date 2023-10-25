use std::collections::HashMap;
use bevy::prelude::{Component, Resource, Event, Vec2, Vec3, Transform, Entity};
use bevy_rapier2d::prelude::Velocity;
use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize)]
pub enum MessageType{
    OnConnect{
        clients_data: ClientsData,
        max_size: Vec2,
        single_chunk_size: Vec2,
    }, // MAP AND CLIENT DATA
    Update{

    }, // DATA ABOUT CHUNKS AROUND
    Inputs{
        
    }, // CLIENT INPUTS
    ChatMessage{
        sender_id: u64,
        message: String,
    }, // SENDER, MESSAGE
    NewConnection{
        client_data: ClientData
    }, // CLIENT DATA
    NewDisconnection{
        id: u64
    }, // CLIENT ID
    Kick{
        reason: String
    }, // REASON
    ERR,
}

#[derive(Serialize, Deserialize)]
pub struct OnConnectg_MSG{
    pub clients_data: ClientsData,
    pub max_size: Vec2,
    pub single_chunk_size: Vec2,
}
#[derive(Serialize, Deserialize)]
pub struct Update_MSG{

}
#[derive(Serialize, Deserialize)]
pub struct ChatMessage_MSG{
    pub sender_id: u64,
    pub message: String,
}
#[derive(Serialize, Deserialize)]
pub struct NewConnection_MSG{
    pub client_data: ClientData
}
#[derive(Serialize, Deserialize)]
pub struct NewDisconnection_MSG{
    pub id: u64
}
#[derive(Serialize, Deserialize)]
pub struct Kick_MSG{
    pub reason: String
}

#[derive(Serialize, Deserialize)]
pub struct MyData{
    pub color: [f32; 3],
    pub style: u8,
    pub name: String,
}

#[derive(Serialize, Deserialize)]
#[derive(Resource)]
pub struct ClientsData{
    binds: HashMap<u64, u64>, // object_id -> client_id
    data: HashMap<u64, ClientData> // client_id -> data
} 

#[derive(Serialize, Deserialize)]
#[derive(Component)]
pub struct ClientData{ 
    pub client_id: u64,
    pub object_id: u64,
    pub style: u8,
    pub color: [f32; 3],
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
impl ClientsData{
    pub fn get_by_object_id(&self, key: u64) -> &ClientData{
        self.data.get(self.binds.get(&key).unwrap()).unwrap()
    }
    pub fn get_by_client_id(&self, key: u64) -> &ClientData{
        self.data.get(&key).unwrap()
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


#[derive (Resource)]
pub struct MapSettings{
    pub last_id: u64,
    pub max_size: Vec2, //          !!!MUST BE INTEGER!!!
    pub single_chunk_size: Vec2, // !!!MUST BE INTEGER!!!
    pub debug_render: bool,
}


impl MapSettings{
    pub fn new_id(&mut self) -> u64{ // ID 0 IS EMPTY!!!!
        self.last_id += 1;
        return self.last_id;
    }
    pub fn pos_to_chunk(&self, pos: &Vec3) -> Vec2{
        Vec2{x: (pos.x / self.single_chunk_size.x).floor(), y: (pos.y / self.single_chunk_size.y).floor()}
    }
    
    pub fn pos_to_real_chunk(&self, pos: &Vec3) -> Vec2{
        let chunk = self.pos_to_chunk(pos);
        Vec2{x: chunk.x.rem_euclid(self.max_size.x), y: chunk.y.rem_euclid(self.max_size.y)}
    }
    pub fn pos_to_chunk_v2(&self, pos: &Vec2) -> Vec2{
        Vec2{x: (pos.x / self.single_chunk_size.x).floor(), y: (pos.y / self.single_chunk_size.y).floor()}
    }
    
    pub fn pos_to_real_chunk_v2(&self, pos: &Vec2) -> Vec2{
        let chunk = self.pos_to_chunk_v2(pos);
        Vec2{x: chunk.x.rem_euclid(self.max_size.x), y: chunk.y.rem_euclid(self.max_size.y)}
    }

    pub fn chunk_to_real_chunk_v2(&self, chunk: &Vec2) -> Vec2{
        Vec2{x: chunk.x.rem_euclid(self.max_size.x), y: chunk.y.rem_euclid(self.max_size.y)}
    }

    pub fn chunk_to_offset(&self, chunk: &Vec2) -> Vec2{
        Vec2{x: chunk.x * self.single_chunk_size.x, y: chunk.y * self.single_chunk_size.y}
    }
}

#[derive(Resource)]
pub struct LoadedChunks{ pub chunks: Vec<Chunk> }

#[derive(Event)]
pub struct BrokeAsteroid( pub Entity );

#[derive(Event)]
pub struct SpawnBullet{ 
    pub transform: Transform, 
    pub velocity: Velocity,
    pub owner: u64 
}

#[derive(Event)]
pub struct SpawnAsteroid{ pub transform: Transform, pub velocity: Velocity, pub seed: u64}


#[derive(Event)]
pub struct SpawnShip{pub id: u64, pub for_preview: bool}

pub enum GameRenderLayers{
    _Main,
    PixelCamera,
    PreviewCamera,
}


#[derive (Component)]
pub struct Object{
    pub id: u64,
    pub object_type: ObjectType
}

#[derive (Component, Clone)]
pub struct Puppet{pub id: u64, pub binded_chunk: Chunk}

impl Puppet{
    pub fn empty() -> Self{
        return Puppet{id:0, binded_chunk: Chunk { pos: Vec2::ZERO }}
    }
}

#[derive (Component, Clone)]
pub struct Chunk{pub pos: Vec2}

#[derive (Component)]
pub struct Bullet{pub previous_position: Transform, pub spawn_time: f32, pub owner: u64}


#[derive (Component)]
pub struct Ship;

#[derive (Component)]
pub struct ControlledPlayer;

#[derive (Component)]
pub struct Debug;

#[derive (Component)]
pub struct PuppetPlayer;

#[derive (Component)]
pub struct Asteroid{
    pub seed: u64,
    pub hp: u64,
}

pub enum ObjectType{
    Asteroid,
    Bullet,
    Ship,
}

#[derive(Component)]
pub struct ShipPreview;