use std::ops::Deref;

use crate::{
    binding::{Binding, BindingGroup, BindingGroupLayout},
    buffers::UniformBuffer,
    device::Device,
    vertex::{VertexFormat, VertexLayout},
};

#[derive(Debug)]
pub struct Pipeline {
    pub wgpu: wgpu::RenderPipeline,

    pub layout: PipelineLayout,
    pub vertex_layout: VertexLayout,
}


#[derive(Debug)]
pub struct Set<'a>(pub &'a [Binding]);

#[derive(Debug)]
pub struct PipelineLayout {
    pub sets: Vec<BindingGroupLayout>,
}

pub struct PipelineCore {
    pub pipeline: Pipeline,
    pub bindings: BindingGroup,
    pub uniforms: UniformBuffer,
}

pub trait AbstractPipeline<'a>: Deref<Target = PipelineCore> {
    type PrepareContext;
    type Uniforms: bytemuck::Pod + Copy + 'static;

    fn description() -> PipelineDescription<'a>;
    fn setup(pip: Pipeline, dev: &Device) -> Self;
    fn prepare(
        &'a self,
        context: Self::PrepareContext,
    ) -> Option<(&'a UniformBuffer, Vec<Self::Uniforms>)>;
}

#[derive(Debug)]
pub struct PipelineDescription<'a> {
    pub vertex_layout: &'a [VertexFormat],
    pub pipeline_layout: &'a [Set<'a>],
    pub shader: &'static str,
}
