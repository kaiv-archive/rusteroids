use bevy::render::mesh::Indices;
use bevy::render::render_resource::PrimitiveTopology;
use bevy::sprite::{Mesh2dHandle, MaterialMesh2dBundle};
use bevy::{prelude::*, core_pipeline::clear_color::ClearColorConfig};
use bevy_egui::*;
#[path = "console.rs"] mod console;
#[path = "client_menu.rs"] mod client_menu;
use bevy_egui::egui::RawInput;
use bevy_inspector_egui::quick::WorldInspectorPlugin;
use console::*;
#[path = "game.rs"] mod game;
use game::*;
use game::components::*;

pub fn main(){
    let mut app = App::new();
    app.add_plugins((DefaultPlugins.set(
        ImagePlugin::default_nearest()
        ),
        EguiPlugin,
        WorldInspectorPlugin::new()
    ));
    app.add_state::<ClientState>();
    setup_commands_executer(&mut app, true);
    app.add_systems(Startup, setup);

    app.add_systems(Update, (
        client_menu::esc_menu, 
        game::update_powerups_animation, 
        client_menu::tab_menu,
    ) /*(console_renderer, command_executer)*/);
    app.insert_resource(GameSettings::default());
    game::init_pixel_camera(&mut app);
    app.run();
}



fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut images: ResMut<Assets<Image>>,
){
    spawn_powerup( PowerUPType::Repair, Vec3::X * -80., &mut commands,&mut meshes, &mut materials, &asset_server);
    spawn_powerup( PowerUPType::DoubleDamage, Vec3::X * -40., &mut commands,&mut meshes, &mut materials, &asset_server);
    spawn_powerup( PowerUPType::Haste, Vec3::X * 0., &mut commands,&mut meshes, &mut materials, &asset_server);
    spawn_powerup( PowerUPType::SuperShield, Vec3::X * 40., &mut commands,&mut meshes, &mut materials, &asset_server);
    spawn_powerup( PowerUPType::Invisibility, Vec3::X * 80., &mut commands,&mut meshes, &mut materials, &asset_server);
}

