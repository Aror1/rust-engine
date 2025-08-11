
use env_logger::fmt::style::RgbColor;
#[cfg(target_arch="wasm32")]
use wgpu::naga::back::spv::SourceLanguage;
use wgpu::naga::back::INDENT;
use wgpu::wgc::device::{self, queue};
use wgpu::wgt::bytemuck_wrapper;
use wgpu::Buffer;
use wgpu::{wgc::instance, Surface};
use wgpu::util::DeviceExt;
use winit::dpi::Pixel;
use std::default;
use std::io::Cursor;
use std::{any, io::SeekFrom, ops::Not, sync::Arc};
extern crate rand;
use rand::{random_range, Rng};


use winit::{
    application::ApplicationHandler, event::{Event, KeyEvent, WindowEvent}, event_loop::{self, ActiveEventLoop, ControlFlow, EventLoop}, keyboard::{KeyCode, PhysicalKey}, window::{self, Window, WindowAttributes, WindowId}
};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;


// All code comments and tips were taken from the site: https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/#render
// Все комментарии кода и подсказки были взяты с сайта: https://sotrh.github.io/learn-wgpu/beginner/tutorial2-surface/#render



const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.0868241, 0.49240386, 0.0], color: [0.1, 0.0, 0.3] }, // A
    Vertex { position: [-0.49513406, 0.06958647, 0.0], color: [0.5, 0.0, 0.2] }, // B
    Vertex { position: [-0.21918549, -0.44939706, 0.0], color: [0.5, 0.0, 0.6] }, // C
    Vertex { position: [0.35966998, -0.3473291, 0.0], color: [0.3, 0.0, 0.9] }, // D
    Vertex { position: [0.44147372, 0.2347359, 0.0], color: [0.4, 0.0, 0.1] }, // E
];


const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];
 

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 3],
}


impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[
            wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x3,
            },
            wgpu::VertexAttribute {
                offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                shader_location: 1,
                format: wgpu::VertexFormat::Float32x3,
            }
        ]
    }
    }    
}

// State struct which init in app fn::new
pub struct State 
{
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    is_surface_configured: bool,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    window: Arc<Window>,
    color: wgpu::Color,
    index_buffer: wgpu::Buffer,
    num_indices: u32,
    index_or_vertices: bool
}

impl State {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {

        let size = window.inner_size();

        // The instance is a handle to our GPU
        // BackendBit::PRIMARY => Vulkan + Metal + DX12 + Browser WebGPU

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor { 
            backends: wgpu::Backends::PRIMARY,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();

        // adapter give Surface and Adapter

        // adapter can give information about gpu


        // RequestAdapterOptions: 
        // power_preference has two variants: LowPower and HighPerformance. LowPower will pick an adapter that favors battery life, such as an integrated GPU. HighPerformance will pick an adapter for more power-hungry yet more performant GPU's, such as a dedicated graphics card. WGPU will favor LowPower if there is no adapter for the HighPerformance option.
        // The compatible_surface field tells wgpu to find an adapter that can present to the supplied surface.
        // The force_fallback_adapter forces wgpu to pick an adapter that will work on all hardware. This usually means that the rendering backend will use a "software" system instead of hardware such as a GPU.

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptionsBase {
            power_preference: wgpu::PowerPreference::default(),
            force_fallback_adapter: false,
            compatible_surface: (Some(&surface)), 
        })
        .await?;

        // Device
        let (device, queue) = adapter.request_device(&wgpu::wgt::DeviceDescriptor {
            label: None,
            required_features: wgpu::Features::empty(),
            required_limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
            memory_hints: Default::default(),
            trace: wgpu::Trace::Off, 
        })
        .await?; 
        
        // surface capabilities
        // surface_caps: Результат - структура wgpu::SurfaceCapabilities, которая содержит несколько векторов (Vec) с допустимыми значениями:
        //     formats: Vec<TextureFormat>: Список форматов текстур (цветовых форматов), которые можно использовать с этой surface и adapter.
        //     present_modes: Vec<PresentMode>: Список режимов представления (как кадры передаются из буфера wgpu в окно).
        //     alpha_modes: Vec<CompositeAlphaMode>: Список режимов альфа-композиции (как обрабатывается прозрачность окна).

        let surface_caps = surface.get_capabilities(&adapter);

        let surface_format = surface_caps.formats.iter()
        .find(|f| f.is_srgb())
        .copied()
        .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::wgt::SurfaceConfiguration { 
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                    format: surface_format,
                    width: size.width,     //  Make sure that the width and height of the SurfaceTexture are not 0, as that can cause your app to crash.
                    height: size.height,    
                    present_mode: surface_caps.present_modes[0],  // режим представления.
                    alpha_mode: surface_caps.alpha_modes[0],  // режим альфа-композиции
                    view_formats: vec![],
                    desired_maximum_frame_latency: 2 // максимальная латентность
                };
        
