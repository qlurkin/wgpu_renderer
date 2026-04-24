use std::sync::Arc;

use crate::GpuContext;

pub struct Renderer {
    gpu_context: Arc<GpuContext>,
    render_pipeline: Option<wgpu::RenderPipeline>,
}

impl Renderer {
    pub fn new(gpu_context: &Arc<GpuContext>) -> Self {
        Self {
            gpu_context: Arc::clone(gpu_context),
            render_pipeline: None,
        }
    }

    fn create_pipeline(&mut self, format: wgpu::TextureFormat) -> wgpu::RenderPipeline {
        let shader = self
            .gpu_context
            .device
            .create_shader_module(wgpu::include_wgsl!("shader.wgsl"));

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
                    buffers: &[],
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

    pub fn begin_frame(&self) {}

    pub fn end_frame(&mut self, texture: &wgpu::Texture) {
        if self.render_pipeline.is_none() {
            self.render_pipeline = Some(self.create_pipeline(texture.format()));
        }

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

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

            render_pass.set_pipeline(self.render_pipeline.as_ref().unwrap());
            render_pass.draw(0..3, 0..1);
        }

        // submit will accept anything that implements IntoIter
        self.gpu_context
            .queue
            .submit(std::iter::once(encoder.finish()));
    }
}
