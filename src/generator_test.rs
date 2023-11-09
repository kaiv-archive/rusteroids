use std::f32::consts::PI;

use bevy_inspector_egui::quick::WorldInspectorPlugin;
use bevy_rapier2d::prelude::Vect;
use rand::{SeedableRng, Rng};
use rand_chacha::ChaCha8Rng;
use bevy::{prelude::*, core_pipeline::clear_color::ClearColorConfig, render::{render_resource::PrimitiveTopology, mesh::Indices}, sprite::{MaterialMesh2dBundle, Mesh2dHandle}};

#[derive (Component)]
struct Asteroid;

pub fn main(){
    let mut app = App::new();
    app.add_plugins((DefaultPlugins.set(
        ImagePlugin::default_nearest()
        ),
        WorldInspectorPlugin::new()
    ));
    app.add_systems(Startup, _on_ready);
    app.add_systems(Update, _on_update);
    app.run();
}

fn _on_ready(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
){
    commands.spawn(Camera2dBundle{camera_2d:Camera2d { clear_color: ClearColorConfig::Custom(Color::BLACK), }, transform: Transform::from_scale(Vec3::splat(0.5)), ..default()});
    let mut mesh = Mesh::new(PrimitiveTopology::LineList);
    let vec: Vec<[f32; 3]> = vec![[30., 0., 0.], [-30., 30., 0.], [-30., -30., 0.], [0., 0., 0.]];
    let ind: Vec<u32> = vec![0, 1, 1, 2, 2, 0, 3, 0];
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec.clone());
    //mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::WHITE.as_rgba_f32(); vec.len()]);
    mesh.set_indices(Some(Indices::U32(ind.clone())));

    /*commands.spawn(
        MaterialMesh2dBundle { //MESH
            mesh: Mesh2dHandle(meshes.add(mesh)),
            material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
            ..default()
        }
    );*/

}

fn _on_update(
    keys: Res<Input<KeyCode>>,
    mut commands: Commands,
    asteroids: Query<Entity, With<Asteroid>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
){
    if keys.just_pressed(KeyCode::Space){
        for asteroid in asteroids.iter(){
            commands.entity(asteroid).despawn_recursive();
        };
        for x in -2..3{
            for y in -2..3{
                let mut mesh = Mesh::new(PrimitiveTopology::LineList);  
                let seed = rand::random::<u64>();
                let size = get_asteroid_size(seed);
                let (vec, ind) = generate_asteroid_vertices_v2(seed);
                mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec.clone());
                //mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, vec![Color::WHITE.as_rgba_f32(); vec.len()]);
                mesh.set_indices(Some(Indices::U32(ind.clone())));

                
                //let (vertices, indices) = prepate_for_polyline(vec, ind);
                
                commands.spawn((
                    Asteroid,
                    MaterialMesh2dBundle { //MESH
                        mesh: Mesh2dHandle(meshes.add(mesh)),
                        material: materials.add(ColorMaterial::default()), //ColorMaterial::from(texture_handle)
                        ..default()
                    }
                )).insert(Transform::from_translation(Vec3 { x: x as f32, y: y as f32, z: 0. } * 50.));
            }
        }
    }
}







pub fn generate_asteroid_vertices_v2(seed: u64) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let size = get_asteroid_size(seed);
    let dots = 4 + size; // number of dots in cloud
    let mut vec = Vec::new();
    let mut ind = Vec::new();

    let max_dist = size * 3;
    let min_dist = size;

    let mut prev_dist = 0;
    let max_step_dist = 0;

    for i in 0..dots{
        let angle = (i as f32 / dots as f32) * PI * 2. + (rand::random::<f32>() * PI / dots as f32);

        let dist = MIN_DIST * size as f32 + rand::random::<f32>() * MAX_DIST * size as f32;

        let vector = Vec2::from_angle(angle) * dist;
        let point = [vector.x, vector.y, 0.];
        //point[2] = 0.0;
        vec.push(point);
        ind.push(i as u32);
        ind.push(((i + 1) % dots) as u32);
    }

    vec.push([0., 0., 0.,]);
    ind.push(dots as u32 + 1);
    ind.push(rng.gen_range(0..(dots as u32)));
    ind.push(dots as u32 + 1);
    ind.push(rng.gen_range(0..(dots as u32)));
    ind.push(dots as u32 + 1);
    ind.push(rng.gen_range(0..(dots as u32)));
    ind.push(rng.gen_range(0..(dots as u32)));
    ind.push(rng.gen_range(0..(dots as u32)));
    (vec, ind)
}


pub fn prepate_for_polyline(vec: Vec<[f32; 3]>, ind: Vec<u32>) -> (Vec<Vect>, Vec<[u32; 2]>) {
    let mut vertices: Vec<Vect> = Vec::new();
    let mut indexes = Vec::new();
    for i in 0..ind.len(){
        if i * 2 + 1 >= ind.len() {break}

        indexes.push([ind[i*2], ind[i*2 + 1]])
    }
    for v in vec.iter(){
        vertices.push(Vect::from((v[0], v[1])));
    }
    
    (vertices, indexes)
}


pub fn get_asteroid_size(seed: u64) -> i8{
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
     match rng.gen_range(0..16) {
        0..=6 => 1,
        7..=14 => 2,
        15..=16 => 3,
        e => {println!("{}", e); 1}
    }
}


pub fn generate_asteroid_vertices(seed: u64) -> (Vec<[f32; 3]>, Vec<u32>) {
    let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let size = get_asteroid_size(seed);
    let sides = 16; // number of verteces
    let mut vec = Vec::new();
    let mut ind = Vec::new();
    for side in 0..sides{
        let mut point: [f32; 3];
        match side {
            0..=2 => point = [-1.0 + side as f32, -3.0, 0.0],
            3 => point = [2.0, -2.0, 0.0],
            4..=6 => point = [3.0, -1.0 + (side as f32 - 4.), 0.0],
            7 => point = [2.0, 2.0, 0.0],
            8..=10 => point = [1.0 - (side as f32 - 8.), 3.0, 0.0],
            11 => point = [-2.0, 2.0, 0.0],
            12..=14 => point = [-3.0, 1.0 - (side as f32 - 12.), 0.0],
            15 => point = [-2.0, -2.0, 0.0],
            _ => point = [-10.0, -10.0, -10.0]
        };
        //                            pre-mul            rand             *    max off   | * size 
        point = point.map(|v| (v * 2.0  + rng.gen_range(-100..101) as f32 * 0.01 ) * size as f32);  
        point[2] = 0.0;
        vec.push(point);
        ind.push(side);
        ind.push((side + 1) % sides); // for loop
    }
    (vec, ind)
}

const MIN_DIST: f32 = 3.;
const MAX_DIST: f32 = 5.;
pub fn generate_asteroid_vertices_v1(seed: u64) -> (Vec<[f32; 3]>, Vec<u32>) {
    //let mut rng = ChaCha8Rng::seed_from_u64(seed);
    let size = get_asteroid_size(seed);
    let dots = 8 * (size + 1); // number of dots in cloud
    let mut vec = Vec::new();
    let mut ind = Vec::new();
    for i in 0..dots{
        let angle = (i as f32 / dots as f32)  * PI * 2. + rand::random::<f32>() * 0.1;// ;
        let dist = MIN_DIST * size as f32 + rand::random::<f32>() * MAX_DIST * size as f32;
        let vector = Vec2::from_angle(angle) * dist;
        let point = [vector.x, vector.y, 0.];
        //point[2] = 0.0;
        vec.push(point);
        ind.push(i as u32);
        ind.push(((i + 1) % dots) as u32);
    }
    (vec, ind)
}
