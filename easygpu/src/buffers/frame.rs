use figures::Rect;

use crate::binding::Bind;
use crate::buffers::DepthBuffer;
use crate::canvas::Canvas;
use crate::color::Bgra8;
use crate::device::Device;
use crate::renderer::RenderTarget;
use crate::texture::Texture;
/// Off-screen framebuffer. Can be used as a render target in render passes.
#[derive(Debug)]
pub struct Framebuffer {
    pub texture: Texture,
    pub depth: DepthBuffer,
}

impl Framebuffer {
    /// Size in pixels of the framebuffer.
    pub fn size(&self) -> usize {
        self.texture.size.area() as usize
    }

    /// Framebuffer width, in pixels.
    pub fn width(&self) -> u32 {
        self.texture.size.width
    }

    /// Framebuffer height, in pixels.
    pub fn height(&self) -> u32 {
        self.texture.size.height
    }
}

impl RenderTarget for Framebuffer {
    fn color_target(&self) -> &wgpu::TextureView {
        &self.texture.view
    }

    fn zdepth_target(&self) -> &wgpu::TextureView {
        &self.depth.texture.view
    }
}

impl Bind for Framebuffer {
    fn binding(&self, index: u32) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: index,
            resource: wgpu::BindingResource::TextureView(&self.texture.view),
        }
    }
}

impl Canvas for Framebuffer {
    type Color = Bgra8;

    fn clear(&self, color: Self::Color, device: &mut Device, encoder: &mut wgpu::CommandEncoder) {
        Texture::clear(&self.texture, color, device, encoder);
        Texture::clear(&self.depth.texture, 0f32, device, encoder);
    }

    fn fill(&self, buf: &[Self::Color], device: &mut Device, encoder: &mut wgpu::CommandEncoder) {
        Texture::fill(&self.texture, buf, device, encoder);
    }

    fn transfer(
        &self,
        buf: &[Self::Color],
        rect: Rect<i32>,
        device: &mut Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        Texture::transfer(&self.texture, buf, rect, device, encoder);
    }

    fn blit(
        &self,
        from: Rect<u32>,
        dst: Rect<u32>,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        Texture::blit(&self.texture, from, dst, encoder);
    }
}
