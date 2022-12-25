use math::prelude::*;
use std::mem;
use std::slice;

pub async fn compatible() -> &'static mut dyn Renderer {
    Box::leak(box webgpu::WebGpu::new().await.unwrap())
}

pub trait Renderer {
    async fn new() -> Result<Self, ()> where Self: Sized;
    fn render(&mut self, render: Render, batches: &[&Batch]);
    fn batch(&self, vertices: &[Vertex], indices: &[Index]) -> Batch;
}

fn get_play_canvas() -> Result<web_sys::HtmlCanvasElement, ()> {
    let window = web_sys::window().ok_or(())?;

    let document = window.document().ok_or(())?;

    let canvas = document.get_element_by_id("play").ok_or(())?;

    use wasm_bindgen::JsCast;

    let canvas = canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())?;

    Ok(canvas)
}

pub type Index = u16;

#[derive(Clone, Copy)]
pub struct Vertex {
    pub position: Vector<f32, 4>,
}

#[derive(Clone, Copy)]
pub struct Render {
    pub mvp: Matrix<f32, 4, 4>,
}


pub mod webgpu {
const WEBGPU_MAX_BUFFER_SIZE: usize = 65536;

use super::*;
        use js_sys::*;
        use web_sys::*;
use wasm_bindgen::prelude::*;
use math::prelude::*;
use std::cmp;
use std::mem;
use std::str;

#[wasm_bindgen]
pub struct WebGpu {
    resolution: (u32, u32),
    window: web_sys::Window,
    canvas: web_sys::HtmlCanvasElement,
    context: web_sys::GpuCanvasContext,
    queue: web_sys::GpuQueue,
    device: web_sys::GpuDevice,
    color_texture: web_sys::GpuTexture,
    color_texture_view: web_sys::GpuTextureView,
    depth_texture: web_sys::GpuTexture,
    depth_texture_view: web_sys::GpuTextureView,
    pipeline: web_sys::GpuRenderPipeline,
    bind_group: web_sys::GpuBindGroup,
    render_buffer: web_sys::GpuBuffer,
    staging_buffer: web_sys::GpuBuffer,
}

impl Renderer for WebGpu {
    async fn new() -> Result<Self, ()> {
        use js_sys::*;
        use web_sys::*;

        use wasm_bindgen_futures as wasm_futures;

        let window = window().ok_or(())?;

        let navigator = window.navigator();

        let gpu = navigator.gpu();

        let js_value = wasm_futures::JsFuture::from(gpu.request_adapter())
            .await
            .map_err(|_| ())?;

        let adapter = GpuAdapter::from(js_value);

        let js_value = wasm_futures::JsFuture::from(adapter.request_device())
            .await
            .map_err(|_| ())?;

        let device = GpuDevice::from(js_value);

        let queue = device.queue();

        let canvas = get_play_canvas()?;

        let resolution = (
            window.inner_width().unwrap().as_f64().unwrap() as u32,
            window.inner_height().unwrap().as_f64().unwrap() as u32,
        );

        canvas.set_width(resolution.0);
        canvas.set_height(resolution.1);

        let canvas_configuration =
            GpuCanvasConfiguration::new(&device, GpuTextureFormat::Bgra8unorm);

        let js_object = canvas.get_context("webgpu").map_err(|_| ())?.ok_or(())?;

        let js_value = JsValue::from(js_object);

        let context = GpuCanvasContext::from(js_value);

        context.configure(&canvas_configuration);

        //textures
        let depth_texture_size = JsValue::from(
            [
                JsValue::from_f64(canvas.width() as f64),
                JsValue::from_f64(canvas.height() as f64),
                JsValue::from_f64(1.0),
            ]
            .into_iter()
            .collect::<Array>(),
        );

        let depth_texture_desc = GpuTextureDescriptor::new(
            GpuTextureFormat::Depth32float,
            &depth_texture_size,
            web_sys::gpu_texture_usage::COPY_SRC | web_sys::gpu_texture_usage::RENDER_ATTACHMENT,
        );

        let depth_texture = device.create_texture(&depth_texture_desc);

        let depth_texture_view = depth_texture.create_view();

        let mut color_texture = context.get_current_texture();

        let mut color_texture_view = color_texture.create_view();

        //buffers
        let staging_buffer_desc = GpuBufferDescriptor::new(
            WEBGPU_MAX_BUFFER_SIZE as _,
            gpu_buffer_usage::COPY_SRC | gpu_buffer_usage::COPY_DST,
        );

        let staging_buffer = device.create_buffer(&staging_buffer_desc);

        let render_buffer_desc = GpuBufferDescriptor::new(
            mem::size_of::<Render>() as _,
            gpu_buffer_usage::COPY_DST | gpu_buffer_usage::UNIFORM,
        );

        let render_buffer = device.create_buffer(&render_buffer_desc);

        //shader module
        let shader_module_desc = GpuShaderModuleDescriptor::new(
            str::from_utf8(include_bytes!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/shaders/main.wgsl"
            )))
            .map_err(|_| ())?,
        );

