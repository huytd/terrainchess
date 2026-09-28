#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_bindings::{material, base_color_texture, base_color_sampler},
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    let tex_color = textureSample(base_color_texture, base_color_sampler, in.uv);
    if tex_color.a < 0.1 {
        discard;
    }
    var out: FragmentOutput;
    out.color = vec4<f32>(material.base_color.rgb, material.base_color.a * tex_color.a);
    return out;
}
