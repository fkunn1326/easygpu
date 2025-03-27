use std::ops::Range;
use std::sync::{Arc, Mutex};

use figures::{Size, Rect};
use wgpu::{FilterMode, MultisampleState, TextureAspect, TextureFormat, TextureViewDescriptor};

use crate::binding::{Bind, BindingGroup, BindingGroupLayout};
use crate::blending::Blending;
use crate::buffers::{DepthBuffer, Framebuffer, IndexBuffer, UniformBuffer, VertexBuffer};
use crate::canvas::Canvas;
use crate::color::{Bgra8, Rgba};
use crate::device::{Device, DeviceBuilder};
use crate::frame::Frame;
use crate::pipeline::AbstractPipeline;
use crate::sampler::Sampler;
use crate::texture::Texture;
use crate::vertex::VertexLayout;

pub trait Draw {
    fn draw<'a>(&'a self, binding: &'a BindingGroup, pass: &mut wgpu::RenderPass<'a>);
}


pub struct RendererBuilder<'a> {
    surface: Option<wgpu::Surface<'a>>,
    instance: Option<wgpu::Instance>,
    adapter: Option<wgpu::Adapter>,
    sample_count: u32,
    offscreen: bool,
}

impl<'a> RendererBuilder<'a> {
    pub fn new() -> Self {
        Self {
            surface: None,
            instance: None,
            adapter: None,
            sample_count: 0,
            offscreen: false,
        }
    }

    pub fn with_surface(mut self, surface: wgpu::Surface<'a>, instance: wgpu::Instance, sample_count: u32) -> Self {
        self.surface = Some(surface);
        self.instance = Some(instance);
        self.sample_count = sample_count;
        self
    }

    pub fn with_offscreen(mut self, offscreen: bool, adapter: wgpu::Adapter, sample_count: u32) -> Self {
        self.offscreen = offscreen;
        self.adapter = Some(adapter);
        self.sample_count = sample_count;
        self
    }

    pub async fn build(self) -> Result<Renderer<'a>, wgpu::RequestDeviceError> {
        if self.offscreen {
            let adapter = self.adapter.unwrap();
            let device = DeviceBuilder::new(adapter).build().await?;

            Ok(Renderer { device, sample_count: self.sample_count })
        } else {
            let instance = self.instance.unwrap();
            let surface = self.surface.unwrap();
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    power_preference: wgpu::PowerPreference::default(),
                    force_fallback_adapter: false,
                })
                .await
                .unwrap();

            let device = DeviceBuilder::new(adapter)
                .with_surface(surface)
                .build()
                .await?;
            Ok(Renderer { device, sample_count: self.sample_count })
        }
    }
}

#[derive(Debug)]
pub struct Renderer<'a> {
    pub device: Device<'a>,
    /// Enables MSAA for values > 1.
    pub(crate) sample_count: u32,
}

impl<'a> Renderer<'a> {
    pub const fn sample_count(&self) -> u32 {
        self.sample_count
    }

    pub fn configure<PresentMode: Into<wgpu::PresentMode>>(
        &mut self,
        size: Size<u32>,
        mode: PresentMode,
        format: TextureFormat,
    ) {
        self.device.configure(size, mode, format)
    }

    pub fn current_frame(&self) -> Result<RenderFrame, wgpu::SurfaceError> {
        let surface = self.device.surface.as_ref().unwrap();
        let surface_texture = surface.get_current_texture()?;
        let view = surface_texture
            .texture
            .create_view(&TextureViewDescriptor::default());
        Ok(RenderFrame {
            wgpu: Some(surface_texture),
            view,
            depth: self
                .device
                .create_zbuffer(self.device.size(), self.sample_count),
            size: self.device.size(),
        })
    }

    pub fn texture(
        &self,
        size: Size<u32>,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        multisampled: bool,
    ) -> Texture {
        let sample_count = if multisampled { self.sample_count } else { 1 };
        self.device
            .create_texture(size, format, usage, sample_count)
    }

    pub fn framebuffer(
        &self,
        size: Size<u32>,
        format: wgpu::TextureFormat,
    ) -> Framebuffer {
        self.device
            .create_framebuffer(size, format, self.sample_count)
    }

    pub fn zbuffer(&self, size: Size<u32>) -> DepthBuffer {
        self.device.create_zbuffer(size, self.sample_count)
    }

    pub fn vertex_buffer<T: bytemuck::Pod>(&self, verts: &[T]) -> VertexBuffer
    where
        T: 'static + Copy,
    {
        self.device.create_buffer(verts)
    }

    pub fn uniform_buffer<T>(&self, buf: &[T]) -> UniformBuffer
    where
        T: bytemuck::Pod + 'static + Copy,
    {
        self.device.create_uniform_buffer(buf)
    }

