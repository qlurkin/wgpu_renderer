use std::{collections::HashMap, sync::Arc};

use crate::GpuContext;

pub trait Vertex: Copy + Clone + std::fmt::Debug + bytemuck::Pod + bytemuck::Zeroable {
    fn layout() -> VertexLayout;
}

#[derive(Hash, Eq, PartialEq, Clone, Debug)]
pub struct VertexLayout {
    pub array_stride: wgpu::BufferAddress,
    pub step_mode: wgpu::VertexStepMode,
    pub attributes: Vec<wgpu::VertexAttribute>,
}

impl VertexLayout {
    pub fn new(
        array_stride: wgpu::BufferAddress,
        step_mode: wgpu::VertexStepMode,
        attributes: Vec<wgpu::VertexAttribute>,
    ) -> Self {
        Self {
            array_stride,
            step_mode,
            attributes,
        }
    }

    pub fn as_wgpu(&self) -> wgpu::VertexBufferLayout<'_> {
        wgpu::VertexBufferLayout {
            array_stride: self.array_stride,
            step_mode: self.step_mode,
            attributes: &self.attributes,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SurfaceKey {
    pub vertex_layout: VertexLayout,
    pub shader: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    pub format: wgpu::TextureFormat,
    pub surface: SurfaceKey,
}

pub struct Renderer {
    gpu_context: Arc<GpuContext>,
    render_pipeline: HashMap<PipelineKey, wgpu::RenderPipeline>,
    vertex_buffer: wgpu::Buffer,
    vertex_layout: Option<VertexLayout>,
    vertices: Option<HashMap<SurfaceKey, Vec<u8>>>,
}

impl Renderer {
    pub fn new(gpu_context: &Arc<GpuContext>) -> Self {
        let vertex_buffer = gpu_context.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Renderer Vertex Buffer"),
            size: 3 * 2 * 3 * 4,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            gpu_context: Arc::clone(gpu_context),
            render_pipeline: HashMap::new(),
            vertex_buffer,
            vertex_layout: None,
            vertices: None,
        }
    }

    fn create_pipeline(
        &mut self,
        format: wgpu::TextureFormat,
        vertex_layout: VertexLayout,
        material_shader: &str,
    ) -> wgpu::RenderPipeline {
        let shader = self
            .gpu_context
            .device
            .create_shader_module(wgpu::include_wgsl!("shader.wgsl"));
        // TODO: Adapt the shader to accpet a vertex buffer

        let render_pipeline_layout =
            self.gpu_context
                .device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("Render Pipeline Layout"),
                    bind_group_layouts: &[],
                    immediate_size: 0,
                });

        self.gpu_context
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("Render Pipeline"),
                layout: Some(&render_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("vs_main"),
                    buffers: &[vertex_layout.as_wgpu()],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: Some(wgpu::BlendState::REPLACE),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: Some(wgpu::Face::Back),
                    // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                    polygon_mode: wgpu::PolygonMode::Fill,
                    // Requires Features::DEPTH_CLIP_CONTROL
                    unclipped_depth: false,
                    // Requires Features::CONSERVATIVE_RASTERIZATION
                    conservative: false,
                },
                depth_stencil: None,
                multisample: wgpu::MultisampleState {
                    count: 1,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
                multiview_mask: None,
                cache: None,
            })
    }

    pub fn begin_frame(&mut self) {
        self.vertices = Some(HashMap::new());
    }

    pub fn triangle<V: Vertex>(&mut self, v1: V, v2: V, v3: V, shader: &str) {
        let vertex_layout = V::layout();
        let surface_key = SurfaceKey {
            vertex_layout,
            shader: String::from(shader),
        };
        // self.vertex_layout = Some(vertex_layout);
        // TODO: Fill the vertex buffer

        // let vertices = &[v1, v2, v3];

        let vertices = self
            .vertices
            .as_mut()
            .unwrap()
            .entry(surface_key)
            .or_insert(Vec::new());

        vertices.extend_from_slice(bytemuck::bytes_of(&[v1, v2, v3]));

        // self.gpu_context.queue.write_buffer(
        //     &self.vertex_buffer,
        //     0, // offset en bytes
        //     bytemuck::cast_slice(vertices),
        // );
    }

    pub fn end_frame(&mut self, texture: &wgpu::Texture) {
        // let vertex_layout = self.vertex_layout.take().unwrap();

        let format = texture.format();

        // if self.render_pipeline.is_none() {
        //     self.render_pipeline = Some(self.create_pipeline(texture.format(), vertex_layout, ""));
        // }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder =
            self.gpu_context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            // render pass to initialize frame buffer
            let _render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }

        self.gpu_context
            .queue
            .submit(std::iter::once(encoder.finish()));

        let vertices = self.vertices.take().unwrap();

        for (surface, verts) in vertices.iter() {
            let pipeline_key = PipelineKey {
                surface: surface.clone(),
                format,
            };

            if !self.render_pipeline.contains_key(&pipeline_key) {
                let pipeline =
                    self.create_pipeline(format, surface.vertex_layout.clone(), &surface.shader);
                self.render_pipeline.insert(pipeline_key.clone(), pipeline);
            }

            let pipeline = self.render_pipeline.get(&pipeline_key).unwrap();

            self.gpu_context.queue.write_buffer(
                &self.vertex_buffer,
                0, // offset en bytes
                verts,
            );

            let mut encoder =
                self.gpu_context
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("Render Encoder"),
                    });

            {
                let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Render Pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &view,
                        resolve_target: None,
                        depth_slice: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color {
                                r: 0.1,
                                g: 0.2,
                                b: 0.3,
                                a: 1.0,
                            }),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    occlusion_query_set: None,
                    timestamp_writes: None,
                    multiview_mask: None,
                });

                render_pass.set_pipeline(pipeline);
                render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
                render_pass.draw(0..3, 0..1);
            }

            // submit will accept anything that implements IntoIter
            self.gpu_context
                .queue
                .submit(std::iter::once(encoder.finish()));
        }
    }
}
