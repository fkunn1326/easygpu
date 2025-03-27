use figures::Size;
use wgpu::util::DeviceExt;
use wgpu::{
    CompositeAlphaMode, FilterMode, MultisampleState, SubmissionIndex, TextureFormat, TextureUsages,
};

use crate::binding::{Bind, Binding, BindingGroup, BindingGroupLayout};
use crate::blending::Blending;
use crate::buffers::{DepthBuffer, Framebuffer, IndexBuffer, UniformBuffer, VertexBuffer};
use crate::pipeline::{Pipeline, PipelineLayout, Set};
use crate::sampler::Sampler;
use crate::shader::Shader;
use crate::texture::Texture;
use crate::vertex::VertexLayout;

pub struct DeviceBuilder<'a> {
    adapter: wgpu::Adapter,
    surface: Option<wgpu::Surface<'a>>,
}

impl<'a> DeviceBuilder<'a> {
    pub fn new(adapter: wgpu::Adapter) -> Self {
        Self {
            adapter,
            surface: None,
        }
    }

    pub fn with_surface(mut self, surface: wgpu::Surface<'a>) -> DeviceBuilder<'a> {
        self.surface = Some(surface);
        self
    }

    pub async fn build(self) -> Result<Device<'a>, wgpu::RequestDeviceError> {
        let (device, queue) = self
            .adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .await?;

        Ok(Device {
            wgpu: device,
            queue,
            surface: self.surface,
            size: Size::default(), 
        })
    }
}

#[derive(Debug)]
pub struct Device<'a> {
    pub wgpu: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: Option<wgpu::Surface<'a>>,
    size: Size<u32>,
}


