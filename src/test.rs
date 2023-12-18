use bevy::{prelude::*, core_pipeline::clear_color::ClearColorConfig};
use bevy_egui::*;
#[path = "console.rs"] mod console;
use console::*;

pub fn main(){
    let mut app = App::new();
    app.add_plugins(DefaultPlugins); 
    app.add_plugins(EguiPlugin);

    setup_commands_executer(&mut app, true);
    app.add_systems(Startup, setup);
    app.add_systems(Update, (console_renderer, command_executer));
    app.run();
}

fn setup(
    mut commands: Commands,
){
    commands.spawn(Camera2dBundle{
        camera_2d: Camera2d{ clear_color: ClearColorConfig::Custom(Color::WHITE) },
        ..default()
    });
}