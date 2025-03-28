use wgpu::ShaderStages;

/// A group of bindings.
#[derive(Debug)]
pub struct BindingGroup {
    pub wgpu: wgpu::BindGroup,
    /// The index of the binding group in the pipeline
    /// This matches the n index value of the corresponding @group(n) attribute in the shader code
    pub set_index: u32,
}

impl BindingGroup {
    pub fn new(set_index: u32, wgpu: wgpu::BindGroup) -> Self {
        Self { wgpu, set_index }
    }
}

/// The layout of a `BindingGroup`.
#[derive(Debug)]
pub struct BindingGroupLayout {
    pub wgpu: wgpu::BindGroupLayout,
    pub size: usize,
    pub set_index: u32,
}

impl BindingGroupLayout {
    pub fn new(set_index: u32, layout: wgpu::BindGroupLayout, size: usize) -> Self {
        Self {
            wgpu: layout,
            size,
            set_index,
        }
    }
}

/// A trait representing a resource that can be bound.
pub trait Bind {
    fn binding(&self, index: u32) -> wgpu::BindGroupEntry;
}

/// A binding type.
#[derive(Debug, Clone, Copy)]
pub enum BindingType {
    UniformBuffer,
    UniformBufferDynamic,
    Sampler,
    SampledTexture { multisampled: bool },
}

impl From<BindingType> for wgpu::BindingType {
    fn from(binding_type: BindingType) -> Self {
        match binding_type {
            BindingType::UniformBuffer => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            BindingType::UniformBufferDynamic => wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: true,
                min_binding_size: None,
            },
            BindingType::SampledTexture { multisampled } => wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                multisampled,
                view_dimension: wgpu::TextureViewDimension::D2,
            },
            BindingType::Sampler => wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        }
    }
}

#[derive(Debug)]
pub struct Binding {
    pub binding: BindingType,
    pub stage: ShaderStages,
}