    pub fn binding_group(&self, layout: &BindingGroupLayout, binds: &[&dyn Bind]) -> BindingGroup {
        self.device.create_binding_group(layout, binds)
    }

    pub fn sampler(&self, min_filter: FilterMode, mag_filter: FilterMode) -> Sampler {
        self.device.create_sampler(min_filter, mag_filter)
    }

    pub fn pipeline<T>(&self, blending: Blending, format: TextureFormat) -> T
    where
        T: AbstractPipeline<'static>,
    {
        let desc = T::description();
        let pip_layout = self.device.create_pipeline_layout(desc.pipeline_layout);
        let vertex_layout = VertexLayout::from(desc.vertex_layout);
        let shader = self.device.create_shader(desc.shader);

        T::setup(
            self.device.create_pipeline(
                pip_layout,
                vertex_layout,
                blending,
                &shader,
                format,
                MultisampleState {
                    count: self.sample_count,
                    mask: !0,
                    alpha_to_coverage_enabled: false,
                },
            ),
            &self.device,
        )
    }

    pub fn read<F>(&mut self, fb: &Framebuffer, f: F) -> Result<(), wgpu::BufferAsyncError>
    where
        F: 'static + FnOnce(&[Bgra8]),
    {
        let mut encoder = self.device.create_command_encoder();

        let bytesize = 4 * fb.size();
        let gpu_buffer = self.device.wgpu.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: bytesize as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &fb.texture.wgpu,
                mip_level: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
                aspect: TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &gpu_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    // TODO: Must be a multiple of 256
                    bytes_per_row: Some(4 * fb.texture.size.width),
                    rows_per_image: Some(fb.texture.size.height),
                },
            },
            fb.texture.extent,
        );
        // let submission_index = self.device.submit(vec![encoder.finish()]);

        let mut buffer: Vec<u8> = Vec::with_capacity(bytesize);

        let dst = gpu_buffer.slice(0..bytesize as u64);
        let result = Arc::new(Mutex::new(None));
        let callback_result = result.clone();
        dst.map_async(wgpu::MapMode::Read, move |map_result| {
            let mut result = callback_result.lock().unwrap();
            *result = Some(map_result);
        });

        // let mut queue_empty = self
        //     .device
        //     .wgpu
        //     .poll(wgpu::MaintainBase::WaitForSubmissionIndex(submission_index));
        loop {
            let result = result.lock().unwrap().take();
            match result {
                Some(Ok(())) => break,
                Some(Err(err)) => return Err(err),
                None => {
                    // We didn't get our map callback, but the submission is done.
                    // We'll keep polling the device until we get our map callback.
                    // queue_empty = self.device.wgpu.poll(wgpu::MaintainBase::Poll);
                }
            }
        }

        let view = dst.get_mapped_range();
        buffer.extend_from_slice(&view);
        if buffer.len() == bytesize {
            let (head, body, tail) = unsafe { buffer.align_to::<Bgra8>() };
            if !(head.is_empty() && tail.is_empty()) {
                panic!("Renderer::read: framebuffer is not a valid Bgra8 buffer");
            }
            f(body);
        }

        gpu_buffer.unmap();

        Ok(())
    }

    pub fn update_pipeline<'b, T>(&mut self, pip: &'b T, p: T::PrepareContext)
    where
        T: AbstractPipeline<'b>,
    {
        if let Some((buffer, uniforms)) = pip.prepare(p) {
            self.device
                .update_uniform_buffer::<T::Uniforms>(uniforms.as_slice(), buffer);
        }
    }

    pub fn frame(&mut self) -> Frame {
        let encoder = self.device.create_command_encoder();
        Frame::new(encoder)
    }

    pub fn present(&mut self, frame: Frame) {
        self.device.submit(vec![frame.encoder.finish()]);
    }

    pub fn submit<T: Copy>(&mut self, commands: &[Op<T>]) {
        let mut encoder = self.device.create_command_encoder();
        for c in commands.iter() {
            c.encode(&mut self.device, &mut encoder);
        }
        self.device.submit(vec![encoder.finish()]);
    }
}

pub enum Op<'a, T> {
    Clear(&'a dyn Canvas<Color = T>, T),
    Fill(&'a dyn Canvas<Color = T>, &'a [T]),
    Transfer {
        f: &'a dyn Canvas<Color = T>,
        buf: &'a [T],
        rect: Rect<i32>,
    },
    Blit(
        &'a dyn Canvas<Color = T>,
        Rect<u32>,
        Rect<u32>,
    ),
}