        let color = wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        };
        
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()) 
        });

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor { 
            label: Some("render pipeline layout"),
            bind_group_layouts: &[], 
            push_constant_ranges: &[]    
        });

        let render_pipeline = device.create_render_pipeline(
          &wgpu::RenderPipelineDescriptor {
            label: Some("render pipeline"), 
            layout: Some(&render_pipeline_layout), 
            vertex: wgpu::VertexState { 
                module: &shader, 
                entry_point: Some("vs_main"), 
                buffers: &[
                    // SHADER BUFFER
                    Vertex::desc(), 
                ], 
                compilation_options: wgpu::PipelineCompilationOptions::default(), 
            }, 
            fragment: Some(wgpu::FragmentState { 
                module: &shader, 
                entry_point: Some("fs_main"), 
                targets: &[Some(wgpu::ColorTargetState { 
                    format: config.format, 
                    blend: Some(wgpu::BlendState::REPLACE), 
                    write_mask: wgpu::ColorWrites::ALL, 
                })], 
                compilation_options: wgpu::PipelineCompilationOptions::default(), 
            }), 

            // Поле primitive описывает, как интерпретировать наши вершины при преобразовании их в треугольники.
            primitive: wgpu::PrimitiveState { 
                topology: wgpu::PrimitiveTopology::TriangleList, 
                strip_index_format: None, 
                front_face: wgpu::FrontFace::Ccw, 
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE 
                polygon_mode: wgpu::PolygonMode::Fill, 
                unclipped_depth: false, 
                conservative: false, 
            }, 

            depth_stencil: None, 
            multisample: wgpu::MultisampleState { 
                count: 1, 
                mask: !0,  // !=0
                alpha_to_coverage_enabled: false }, 
            multiview: None, 
            cache: None,
        });


        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("vertex buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let num_vertices = VERTICES.len() as u32;

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("index buffer"),
                contents: bytemuck::cast_slice(INDICES),
                usage: wgpu::BufferUsages::INDEX,
            }
        );  // координаты передаются индексом что занимает меньше памяти

        let num_indices = INDICES.len() as u32;  
        
        let mut index_or_vertices = false;



        // SELF
        Ok(Self
            {
                surface,
                device,
                queue,
                config,
                is_surface_configured: false,
                render_pipeline,
                vertex_buffer,
                num_vertices,
                window,
                color,
                index_buffer, 
                num_indices,
                index_or_vertices,
            })
        // SELF

    }
        

    pub fn resize(&mut self, _width: u32, _height: u32) {
        if _width > 0 && _height > 0 {
        self.config.width = _width;
        self.config.height = _height;
        self.surface.configure(&self.device, &self.config);
        self.is_surface_configured = true;
        }
    }
    

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {

        self.window.request_redraw();
        
        if !self.is_surface_configured {
            return Ok(());
        }

        // Функция get_current_texture будет ждать, пока , surface не предоставят новый объект SurfaceTexture, который будет использоваться для рендеринга. Мы сохраним его outputдля дальнейшего использования.
        let output = self.surface.get_current_texture()?;
        
        let view = output.texture.create_view(&wgpu::wgt::TextureViewDescriptor::default());

        // CommandEncoder для формирования команд для отправки на gpu
        let mut encoder = self.device.create_command_encoder(&wgpu::wgt::CommandEncoderDescriptor { label: Some("render encoder") });

        // Нам нужно использовать , encoder чтобы создать RenderPass. В RenderPass содержатся все методы для отрисовки.
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor { 
                label: Some("Some render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    depth_slice: None,
                    view: &view,
                    resolve_target: None, // resolve_target — это текстура, которая получит финальное (разрешённое) изображение
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(self.color),
                        // Поле store указывает, хотим ли мы сохранить результаты рендеринга в текстуру, лежащую за TextureView (в данном случае — в SurfaceTexture). Мы используем StoreOp::Store, поскольку хотим сохранить результаты рендеринга.
                        store: wgpu::StoreOp::Store, 
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None 
                });

            render_pass.set_pipeline(&self.render_pipeline);

            // SET VERTEX BUFFER TO RENDER PASS
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            

            // SET INDEX BUFFER 
            render_pass.set_index_buffer(self.index_buffer.slice(..), wgpu::IndexFormat::Uint16);

            // render_pass.draw_indexed(0..self.num_indices, 0, 0..1);
            
            
            // CHANGE INDEX OR VERTICES DRAW
            if self.index_or_vertices {
                render_pass.draw(0..self.num_vertices, 0..1);
            }
            else {
                render_pass.draw_indexed(0..self.num_indices, 0, 0..1);

            }

            // RENDER PASS DRAW
            // render_pass.draw(0..self.num_vertices, 0..1);
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    pub fn update(&mut self) {

        // later
    }


    
}