impl Device<'_> {
    pub const fn device(&self) -> &wgpu::Device {
        &self.wgpu
    }

    pub const fn size(&self) -> Size<u32> {
        self.size
    }

    pub fn device_mut(&mut self) -> &mut wgpu::Device {
        &mut self.wgpu
    }

    pub fn create_command_encoder(&self) -> wgpu::CommandEncoder {
        self.wgpu
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None })
    }

    pub fn configure<PM: Into<wgpu::PresentMode>>(
        &mut self,
        size: Size<u32>,
        mode: PM,
        format: TextureFormat,
    ) {
        let desc = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            present_mode: mode.into(),
            width: size.width,
            height: size.height,
            alpha_mode: CompositeAlphaMode::Auto,
            view_formats: Vec::new(),
            desired_maximum_frame_latency: 2,
        };

        self.surface
            .as_ref()
            .expect("create_swap_chain only works when initalized with a wgpu::Surface")
            .configure(&self.wgpu, &desc);
        self.size = size;
    }

    pub fn create_pipeline_layout(&self, ss: &[Set]) -> PipelineLayout {
        let mut sets = Vec::new();
        for (i, s) in ss.iter().enumerate() {
            sets.push(self.create_binding_group_layout(i as u32, s.0));
        }
        PipelineLayout { sets }
    }

    pub fn create_shader(&self, source: &str) -> Shader {
        Shader {
            wgpu: self
                .wgpu
                .create_shader_module(wgpu::ShaderModuleDescriptor {
                    source: wgpu::ShaderSource::Wgsl(source.into()),
                    label: None, // TODO labels would be nice
                }),
        }
    }

    pub fn create_texture(
        &self,
        size: Size<u32>,
        format: TextureFormat,
        usage: TextureUsages,
        sample_count: u32,
    ) -> Texture {
        let texture_extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let texture = self.wgpu.create_texture(&wgpu::TextureDescriptor {
            size: texture_extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            label: None,
            view_formats: &[],
        });
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Texture {
            wgpu: texture,
            view: texture_view,
            extent: texture_extent,
            format,
            size,
        }
    }

    pub fn create_framebuffer(
        &self,
        size: Size<u32>,
        format: TextureFormat,
        sample_count: u32,
    ) -> Framebuffer {
        let extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let texture = self.wgpu.create_texture(&wgpu::TextureDescriptor {
            size: extent,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_DST
                | TextureUsages::COPY_SRC
                | TextureUsages::RENDER_ATTACHMENT,
            label: None,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Framebuffer {
            texture: Texture {
                wgpu: texture,
                view,
                extent,
                format,
                size,
            },
            depth: self.create_zbuffer(size, sample_count),
        }
    }

    pub fn create_zbuffer(&self, size: Size<u32>, sample_count: u32) -> DepthBuffer {
        let format = DepthBuffer::FORMAT;
        let extent = wgpu::Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        };
        let wgpu = self.wgpu.create_texture(&wgpu::TextureDescriptor {
            size: extent,
            label: None,
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: TextureUsages::COPY_DST | TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        let view = wgpu.create_view(&wgpu::TextureViewDescriptor::default());

        DepthBuffer {
            texture: Texture {
                wgpu,
                view,
                extent,
                format,
                size,
            },
        }
    }

    pub fn create_binding_group(
        &self,
        layout: &BindingGroupLayout,
        binds: &[&dyn Bind],
    ) -> BindingGroup {
        assert_eq!(
            binds.len(),
            layout.size,
            "layout slot count does not match bindings"
        );

        let mut bindings = Vec::new();

        for (i, b) in binds.iter().enumerate() {
            bindings.push(b.binding(i as u32));
        }

        BindingGroup::new(
            layout.set_index,
            self.wgpu.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &layout.wgpu,
                label: None,
                entries: bindings.as_slice(),
            }),
        )
    }

    pub fn create_buffer<T: bytemuck::Pod>(&self, vertices: &[T]) -> VertexBuffer
    where
        T: 'static + Copy,
    {
        VertexBuffer {
            wgpu: self.create_buffer_from_slice(vertices, wgpu::BufferUsages::VERTEX),
            size: (vertices.len() * std::mem::size_of::<T>()) as u32,
        }
    }

    pub fn create_uniform_buffer<T>(&self, buf: &[T]) -> UniformBuffer
    where
        T: bytemuck::Pod + 'static + Copy,
    {
        UniformBuffer {
            size: std::mem::size_of::<T>(),
            count: buf.len(),
            wgpu: self
                .wgpu
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Uniform Buffer"),
                    contents: bytemuck::cast_slice(buf),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                }),
        }
    }

    pub fn create_index(&self, indices: &[u16]) -> IndexBuffer {
        let index_buf = self.create_buffer_from_slice(indices, wgpu::BufferUsages::INDEX);
        IndexBuffer {
            wgpu: index_buf,
            elements: indices.len() as u32,
        }
    }

    pub fn create_sampler(&self, min_filter: FilterMode, mag_filter: FilterMode) -> Sampler {
        Sampler {
            wgpu: self.wgpu.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter,
                min_filter,
                mipmap_filter: wgpu::FilterMode::Nearest,
                lod_min_clamp: 0.,
                lod_max_clamp: 100.0,
                compare: None,
                anisotropy_clamp: 1,
                label: None,
                border_color: None,
            }),
        }
    }

    pub fn create_binding_group_layout(&self, index: u32, slots: &[Binding]) -> BindingGroupLayout {
        let mut bindings = Vec::new();

        for s in slots {
            bindings.push(wgpu::BindGroupLayoutEntry {
                binding: bindings.len() as u32,
                visibility: s.stage,
                ty: s.binding.into(),
                count: None,
            });
        }
        let layout = self
            .wgpu
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: None,
                entries: bindings.as_slice(),
            });
        BindingGroupLayout::new(index, layout, bindings.len())
    }

    pub fn create_buffer_from_slice<T: bytemuck::Pod>(
        &self,
        slice: &[T],
        usage: wgpu::BufferUsages,
    ) -> wgpu::Buffer {
        self.wgpu
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: None,
                contents: bytemuck::cast_slice(slice),
                usage,
            })
    }

    pub fn update_uniform_buffer<T: bytemuck::Pod + Copy + 'static>(
        &self,
        slice: &[T],
        buf: &UniformBuffer,
    ) {
        self.queue
            .write_buffer(&buf.wgpu, 0, bytemuck::cast_slice(slice));
    }

    pub fn submit<I: IntoIterator<Item = wgpu::CommandBuffer>>(
        &mut self,
        cmds: I,
    ) -> SubmissionIndex {
        self.queue.submit(cmds)
    }

    // TODO clippy::too_many_arguments
    #[allow(clippy::too_many_arguments)]
    pub fn create_pipeline(
        &self,
        pipeline_layout: PipelineLayout,
        vertex_layout: VertexLayout,
        blending: Blending,
        shader: &Shader,
        swapchain_format: TextureFormat,
        multisample: MultisampleState,
    ) -> Pipeline {
        let vertex_attrs = (&vertex_layout).into();

        let mut sets = Vec::new();
        for s in pipeline_layout.sets.iter() {
            sets.push(&s.wgpu);
        }
        let layout = &self
            .wgpu
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: sets.as_slice(),
                push_constant_ranges: &[],
            });

        let (src_factor, dst_factor, operation) = blending.as_wgpu();

        let wgpu = self
            .wgpu
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: None,
                layout: Some(layout),
                vertex: wgpu::VertexState {
                    module: &shader.wgpu,
                    entry_point: Some("vs_main"),
                    buffers: &[vertex_attrs],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                },
                primitive: wgpu::PrimitiveState {
                    topology: wgpu::PrimitiveTopology::TriangleList,
                    strip_index_format: None,
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: None,
                    polygon_mode: wgpu::PolygonMode::Fill,
                    unclipped_depth: false,
                    conservative: false,
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: DepthBuffer::FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: wgpu::StencilState {
                        front: wgpu::StencilFaceState::IGNORE,
                        back: wgpu::StencilFaceState::IGNORE,
                        read_mask: 0,
                        write_mask: 0,
                    },
                    bias: wgpu::DepthBiasState {
                        constant: 0,
                        slope_scale: 0.,
                        clamp: 0.,
                    },
                }),
                multisample,
                multiview: None,
                fragment: Some(wgpu::FragmentState {
                    module: &shader.wgpu,
                    entry_point: Some("fs_main"),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: swapchain_format,
                        blend: Some(wgpu::BlendState {
                            color: wgpu::BlendComponent {
                                src_factor,
                                dst_factor,
                                operation,
                            },
                            alpha: wgpu::BlendComponent {
                                src_factor,
                                dst_factor,
                                operation,
                            },
                        }),

                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                }),
                cache: None,
            });

        Pipeline {
            layout: pipeline_layout,
            vertex_layout,
            wgpu,
        }
    }
}
