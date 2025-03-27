#[derive(Debug, Clone, Copy)]
pub enum VertexFormat {
    Float,
    Floatx2,
    Floatx3,
    Floatx4,
    UBytex4,
}

impl VertexFormat {
    const fn bytesize(self) -> usize {
        match self {
            VertexFormat::Float => std::mem::size_of::<f32>() as usize,
            VertexFormat::Floatx2 => std::mem::size_of::<[f32; 2]>() as usize,
            VertexFormat::Floatx3 => std::mem::size_of::<[f32; 3]>() as usize,
            VertexFormat::Floatx4 => std::mem::size_of::<[f32; 4]>() as usize,
            VertexFormat::UBytex4 => std::mem::size_of::<u32>() as usize,
        }
    }
}
impl From<VertexFormat> for wgpu::VertexFormat {
    fn from(format: VertexFormat) -> Self {
        match format {
            VertexFormat::Float => wgpu::VertexFormat::Float32,
            VertexFormat::Floatx2 => wgpu::VertexFormat::Float32x2,
            VertexFormat::Floatx3 => wgpu::VertexFormat::Float32x3,
            VertexFormat::Floatx4 => wgpu::VertexFormat::Float32x4,
            VertexFormat::UBytex4 => wgpu::VertexFormat::Unorm8x4,
        }
    }
}

#[derive(Default, Debug)]
pub struct VertexLayout {
    attributes: Vec<wgpu::VertexAttribute>,
    size: usize,
}

impl VertexLayout {
    pub fn from(vertex_formats: &[VertexFormat]) -> Self {
        let mut layouts: Self = Self::default();
        for format in vertex_formats {
            layouts.attributes.push(wgpu::VertexAttribute {
                shader_location: layouts.attributes.len() as u32,
                offset: layouts.size as wgpu::BufferAddress,
                format: (*format).into(),
            });
            layouts.size += format.bytesize();
        }
        layouts
    }
}

impl<'a> From<&'a VertexLayout> for wgpu::VertexBufferLayout<'a> {
    fn from(layout: &'a VertexLayout) -> Self {
        wgpu::VertexBufferLayout {
            array_stride: layout.size as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: layout.attributes.as_slice(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct VertexBuffer {
    pub size: u64,
    pub buffer: wgpu::Buffer,
}