impl<'a, T> Op<'a, T>
where
    T: Copy,
{
    fn encode(&self, dev: &mut Device, encoder: &mut wgpu::CommandEncoder) {
        match *self {
            Op::Clear(f, color) => {
                f.clear(color, dev, encoder);
            }
            Op::Fill(f, buf) => {
                f.fill(buf, dev, encoder);
            }
            Op::Transfer { f, buf, rect } => {
                f.transfer(buf, rect, dev, encoder);
            }
            Op::Blit(f, src, dst) => {
                f.blit(src, dst, encoder);
            }
        }
    }
}

pub trait RenderPassExt<'a> {
    fn begin(
        encoder: &'a mut wgpu::CommandEncoder,
        view: &'a wgpu::TextureView,
        resolve_target: Option<&'a wgpu::TextureView>,
        depth: &'a wgpu::TextureView,
        op: PassOp,
    ) -> Self;

    fn set_easy_pipeline<'b, T>(&mut self, pipeline: &'a T)
    where
        T: AbstractPipeline<'b>;

    fn set_binding(&mut self, group: &'a BindingGroup, offsets: &[u32]);

    fn set_easy_index_buffer(&mut self, index_buf: &'a IndexBuffer);
    fn set_easy_vertex_buffer(&mut self, vertex_buf: &'a VertexBuffer);
    fn easy_draw<T: Draw>(&mut self, drawable: &'a T, binding: &'a BindingGroup);
    fn draw_buffer(&mut self, buf: &'a VertexBuffer);
    fn draw_buffer_range(&mut self, buf: &'a VertexBuffer, range: Range<u32>);
    fn draw_indexed(&mut self, indices: Range<u32>, instances: Range<u32>);
}

impl<'a> RenderPassExt<'a> for wgpu::RenderPass<'a> {
    fn begin(
        encoder: &'a mut wgpu::CommandEncoder,
        view: &'a wgpu::TextureView,
        resolve_target: Option<&'a wgpu::TextureView>,
        depth: &'a wgpu::TextureView,
        op: PassOp,
    ) -> Self {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target,
                ops: wgpu::Operations {
                    load: op.to_wgpu(),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(0),
                    store: wgpu::StoreOp::Store,
                }),
            }),
            occlusion_query_set: None,
            timestamp_writes: None,
        })
    }

    fn set_easy_pipeline<'b, T>(&mut self, pipeline: &'a T)
    where
        T: AbstractPipeline<'b>,
    {
        self.set_pipeline(&pipeline.pipeline.wgpu);
        self.set_binding(&pipeline.bindings, &[]);
    }

    fn set_binding(&mut self, group: &'a BindingGroup, offsets: &[u32]) {
        self.set_bind_group(group.set_index, &group.wgpu, offsets);
    }

    fn set_easy_index_buffer(&mut self, index_buf: &'a IndexBuffer) {
        self.set_index_buffer(index_buf.slice(), wgpu::IndexFormat::Uint16)
    }

    fn set_easy_vertex_buffer(&mut self, vertex_buf: &'a VertexBuffer) {
        self.set_vertex_buffer(0, vertex_buf.slice())
    }

    fn easy_draw<T: Draw>(&mut self, drawable: &'a T, binding: &'a BindingGroup) {
        drawable.draw(binding, self);
    }

    fn draw_buffer(&mut self, buf: &'a VertexBuffer) {
        self.set_easy_vertex_buffer(buf);
        self.draw(0..buf.size, 0..1);
    }

    fn draw_buffer_range(&mut self, buf: &'a VertexBuffer, range: Range<u32>) {
        self.set_easy_vertex_buffer(buf);
        self.draw(range, 0..1);
    }

    fn draw_indexed(&mut self, indices: Range<u32>, instances: Range<u32>) {
        self.draw_indexed(indices, 0, instances)
    }
}

#[derive(Debug)]
pub enum PassOp {
    Clear(Rgba),
    Load(),
}

impl PassOp {
    fn to_wgpu(&self) -> wgpu::LoadOp<wgpu::Color> {
        match self {
            PassOp::Clear(color) => wgpu::LoadOp::Clear((*color).into()),
            PassOp::Load() => wgpu::LoadOp::Load,
        }
    }
}

/// Can be rendered to in a pass.
pub trait RenderTarget {
    /// Color component.
    fn color_target(&self) -> &wgpu::TextureView;
    /// Depth component.
    fn zdepth_target(&self) -> &wgpu::TextureView;
}

pub struct RenderFrame {
    pub view: wgpu::TextureView,
    pub wgpu: Option<wgpu::SurfaceTexture>,
    pub depth: DepthBuffer,
    pub size: Size<u32>,
}

impl RenderTarget for RenderFrame {
    fn color_target(&self) -> &wgpu::TextureView {
        &self.view
    }

    fn zdepth_target(&self) -> &wgpu::TextureView {
        &self.depth.texture.view
    }
}

impl Drop for RenderFrame {
    fn drop(&mut self) {
        if let Some(wgpu) = self.wgpu.take() {
            wgpu.present();
        }
    }
}
