use wgpu::ShaderStage;

/// A group of bindings.
#[derive(Debug)]
pub struct BindingGroup {
    pub wgpu: wgpu::BindGroup,
    pub set_index: u32,
}

impl BindingGroup {
    pub fn new(set_index: u32, wgpu: wgpu::BindGroup) -> Self {
        Self { set_index, wgpu }
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
#[derive(Debug)]
pub enum BindingType {
    UniformBuffer,
    UniformBufferDynamic,
    Sampler,
    SampledTexture,
}

impl BindingType {
    pub fn to_wgpu(&self) -> wgpu::BindingType {
        match self {
            BindingType::UniformBuffer => wgpu::BindingType::UniformBuffer {
                dynamic: false,
                min_binding_size: None,
            },
            BindingType::UniformBufferDynamic => wgpu::BindingType::UniformBuffer {
                dynamic: true,
                min_binding_size: None,
            },
            BindingType::SampledTexture => wgpu::BindingType::SampledTexture {
                multisampled: false,
                dimension: wgpu::TextureViewDimension::D2,
                component_type: wgpu::TextureComponentType::Float,
            },
            BindingType::Sampler => wgpu::BindingType::Sampler { comparison: true },
        }
    }
}

#[derive(Debug)]
pub struct Binding {
    pub binding: BindingType,
    pub stage: ShaderStage,
}
