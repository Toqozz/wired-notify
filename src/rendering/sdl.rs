use sdl2::{
    Sdl,
    VideoSubsystem,
    video::GLProfile,
    ttf::Sdl2TtfContext,
};

pub struct SDL2State {
    pub context: Sdl,
    pub video_subsys: VideoSubsystem,
    pub ttf_context: Sdl2TtfContext,
    //pub event_pump: EventPump,
}

impl SDL2State {
    pub fn new() -> Result<SDL2State, String> {
        let context = sdl2::init()?;
        let video_subsys = context.video()?;
        let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;

        let gl_attr = video_subsys.gl_attr();
        gl_attr.set_context_profile(GLProfile::Core);
        gl_attr.set_context_version(3, 3);

        let state = Self {
            context,
            video_subsys,
            ttf_context,
        };

        Ok(state)
    }
}

