use glutin::{WindowedContext, EventsLoop};
use glutin::os::unix::WindowBuilderExt;

use gfx;
use gfx::handle::{RenderTargetView, DepthStencilView};
use gfx::traits::FactoryExt;
use gfx::Device;                                                        // Must bring gfx::Device into scope to use trait methods.
use gfx::Slice;

use gfx::format::Srgba8 as ColorFormat;
use gfx::format::DepthStencil as DepthFormat;

use gfx::Encoder;
use gfx_device_gl::{Device as GLDevice, Factory, Resources, CommandBuffer};

gfx_defines! {
    vertex Vertex {
        position: [f32; 3] = "v_pos",
        texcoords: [f32; 2] = "v_texcoords",
        normal: [f32; 2] = "v_normal",
        color: [f32; 4] = "v_color",
    }

    pipeline pipe {
        vbuf: gfx::VertexBuffer<Vertex> = (),
        //font: gfx::TextureSampler<[f32; 4]> = "t_font",
        proj: gfx::Global<[[f32; 4]; 4]> = "u_proj",
        out: gfx::RenderTarget<gfx::format::Srgba8> = "out_color",
        out_depth: gfx::DepthTarget<gfx::format::DepthStencil> = gfx::preset::depth::LESS_EQUAL_WRITE,
    }
}

const BLACK: [f32; 4] = [0.0, 0.0, 0.0, 1.0];


pub struct GLWindow {
    pub context: WindowedContext,
    pub events_loop: EventsLoop,
    pub device: GLDevice,
    pub factory: Factory,
    pub render_target: RenderTargetView<Resources, ColorFormat>,
    pub depth_target: DepthStencilView<Resources, DepthFormat>,
    pub encoder: Encoder<Resources, CommandBuffer>,
    pub pso: gfx::PipelineState<Resources, pipe::Meta>,

    data: Option<pipe::Data<Resources>>,
    slice: Option<Slice<Resources>>,
}

impl GLWindow {
    pub fn build_window() -> GLWindow {
        // Events loop to caputer window events (clicked, moved, resized, etc).
        let events_loop = glutin::EventsLoop::new();

        // Initialize a window and context but don't build them yet.
        let window_builder = glutin::WindowBuilder::new()
            .with_title("yarn")
            .with_class("yarn2".to_owned(), "yarn2".to_owned())
            .with_transparency(false)
            .with_always_on_top(true)
            .with_x11_window_type(glutin::os::unix::XWindowType::Utility);
        let context_builder = glutin::ContextBuilder::new()
            .with_vsync(true);

        // Build the window using the glutin backend for gfx-rs.
        // window -- obvious, device -- rendering device, factory -- creation?, color_view -- base
        // color, depth_view -- ?
        let (window, device, mut factory, color_view, depth_view) =
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context_builder, &events_loop)
                .expect("Failed to create a window.");

        // Using an encoder avoids having to use raw OpenGL procedures.
        let encoder = factory.create_command_buffer().into();

        // To my understanding, pipeline state objects essentially batch shader commands.
        // TODO: better explanation.
        let pso = factory.create_pipeline_simple(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/base.glslv")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/base.glslf")),
            pipe::new()
        ).unwrap();

        GLWindow {
            context: window,
            events_loop,
            device,
            factory,
            render_target: color_view,
            depth_target: depth_view,
            encoder,
            pso,
            data: None,
            slice: None,
        }
    }

    pub fn set_data(&mut self, data: pipe::Data<Resources>) {
        self.data = Some(data);
    }

    pub fn set_slice(&mut self, slice: Slice<Resources>) {
        self.slice = Some(slice);
    }

    pub fn draw(&mut self) {
        if let (Some(data), Some(slice)) = (&self.data, &self.slice) {
            //let data = &self.data.clone().unwrap();
            //let slice = &self.slice.clone().unwrap();

            self.encoder.clear(&data.out, BLACK);
            self.encoder.clear_depth(&data.out_depth, 1.0);

            //glyph_brush.queue(section);
            self.encoder.draw(slice, &self.pso, data);

            // Always draw text last because it's the most prone to fuzzing the depth test.
            /*
            glyph_brush.draw_queued(
                &mut gl_window.encoder,
                &gl_window.render_target,
                &gl_window.depth_target).expect("FAIL");
                */
            self.encoder.flush(&mut self.device);


            self.context.swap_buffers().unwrap();
            self.device.cleanup();
        }
    }
}
