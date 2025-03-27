// 共通のuniform構造体
struct Globals {
    ortho: mat4x4<f32>,
    transform: mat4x4<f32>,
};

// バインディングの設定
@group(0) @binding(0) var<uniform> global: Globals;

// 頂点シェーダーの入力
struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec4<f32>,
};

// 頂点シェーダーの出力（フラグメントシェーダーへの入力）
struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

// sRGBからリニア空間への変換関数
fn linearize(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(0.04045);
    let higher = pow((srgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    let lower = srgb / vec3<f32>(12.92);

    return select(higher, lower, cutoff);
}

// 頂点シェーダーのエントリーポイント
@vertex
fn vs_main(input: VertexInput) -> VertexOutput {
    var output: VertexOutput;
    output.color = vec4<f32>(linearize(input.color.rgb), input.color.a);
    output.position = global.ortho * global.transform * vec4<f32>(input.position, 1.0);
    return output;
}

// フラグメントシェーダーのエントリーポイント
@fragment
fn fs_main(input: VertexOutput) -> @location(0) vec4<f32> {
    return input.color;
}