struct Render {
	mvp: mat4x4<f32>,
};

@group(0) @binding(0) var<uniform> render: Render;

struct VSOut {
    @builtin(position) pos: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs(
	@builtin(vertex_index) index: u32,
	@location(0) in_pos: vec4<f32>,
) -> VSOut {
    var vs_out: VSOut;
    vs_out.pos = render.mvp * in_pos;
    vs_out.color = vec4(1, 0, 0, 1);
    return vs_out;
}

@fragment
fn fs(@location(0) inColor: vec4<f32>) -> @location(0) vec4<f32> {
    return inColor;
}
