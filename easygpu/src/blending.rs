#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Blending {
    src_factor: BlendFactor,
    dst_factor: BlendFactor,
    operation: BlendOp,
}

impl Blending {
    pub fn new(src_factor: BlendFactor, dst_factor: BlendFactor, operation: BlendOp) -> Self {
        Blending {
            src_factor,
            dst_factor,
            operation,
        }
    }

    pub fn constant() -> Self {
        Blending {
            src_factor: BlendFactor::One,
            dst_factor: BlendFactor::Zero,
            operation: BlendOp::Add,
        }
    }

    pub fn as_wgpu(&self) -> (wgpu::BlendFactor, wgpu::BlendFactor, wgpu::BlendOperation) {
        (
            self.src_factor.into(),
            self.dst_factor.into(),
            self.operation.into(),
        )
    }
}

impl From<Blending> for wgpu::BlendState {
    fn from(blending: Blending) -> Self {
        wgpu::BlendState {
            color: wgpu::BlendComponent {
                src_factor: blending.src_factor.into(),
                dst_factor: blending.dst_factor.into(),
                operation: blending.operation.into(),
            },
            alpha: wgpu::BlendComponent {
                src_factor: blending.src_factor.into(),
                dst_factor: blending.dst_factor.into(),
                operation: blending.operation.into(),
            },
        }
    }
}

impl Default for Blending {
    fn default() -> Self {
        Blending {
            src_factor: BlendFactor::SrcAlpha,
            dst_factor: BlendFactor::OneMinusSrcAlpha,
            operation: BlendOp::Add,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlendFactor {
    One,
    Zero,
    SrcAlpha,
    OneMinusSrcAlpha,
}

impl From<BlendFactor> for wgpu::BlendFactor {
    fn from(factor: BlendFactor) -> Self {
        match factor {
            BlendFactor::One => wgpu::BlendFactor::One,
            BlendFactor::Zero => wgpu::BlendFactor::Zero,
            BlendFactor::SrcAlpha => wgpu::BlendFactor::SrcAlpha,
            BlendFactor::OneMinusSrcAlpha => wgpu::BlendFactor::OneMinusSrcAlpha,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlendOp {
    Add,
}

impl From<BlendOp> for wgpu::BlendOperation {
    fn from(op: BlendOp) -> Self {
        match op {
            BlendOp::Add => wgpu::BlendOperation::Add,
        }
    }
}