        let shader_module = device.create_shader_module(&shader_module_desc);

        //bind groups
        const BINDING: u32 = 0;

        let buffer_binding_layout = GpuBufferBindingLayout::new();

        let mut bind_group_layout_entry =
            GpuBindGroupLayoutEntry::new(BINDING, gpu_shader_stage::VERTEX);
        bind_group_layout_entry.buffer(&buffer_binding_layout);

        let bind_group_layout_entries = [&bind_group_layout_entry].into_iter().collect::<Array>();

        let bind_group_layout_desc = GpuBindGroupLayoutDescriptor::new(&bind_group_layout_entries);

        let bind_group_layout = device.create_bind_group_layout(&bind_group_layout_desc);

        let bind_group_layouts = [&bind_group_layout].into_iter().collect::<Array>();

        let buffer_binding = GpuBufferBinding::new(&render_buffer);

        let bind_group_entry = GpuBindGroupEntry::new(BINDING, &buffer_binding);

        let bind_group_entries = [&bind_group_entry].into_iter().collect::<Array>();

        let bind_group_desc = GpuBindGroupDescriptor::new(&bind_group_entries, &bind_group_layout);

        let bind_group = device.create_bind_group(&bind_group_desc);

        //pipeline layout
        let pipeline_layout_desc = GpuPipelineLayoutDescriptor::new(&bind_group_layouts);

