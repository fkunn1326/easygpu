use figures::{Point, Rect, Size};
use wgpu::TextureAspect;

use crate::binding::Bind;
use crate::buffers::Framebuffer;
use crate::canvas::Canvas;
use crate::color::Rgba8;
use crate::device::Device;

#[derive(Debug)]
pub struct Texture {
    pub wgpu: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub extent: wgpu::Extent3d,
    pub format: wgpu::TextureFormat,

    pub size: Size<u32>,
}

impl Texture {
    pub fn clear<T>(
        texture: &Texture,
        value: T,
        device: &mut Device,
        encoder: &mut wgpu::CommandEncoder,
    ) where
        T: Clone,
    {
        let capacity = texture.size.area() as usize;
        let mut texels: Vec<T> = Vec::with_capacity(capacity);
        texels.resize(capacity, value);

        let (head, body, tail) = unsafe { texels.align_to::<Rgba8>() };
        assert!(head.is_empty());
        assert!(tail.is_empty());

        Self::fill(texture, body, device, encoder);
    }

    pub fn fill<T: bytemuck::Pod + 'static>(
        texture: &Texture,
        texels: &[T],
        device: &mut Device,
        encoder: &mut wgpu::CommandEncoder,
    ) where
        T: Clone + Copy,
    {
        assert!(
            texels.len() as u32 >= texture.size.area(),
            "fatal: incorrect length for texel buffer"
        );

        let buf = device.create_buffer_from_slice(texels, wgpu::BufferUsages::COPY_SRC);

        Self::copy(
            &texture.wgpu,
            Rect::new(Point::default(), texture.size),
            texels.len() as u32 / texture.extent.height * 4,
            texture.extent,
            &buf,
            encoder,
        );
    }

    pub fn transfer<T: bytemuck::Pod + 'static>(
        texture: &Texture,
        texels: &[T],
        rect: Rect<i32>,
        device: &mut Device,
        encoder: &mut wgpu::CommandEncoder,
    ) where
        T: Into<Rgba8> + Clone + Copy,
    {
        // Wgpu's coordinate system has a downwards pointing Y axis.
        let destination = rect;
        // Make sure we have a positive rectangle
        let destination = Rect::from_extents(
            Point::new(
            destination.origin.x.min(destination.extent().x),
            destination.origin.y.min(destination.extent().y),
            ),
            Point::new(
                destination.extent().x.max(destination.origin.x) - destination.origin.x.min(destination.extent().x),
                destination.extent().y.max(destination.origin.y) - destination.origin.y.min(destination.extent().y),
            )
        );
        // flip y, making it negative in the y direction
        let rect = Rect::from_extents(
            Point::new(destination.origin.x, destination.extent().y),
            Point::new(destination.extent().x, destination.origin.y),
        );

        // The width and height of the transfer area.
        let destination_size = Size::new(rect.size.width as u32, rect.size.height as u32);

        // The destination coordinate of the transfer, on the texture.
        // We have to invert the Y coordinate as explained above.
        let destination_point = Point::new(
            rect.origin.x as u32,
            texture.size.height - rect.origin.y as u32,
        );

        assert!(
            destination_size.area() <= texture.size.area(),
            "fatal: transfer size must be <= texture size"
        );

        let buf = device.create_buffer_from_slice(texels, wgpu::BufferUsages::COPY_SRC);

        let extent = wgpu::Extent3d {
            width: destination_size.width,
            height: destination_size.height,
            depth_or_array_layers: 1,
        };
        Self::copy(
            &texture.wgpu,
            Rect::new(destination_point, destination_size.cast()),
            texels.len() as u32 / destination_size.height * 4,
            extent,
            &buf,
            encoder,
        );
    }

    fn blit(
        &self,
        src: Rect<u32>,
        dst: Rect<u32>,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        assert!(
            src.size.area() != dst.size.area(),
            "source and destination rectangles must be of the same size"
        );

        encoder.copy_texture_to_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.wgpu,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: src.origin.x,
                    y: src.origin.y,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            wgpu::TexelCopyTextureInfo {
                texture: &self.wgpu,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: dst.origin.x,
                    y: dst.origin.y,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            wgpu::Extent3d {
                width: src.size.width,
                height: src.size.height,
                depth_or_array_layers: 1,
            },
        );
    }

    fn copy(
        texture: &wgpu::Texture,
        destination: Rect<u32>,
        bytes_per_row: u32,
        extent: wgpu::Extent3d,
        buffer: &wgpu::Buffer,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        encoder.copy_buffer_to_texture(
            wgpu::TexelCopyBufferInfo {
                buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(bytes_per_row),
                    rows_per_image: Some(destination.size.height),
                },
            },
            wgpu::TexelCopyTextureInfo {
                texture,
                mip_level: 0,
                origin: wgpu::Origin3d {
                    x: destination.origin.x,
                    y: destination.origin.y,
                    z: 0,
                },
                aspect: TextureAspect::All,
            },
            extent,
        );
    }
}

impl Bind for Texture {
    fn binding(&self, index: u32) -> wgpu::BindGroupEntry {
        wgpu::BindGroupEntry {
            binding: index,
            resource: wgpu::BindingResource::TextureView(&self.view),
        }
    }
}

impl Canvas for Texture {
    type Color = Rgba8;

    fn fill(&self, buf: &[Rgba8], device: &mut Device, encoder: &mut wgpu::CommandEncoder) {
        Texture::fill(self, buf, device, encoder);
    }

    fn clear(&self, color: Rgba8, device: &mut Device, encoder: &mut wgpu::CommandEncoder) {
        Texture::clear(self, color, device, encoder);
    }

    fn transfer(
        &self,
        buf: &[Rgba8],
        rect: Rect<i32>,
        device: &mut Device,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        Texture::transfer(self, buf, rect, device, encoder);
    }

    fn blit(
        &self,
        src: Rect<u32>,
        dst: Rect<u32>,
        encoder: &mut wgpu::CommandEncoder,
    ) {
        Texture::blit(self, src, dst, encoder);
    }
}

impl From<Framebuffer> for Texture {
    fn from(fb: Framebuffer) -> Self {
        fb.texture
    }
}
