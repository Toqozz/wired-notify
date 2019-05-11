use glutin::WindowedContext;
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

use gfx_glyph::{ OwnedVariedSection, OwnedSectionText, GlyphBrushBuilder, Scale };

use crate::bus;

// Window factory.
use std::collections::HashMap;
use crate::window;

// draw_rectangle
use lyon::math::rect;
use lyon::tessellation::{ VertexBuffers, FillOptions, FillVertex };
use lyon::tessellation::basic_shapes::*;
use lyon::tessellation::geometry_builder::VertexConstructor;
use lyon::tessellation::BuffersBuilder;


pub struct WindowFactory {
    pub window_map: HashMap<glutin::WindowId, window::GLWindow>,

    //pub events_loop: glutin::EventsLoop,
}

impl WindowFactory {
    pub fn new() -> (WindowFactory, glutin::EventsLoop) {
        let window_map = HashMap::new();

        // Events loop to caputer window events (clicked, moved, resized, etc).
        let events_loop = glutin::EventsLoop::new();

        (WindowFactory {
            window_map,
        }, events_loop)
    }

    pub fn create_window(&mut self, events_loop: &glutin::EventsLoop) -> &mut GLWindow {
        let window = GLWindow::new(events_loop);

        let id = window.windowed_context.window().id();
        self.window_map.insert(id, window);

        // TODO: keep window reference instead of fetching.
        self.window_map.get_mut(&id)
            .expect("Failed to create window.")
    }
}

pub struct GLWindow {
    pub windowed_context: WindowedContext,
    pub device: GLDevice,
    pub factory: Factory,
    pub encoder: Encoder<Resources, CommandBuffer>,
    pub pso: gfx::PipelineState<Resources, pipe::Meta>,
    pub color_view: RenderTargetView<Resources, ColorFormat>,
    pub depth_view: DepthStencilView<Resources, DepthFormat>,

    glyph_brush: gfx_glyph::GlyphBrush<'static, Resources, Factory>,
    // TODO: section probably shouldn't be here.
    section: Option<OwnedVariedSection>,

    pub data: Option<pipe::Data<Resources>>,
    pub slice: Option<Slice<Resources>>,

    pub notification: Option<bus::dbus::Notification>,
}

impl GLWindow {
    pub fn new(events_loop: &glutin::EventsLoop) -> GLWindow {
        // Initialize a window and context but don't build them yet.
        let window_builder = glutin::WindowBuilder::new()
            .with_dimensions(LogicalSize { width: 300.0, height: 23.0 })
            .with_title("wiry")
            .with_class("wiry".to_owned(), "wiry".to_owned())
            .with_transparency(true)
            .with_always_on_top(true)
            .with_x11_window_type(glutin::os::unix::XWindowType::Utility);
        let context_builder = glutin::ContextBuilder::new()
            .with_vsync(true);

        // Build the window using the glutin backend for gfx-rs.
        // window -- obvious, device -- rendering device, factory -- creation?, color_view -- base
        // color, depth_view -- ?
        let (windowed_context, device, mut factory, color_view, depth_view) =
            gfx_window_glutin::init::<ColorFormat, DepthFormat>(window_builder, context_builder, events_loop)
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

        let arial: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/arial.ttf"));
        let glyph_brush = GlyphBrushBuilder::using_font_bytes(arial)
            .depth_test(gfx::preset::depth::LESS_EQUAL_WRITE)
            .build(factory.clone());

        let ortho = nalgebra::Orthographic3::new(0.0, 300.0, 0.0, 23.0, -1.0, 1.0);

        // Create vertex buffer and slice from supplied vertices.
        // A slice dictates what and in what order vertices are processed.
        let vertex = Vertex {
            position: [0.0, 0.0, 0.0],
            texcoords: [0.0, 0.0],
            normal: [0.0, 0.0],
            color: [1.0, 1.0, 1.0, 1.0],
        };

        let (vertex_buffer, _slice) = factory.create_vertex_buffer_with_slice(&[vertex; 4], ());
        let data = pipe::Data {
            vbuf: vertex_buffer,
            //font: (glyph_brush.into, sampler),
            proj: ortho.to_homogeneous().into(),
            out: color_view.clone(),
            out_depth: depth_view.clone(),
        };

        GLWindow {
            windowed_context,
            device,
            factory,
            color_view,
            depth_view,
            encoder,
            pso,
            glyph_brush,
            section: None,
            data: Some(data),
            slice: None,
            notification: None,
        }
    }

    pub fn set_notification(&mut self, notification: bus::dbus::Notification) {
        self.set_text(notification.summary.clone());
        self.notification = Some(notification);
    }

    pub fn set_slice(&mut self, slice: Slice<Resources>) {
        self.slice = Some(slice);
    }

    pub fn set_text(&mut self, text: String) {
        let section = OwnedSectionText {
            text: text,
            scale: Scale::uniform(12.0),
            color: [1.0, 0.0, 0.0, 1.0],
            ..OwnedSectionText::default()
        };

        let varied_section = OwnedVariedSection {
            text: vec![section],
            screen_position: (10.0, 10.0),
            z: -1.0,
            ..OwnedVariedSection::default()
        };

        self.section = Some(varied_section);
    }

    // TODO: theres much better ways of doing this.  Also, add parameters.
    pub fn draw_rectangle(&mut self) {
        let mut geometry: VertexBuffers<Vertex, u16> = VertexBuffers::new();
        let options = FillOptions::tolerance(0.01);
        fill_rounded_rectangle(
            &rect(0.0, 0.0, 300.0, 23.0),
            &BorderRadii {
                top_left: 50.0,
                top_right: 50.0,
                bottom_left: 50.0,
                bottom_right: 50.0,
            },
            &options,
            &mut BuffersBuilder::new(&mut geometry, VertexCtor { color: [0.0, 1.0, 0.0, 1.0] }),
        ).expect("Could not build rectangle.");

        let (vertex_buffer, slice) = self.factory.create_vertex_buffer_with_slice(&geometry.vertices.as_slice(), geometry.indices.as_slice());

        self.data.as_mut().unwrap().vbuf = vertex_buffer;
        self.set_slice(slice);
    }

    pub fn draw(&mut self) {
        if let (Some(data), Some(slice)) = (&self.data, &self.slice) {
            self.encoder.clear(&data.out, BLACK);
            self.encoder.clear_depth(&data.out_depth, 1.0);

            //glyph_brush.queue(section);
            self.encoder.draw(slice, &self.pso, data);

            // Always draw text last because it's the most prone to fuzzing the depth test.
            if let Some(section) = &self.section {
                self.glyph_brush.queue(section.to_borrowed());
                self.glyph_brush.draw_queued(&mut self.encoder, &self.color_view, &self.depth_view)
                    .expect("Failed to draw font.");
            }

            self.encoder.flush(&mut self.device);

            self.windowed_context.swap_buffers().unwrap();
            self.device.cleanup();
        }
    }

    /*
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
    */
}
