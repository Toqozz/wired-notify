use glutin::{WindowedContext, EventsLoop};
use glutin::os::unix::WindowBuilderExt;
use glutin::dpi::LogicalSize;

use gfx;
use gfx::handle::{RenderTargetView, DepthStencilView};
use gfx::traits::FactoryExt;
use gfx::Device;                                                        // Must bring gfx::Device into scope to use trait methods.
use gfx::Slice;

use gfx::format::Srgba8 as ColorFormat;
use gfx::format::DepthStencil as DepthFormat;

use gfx::Encoder;
use gfx_device_gl::{Device as GLDevice, Factory, Resources, CommandBuffer};

use gfx_glyph::{ Section, GlyphBrushBuilder, Scale };

use glutin::WindowEvent::*;

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

pub struct GLWindow<'a> {
    pub context: WindowedContext,
    //pub events_loop: EventsLoop,
    pub device: GLDevice,
    pub factory: Factory,
    pub render_target: RenderTargetView<Resources, ColorFormat>,
    pub depth_target: DepthStencilView<Resources, DepthFormat>,
    pub encoder: Encoder<Resources, CommandBuffer>,
    pub pso: gfx::PipelineState<Resources, pipe::Meta>,

    glyph_brush: gfx_glyph::GlyphBrush<'static, Resources, Factory>,
    // TODO: section probably shouldn't be here.
    section: Option<Section<'a>>,

    pub data: Option<pipe::Data<Resources>>,
    slice: Option<Slice<Resources>>,

    pub running: bool,
}

impl<'a> GLWindow<'a> {
    pub fn build_window() -> (GLWindow<'a>, glutin::EventsLoop) {
        // Events loop to caputer window events (clicked, moved, resized, etc).
        let events_loop = glutin::EventsLoop::new();

        // Initialize a window and context but don't build them yet.
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(LogicalSize { width: 1280.0, height: 720.0 })
            .with_title("wiry")
            .with_class("wiry".to_owned(), "wiry".to_owned())
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

        // This may need to change with multiple windows/threads?
        //window.make_current();

        // Using an encoder avoids having to use raw OpenGL procedures.
        let encoder = factory.create_command_buffer().into();

        // To my understanding, pipeline state objects essentially batch shader commands.
        // TODO: better explanation.
        let pso = factory.create_pipeline_simple(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/base.glslv")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/shaders/base.glslf")),
            pipe::new()
        ).unwrap();

        let arial: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/arial.ttf"));
        let glyph_brush = GlyphBrushBuilder::using_font_bytes(arial)
            .depth_test(gfx::preset::depth::LESS_EQUAL_WRITE)
            .build(factory.clone());

        (GLWindow {
            context: window,
            device,
            factory,
            render_target: color_view,
            depth_target: depth_view,
            encoder,
            pso,
            glyph_brush,
            section: None,
            data: None,
            slice: None,
            running: true,
        }, events_loop)
    }

    pub fn set_slice(&mut self, slice: Slice<Resources>) {
        self.slice = Some(slice);
    }

    pub fn set_text(&mut self, text: &'a str) {
        let section = Section {
            text: text,
            screen_position: (10.0, 10.0),
            scale: Scale::uniform(32.0),
            color: [1.0, 0.0, 0.0, 1.0],
            z: -1.0,
            ..Section::default()
        };

        self.section = Some(section);
    }

    pub fn resize(&mut self, size: &glutin::dpi::LogicalSize) {
        gfx_window_glutin::update_views(
            &self.context,
            &mut self.render_target,
            &mut self.depth_target,
        );

        let physical_size = glutin::dpi::PhysicalSize::from_logical(size.clone(), 1.0);
        self.context.resize(physical_size);

        // TODO: messy.... and also somewhat incorrect.
        let ortho = nalgebra::Orthographic3::new(0.0, size.width as f32, 0.0, size.height as f32, -1.0, 1.0);
        let data = self.data.clone();
        if let Some(mut d) = data {
            d.proj = ortho.to_homogeneous().into();
            self.data = Some(d);
        }
    }

    pub fn draw(&mut self) {
        if let (Some(data), Some(slice)) = (&self.data, &self.slice) {
            self.encoder.clear(&data.out, BLACK);
            self.encoder.clear_depth(&data.out_depth, 1.0);

            //glyph_brush.queue(section);
            self.encoder.draw(slice, &self.pso, data);

            // Always draw text last because it's the most prone to fuzzing the depth test.
            self.glyph_brush.queue(self.section.unwrap());
            self.glyph_brush.draw_queued(&mut self.encoder, &self.render_target, &self.depth_target)
                .expect("Failed to draw font.");

            self.encoder.flush(&mut self.device);

            self.context.swap_buffers().unwrap();
            self.device.cleanup();
        }
    }
}