        let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_desc);

        //pipeline
        let position_vertex_attribute = web_sys::GpuVertexAttribute::new(GpuVertexFormat::Float32x4, 0.0, 0);

        let vertex_buffer_stride = (
   mem::size_of::<f32>() * 4 
        ) as _;

        let vertex_buffer_attributes = [
            &position_vertex_attribute
        ].into_iter()
        .collect::<Array>();

        let mut vertex_buffer_layout = web_sys::GpuVertexBufferLayout::new(vertex_buffer_stride, &vertex_buffer_attributes);
        vertex_buffer_layout.step_mode(GpuVertexStepMode::Vertex);

        let vertex_buffer_layouts = [
            &vertex_buffer_layout
        ].into_iter()
        .collect::<Array>();

        let mut vertex_state = GpuVertexState::new("vs", &shader_module);
        vertex_state.buffers(&vertex_buffer_layouts);

        let mut depth_stencil_state = GpuDepthStencilState::new(GpuTextureFormat::Depth32float);
        depth_stencil_state.depth_write_enabled(true);
        depth_stencil_state.depth_compare(GpuCompareFunction::Less);

        let color_state = GpuColorTargetState::new(GpuTextureFormat::Bgra8unorm);

        let fragment_state_targets = [&color_state].into_iter().collect::<Array>();

        let mut fragment_state =
            GpuFragmentState::new("fs", &shader_module, &fragment_state_targets);

        let mut primitive_state = GpuPrimitiveState::new();
        primitive_state.cull_mode(GpuCullMode::None);
        primitive_state.front_face(GpuFrontFace::Cw);
        primitive_state.topology(GpuPrimitiveTopology::TriangleList);

        let mut pipeline_desc = GpuRenderPipelineDescriptor::new(&pipeline_layout, &vertex_state);
        pipeline_desc.depth_stencil(&depth_stencil_state);
        pipeline_desc.fragment(&fragment_state);
        pipeline_desc.primitive(&primitive_state);

        let pipeline = device.create_render_pipeline(&pipeline_desc);

        Ok(Self {
            resolution,
            window,
            canvas,
            context,
            queue,
            device,
            color_texture,
            color_texture_view,
            depth_texture,
            depth_texture_view,
            render_buffer,
            staging_buffer,
            pipeline,
            bind_group,
        })
    }

    fn render(&mut self, render: Render, batches: &[&Batch]) {

        let current_resolution = (
            self.window.inner_width().unwrap().as_f64().unwrap() as u32,
            self.window.inner_height().unwrap().as_f64().unwrap() as u32,
        );

        if current_resolution != self.resolution {
            self.resolution = current_resolution;

            self.canvas.set_width(self.resolution.0);
            self.canvas.set_height(self.resolution.1);

            let depth_texture_size = JsValue::from(
                [
                    JsValue::from_f64(self.canvas.width() as f64),
                    JsValue::from_f64(self.canvas.height() as f64),
                    JsValue::from_f64(1.0),
                ]
                .into_iter()
                .collect::<Array>(),
            );

            let depth_texture_desc = GpuTextureDescriptor::new(
                GpuTextureFormat::Depth32float,
                &depth_texture_size,
                web_sys::gpu_texture_usage::COPY_SRC
                    | web_sys::gpu_texture_usage::RENDER_ATTACHMENT,
            );

            self.depth_texture = self.device.create_texture(&depth_texture_desc);

            self.depth_texture_view = self.depth_texture.create_view();
        }
        //encode commands
        self.color_texture = self.context.get_current_texture();
        self.color_texture_view = self.color_texture.create_view();

        let color_clear_value = unsafe { js_sys::Float64Array::view(&[1.0, 1.0, 1.0, 1.0]) };

        let mut color_attachment = GpuRenderPassColorAttachment::new(
            GpuLoadOp::Clear,
            GpuStoreOp::Store,
            &self.color_texture_view,
        );
        color_attachment.clear_value(&color_clear_value);

        let color_attachments = [&color_attachment].into_iter().collect::<Array>();

        let mut depth_attachment =
            GpuRenderPassDepthStencilAttachment::new(&self.depth_texture_view);
        depth_attachment.depth_load_op(GpuLoadOp::Clear);
        depth_attachment.depth_store_op(GpuStoreOp::Store);
        depth_attachment.depth_clear_value(1.0);

        let mut render_pass_desc = GpuRenderPassDescriptor::new(&color_attachments);
        render_pass_desc.depth_stencil_attachment(&depth_attachment);

        
                self.queue.write_buffer_with_f64_and_u8_array(
                    &self.staging_buffer,
                    0.0,
                    to_bytes(&[render]),
                );
        

        let command_encoder = self.device.create_command_encoder();

        command_encoder.copy_buffer_to_buffer_with_f64_and_f64_and_f64(
            &self.staging_buffer,
            0.0,
            &self.render_buffer,
            0.0,
            mem::size_of::<Render>() as _
        );

        let mut render_pass = command_encoder.begin_render_pass(&render_pass_desc);
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_viewport(
            0.0,
            0.0,
            self.canvas.width() as f32,
            self.canvas.height() as f32,
            0.0,
            1.0,
        );
        render_pass.set_scissor_rect(0, 0, self.canvas.width(), self.canvas.height());
        render_pass.set_bind_group(0, &self.bind_group);
        render_pass.set_vertex_buffer(0, &batches[0].vertex_buffer);
        render_pass.set_index_buffer(&batches[0].index_buffer, GpuIndexFormat::Uint16);
        render_pass.draw_indexed(3);
        render_pass.end();

        let command_buffer = command_encoder.finish();

        let command_submission = [&command_buffer].into_iter().collect::<Array>();

        self.queue.submit(&command_submission);
    }

    fn batch(&self, vertices: &[Vertex], indices: &[Index]) -> Batch {
        use js_sys::*;
        use web_sys::*;
        
        let vertex_buffer_size = ((mem::size_of::<Vertex>() * vertices.len()) as f64 / 4.0).ceil() * 4.0;
        
        let vertex_buffer_desc = GpuBufferDescriptor::new(
            vertex_buffer_size,
            gpu_buffer_usage::COPY_DST | gpu_buffer_usage::VERTEX,
        );

        let vertex_buffer = self.device.create_buffer(&vertex_buffer_desc);

        let index_buffer_size = ((mem::size_of::<Index>() * indices.len()) as f64 / 4.0).ceil() * 4.0;

        let index_buffer_desc = GpuBufferDescriptor::new(
            index_buffer_size,
            gpu_buffer_usage::COPY_DST | gpu_buffer_usage::INDEX,
        );

        let index_buffer = self.device.create_buffer(&index_buffer_desc);
        
        fn upload_and_transfer<T: Copy>(
            webgpu: &WebGpu,
            data: &[T],
            recipient_buffer: &web_sys::GpuBuffer,
        ) {
            let mut bytes = to_bytes(data).to_vec();
                
            while bytes.len() % 4 != 0 {
                bytes.push(0);
            }

            for cursor in (0..bytes.len()).step_by(WEBGPU_MAX_BUFFER_SIZE) {
                let start = cursor;
                let length = cmp::min(WEBGPU_MAX_BUFFER_SIZE, bytes.len());
                let end = start + length;

                webgpu.queue.write_buffer_with_f64_and_u8_array(
                    &webgpu.staging_buffer,
                    0.0,
                    &bytes[start..end],
                );

                let command_encoder = webgpu.device.create_command_encoder();

                command_encoder.copy_buffer_to_buffer_with_f64_and_f64_and_f64(
                    &webgpu.staging_buffer,
                    0.0,
                    recipient_buffer,
                    start as _, 
                    length as _,
                );

                let command_buffer = command_encoder.finish();

                let command_submission = [&command_buffer].into_iter().collect::<Array>();

                webgpu.queue.submit(&command_submission);
            }
        }

        upload_and_transfer(&self, vertices, &vertex_buffer);
        upload_and_transfer(&self, indices, &index_buffer);

        Batch {
            vertex_buffer,
            index_buffer,
        }
    }
}
}