#[derive(Default)]

struct App {
    #[cfg(target_arch = "wasm32")]
    proxy: Option<winit::event_loop::EventLoopProxy<State>>,
    state: Option<State>,
}

impl App {
    pub fn new(#[cfg(target_arch="wasm32")] event_loop: &EventLoop<State>) -> Self {
        #[cfg(target_arch="wasm32")]
        let proxy = Some(
            event_loop.create_proxy()
        ); 

        Self {
            state: None,
            #[cfg(target_arch="wasm32")]
            proxy,
         }
    }
}

impl ApplicationHandler for App {

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: WindowEvent,
    ) {
        let wgpu_state = match &mut self.state {
            Some(canvas) => canvas,
            None => return,
        };

    match event {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(size) => wgpu_state.resize(size.width, size.height),
                WindowEvent::RedrawRequested => {
                    match wgpu_state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                            let size = wgpu_state.window.inner_size();
                            wgpu_state.resize(size.width, size.height);
                        }
                        Err(e) => {
                            log::error!("unable to render {}", e);
                        }
                    }
                }
              

                // TASK 
                // WindowEvent::CursorMoved { position, device_id } => 
                // {
                //     if position.x % 2.0 == 0.0 {
                //         state.color = wgpu::Color {
                //             r: 0.1,
                //             g: 0.2,
                //             b: 0.3,
                //             a: 1.0,
                //         };
                //     }
                //     else {

                //         let mut rng = rand::rng();

                //         let mut rand_c: f64 = rng.r#gen();

                //         state.color = wgpu::Color {
                //             r: rng.r#gen(),
                //             g: rng.r#gen(),
                //             b: rng.r#gen(),
                //             a: 1.0,
                //         };
                //     }
                // }
                // Modify the input() method to capture mouse events, and update the clear color using that. Hint: you'll probably need to use WindowEvent::CursorMoved. TASK 
                
                
                WindowEvent::KeyboardInput {
                    event:
                        KeyEvent {
                            physical_key: PhysicalKey::Code(code),
                            state,
                            ..
                        },
                    ..
                } => match (code, state.is_pressed()) {
                    (KeyCode::Escape, true) => event_loop.exit(),
                    (KeyCode::KeyV, true) => {
                        wgpu_state.index_or_vertices = !wgpu_state.index_or_vertices;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {   

        let mut window_attributes: WindowAttributes = Window::default_attributes();

         #[cfg(target_arch = "wasm32")]
        {
            use wasm_bindgen::JsCast;
            use winit::platform::web::WindowAttributesExtWebSys;
            
            const CANVAS_ID: &str = "canvas";

            let window = wgpu::web_sys::window().unwrap_throw();
            let document = window.document().unwrap_throw();
            let canvas = document.get_element_by_id(CANVAS_ID).unwrap_throw();
            let html_canvas_element = canvas.unchecked_into();
            window_attributes = window_attributes.with_canvas(Some(html_canvas_element));
        }   

        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

         #[cfg(not(target_arch = "wasm32"))] {

            // non webassembly, use pollster
            // await the 
            self.state = Some(pollster::block_on(State::new(window)).unwrap())
        }

        #[cfg(target_arch = "wasm32")]
        {
            // Run the future asynchronously and use the
            // proxy to send the results to the event loop
            if let Some(proxy) = self.proxy.take() {
                wasm_bindgen_futures::spawn_local(async move {
                    assert!(proxy
                        .send_event(
                            State::new(window)
                                .await
                                .expect("Unable to create canvas!!!")
                        )
                        .is_ok())
                });
            }
        }
    }
    
    // user event
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {    
        // This is where proxy.send_event() ends up
        #[cfg(target_arch="wasm32")] {
            event.window.request_redraw();
            event.resize(
                event.window.inner_size()._width,
                event.window.inner_size()._height
            );

            self.state = Some(event);   
        }
    }
}




fn main() -> anyhow::Result<()> {

    #[cfg(not(target_arch = "wasm32"))]
    {
        env_logger::init();
    }
    #[cfg(target_arch = "wasm32")]
    {
        console_log::init_with_level(log::Level::Info).unwrap_throw();
    }

    let event_loop = EventLoop::with_user_event().build()?;
    
    let mut app = App::new(
        #[cfg(target_arch = "wasm32")]
        &event_loop,
    );

    event_loop.run_app(&mut app)?;

    Ok(())
}

