use bevy::{prelude::{Resource, Color}, core_pipeline::{bloom::BloomCompositeMode, tonemapping::{Tonemapping, DebandDither}}};

#[derive (Resource)]
pub struct GameSettings{
    pub font_path: String,

    // SOUND
    pub volume: f32,
    pub music_volume: f32,
    pub effects_volume: f32,

    // MENU BEAMS
    pub beams_lifetime: f32,
    pub beams_perspective_factor: f32,
    pub beams_spawn_radius: f32,
    pub beams_speed: f32,
    pub beams_number: u32,


    // GRAPHICS SETTINGS
    pub deband_dither: DebandDither,
    pub tonemapping: Tonemapping,
    pub bloom_intensity: f32,
    pub composite_mode: BloomCompositeMode,
    pub low_frequency_boost: f32,
    pub low_frequency_boost_curvature: f32,
    pub high_pass_frequency: f32,
    pub threshold: f32,
    pub threshold_softness: f32,

    // GAME BG

    // GAME
    pub name: String,
    pub color:  [f32; 3],
    pub color2: [f32; 3],
    pub color3: [f32; 3],
    pub style: u8,



}
impl Default for GameSettings {
    fn default() -> GameSettings{
        GameSettings{
            font_path: "fonts/F77MinecraftRegular-0VYv.ttf".into(),
            // SOUND
            volume: 100., // ADD IMPLEMENTATION!!!
            music_volume: 100., // ADD IMPLEMENTATION!!!
            effects_volume: 100., // ADD IMPLEMENTATION!!!
            // MENU BEAMS
            beams_lifetime: 100., // -> beams 

            beams_number: 1000,
            beams_spawn_radius: 4000.,
            beams_perspective_factor: 0.13,
            beams_speed: 0.4,

            // todo ADD MSAA SAMPLING SETTING!
            // GRAPHICS VFX SETTINGS 
            bloom_intensity: 0.1, 
            deband_dither: DebandDither::Enabled,
            tonemapping: Tonemapping::TonyMcMapface,
            composite_mode: BloomCompositeMode::Additive, 
            low_frequency_boost: 0.1, 
            low_frequency_boost_curvature: 0.1, 
            high_pass_frequency: 1., 
            threshold: 0.1, 
            threshold_softness: 0.1, 

            // GAME
            name: "CoolName".into(),
            color:  [1.; 3],
            color2: [1.; 3],
            color3: [1.; 3],
            style: 0,
        }
    }
}


impl GameSettings{
    pub fn get_font_path(&self) -> String{
        self.font_path.clone()
    }
}
















