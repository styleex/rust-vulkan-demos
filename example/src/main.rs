use std::ops::RangeInclusive;
use std::path::Path;
use std::ptr;
use std::sync::Arc;

use ash::version::DeviceV1_0;
use ash::vk;
use cgmath::{Matrix4, SquareMatrix};
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::platform::run_return::EventLoopExtRunReturn;

use ash_render_env::{env, frame_buffer};
use ash_render_env::camera::Camera;
use ash_render_env::egui::Egui;
use ash_render_env::env::RenderEnv;
use ash_render_env::fps_limiter::FPSLimiter;
use ash_render_env::primary_cmd_buffer::PrimaryCommandBuffer;
use utils::{render_pass, sync};

use crate::shadow_map::ShadowMapFramebuffer;
use crate::utils::heightmap_terrain::terrain::{HeightMap, TerrainData};
use crate::utils::heightmap_terrain::terrain_renderer::TerrainRenderer;
use crate::utils::mesh::Mesh;
use crate::utils::mesh_render::MeshRenderer;
use crate::utils::mesh_shadowmap_render::MeshShadowMapRenderer;
use crate::utils::quad_render::QuadRenderer;
use crate::utils::skybox_render::SkyboxRenderer;
use crate::utils::sync::MAX_FRAMES_IN_FLIGHT;

mod utils;
mod shadow_map;

struct HelloApplication {
    egui: Egui,

    final_pass_draw_command: PrimaryCommandBuffer,
    geometry_pass_draw_command: PrimaryCommandBuffer,
    shadowmap_pass_draw_command: PrimaryCommandBuffer,

    quad_renderer: QuadRenderer,
    swapchain_stuff: ash_render_env::swapchain::SwapChain,

    mesh: Arc<Mesh>,
    mesh_renderer: MeshRenderer,
    mesh_shadow_map_renderer: MeshShadowMapRenderer,

    skybox_renderer: SkyboxRenderer,

    sync: sync::SyncObjects,

    current_frame: usize,
    is_window_resized: bool,

    camera: Camera,

    offscreen_buffer: frame_buffer::Framebuffer,

    terrain_renderer: TerrainRenderer,

    final_render_pass: vk::RenderPass,

    clear_color: [f32; 3],
    tick_counter: FPSLimiter,

    shadow_map_fb: ShadowMapFramebuffer,

    env: Arc<env::RenderEnv>,
    light_vp: Matrix4<f32>,
    cascade_split_lambda: f32,
}

impl HelloApplication {
    pub fn new(wnd: &winit::window::Window) -> HelloApplication {
        let env = Arc::new(env::RenderEnv::new(wnd));

        let msaa_samples = ash_render_env::utils::get_max_usable_sample_count(&env);

        let mut swapchain_stuff = ash_render_env::swapchain::SwapChain::new(&env, wnd.inner_size());

        let quad_render_pass = render_pass::create_quad_render_pass(env.device(), swapchain_stuff.format);
        swapchain_stuff.create_framebuffers(env.device(), quad_render_pass);

        let mut camera = Camera::new();
        camera.set_viewport(
            swapchain_stuff.size.width,
            swapchain_stuff.size.height,
        );

        let dimensions = [swapchain_stuff.size.width, swapchain_stuff.size.height];
        let mut offscreen_framebuffer = frame_buffer::Framebuffer::new(env.clone(), vec!(
            frame_buffer::AttachmentDesciption {  // color
                samples_count: msaa_samples,
                format: vk::Format::R8G8B8A8_SRGB,
            },
            frame_buffer::AttachmentDesciption {  // pos
                samples_count: msaa_samples,
                format: vk::Format::R16G16B16A16_SFLOAT,
            },
            frame_buffer::AttachmentDesciption {  // normal
                samples_count: msaa_samples,
                format: vk::Format::R16G16B16A16_SFLOAT,
            },
            frame_buffer::AttachmentDesciption {  // depth
                samples_count: msaa_samples,
                format: vk::Format::D32_SFLOAT,
            },
        ));
        offscreen_framebuffer.resize_swapchain(dimensions);

        let sync = sync::create_sync_objects(env.device());

        let mut egui = Egui::new(env.clone(), swapchain_stuff.format, wnd.scale_factor(), dimensions, MAX_FRAMES_IN_FLIGHT, msaa_samples);
        egui.register_texture(0, offscreen_framebuffer.attachments[2].view, true);

        let mut draw_mesh_render_system = PrimaryCommandBuffer::new(env.clone(), MAX_FRAMES_IN_FLIGHT);
        draw_mesh_render_system.set_dimensions(dimensions);

        let mut quad_render_system = PrimaryCommandBuffer::new(env.clone(), MAX_FRAMES_IN_FLIGHT);
        quad_render_system.set_dimensions(dimensions);

        let mesh = Arc::new(
            Mesh::load_from_file(env.clone(), Path::new("assets/chalet2.obj"))
        );

        let mesh_renderer = MeshRenderer::new(
            env.clone(),
            offscreen_framebuffer.render_pass(),
            mesh.clone(),
            offscreen_framebuffer.attachments.len() - 1, // color attachments only
            msaa_samples,
            MAX_FRAMES_IN_FLIGHT,
            dimensions,
        );

        let skybox_renderer = SkyboxRenderer::new(
            env.clone(),
            offscreen_framebuffer.render_pass(),
            offscreen_framebuffer.attachments.len() - 1, // color attachments only
            msaa_samples,
            MAX_FRAMES_IN_FLIGHT,
            dimensions,
        );

        let height_map = HeightMap::from_png(Path::new("./assets/terrain/heightmap2.png"));
        let terrain_data = TerrainData::new(env.clone(), height_map);
        let terrain_renderer = TerrainRenderer::new(
            env.clone(),
            offscreen_framebuffer.render_pass(),
            offscreen_framebuffer.attachments.len() - 1,
            terrain_data, msaa_samples,
            MAX_FRAMES_IN_FLIGHT,
            dimensions);


        let shadow_map_fb = ShadowMapFramebuffer::new(env.clone());
        egui.register_texture_layout(1, shadow_map_fb.get_cascade_view(0), vk::ImageLayout::DEPTH_STENCIL_READ_ONLY_OPTIMAL);

        let mut shadowmap_pass_draw_command = PrimaryCommandBuffer::new(env.clone(), MAX_FRAMES_IN_FLIGHT);
        shadowmap_pass_draw_command.set_dimensions([4096 as u32, 4096 as u32]);

        let mesh_shadow_map_renderer = MeshShadowMapRenderer::new(
            env.clone(),
            shadow_map_fb.render_pass(),
            mesh.clone(),
            MAX_FRAMES_IN_FLIGHT,
            [4096, 4096],
        );

        let quad_renderer = QuadRenderer::new(
            env.clone(),
            &offscreen_framebuffer,
            shadow_map_fb.get_cascade_view(0),
            quad_render_pass,
            msaa_samples,
            dimensions);

        println!("created");

        let tick_counter = FPSLimiter::new();
        HelloApplication {
            env,
            shadow_map_fb,
            final_pass_draw_command: quad_render_system,
            geometry_pass_draw_command: draw_mesh_render_system,
            shadowmap_pass_draw_command,

            quad_renderer,
            swapchain_stuff,

            sync,
            current_frame: 0,
            is_window_resized: false,
            camera,

            offscreen_buffer: offscreen_framebuffer,

            egui,

            clear_color: [0.0, 0.0, 0.0],
            final_render_pass: quad_render_pass,

            mesh,
            mesh_renderer,
            mesh_shadow_map_renderer,

            skybox_renderer,
            terrain_renderer,

            tick_counter,
            light_vp: Matrix4::identity(),
            cascade_split_lambda: 0.9,
        }
    }

    pub fn run(&mut self, mut event_loop: EventLoop<()>, wnd: winit::window::Window) {
        event_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent { event, window_id } => {
                    if window_id != wnd.id() {
                        return;
                    }

                    if let WindowEvent::CloseRequested = event {
                        *control_flow = ControlFlow::Exit;
                        return;
                    }

                    if let WindowEvent::KeyboardInput { input, .. } = event {
                        if input.virtual_keycode.is_some() && input.virtual_keycode.unwrap() == VirtualKeyCode::Escape {
                            *control_flow = ControlFlow::Exit;
                            return;
                        }
                    }

                    if let WindowEvent::Resized(_) = event {
                        self.is_window_resized = true;
                    }

                    if !self.egui.context().is_pointer_over_area() {
                        let changed = self.camera.handle_event(&event);
                        if changed {
                            self.light_vp = self.shadow_map_fb.update_cascades(&self.camera, self.cascade_split_lambda);
                        }
                    }

                    if !self.camera.mouse_acquired() {
                        self.egui.handle_event(&event);
                    }
                }
                Event::MainEventsCleared => {
                    wnd.request_redraw()
                }
                Event::RedrawRequested(_) => {
                    self.draw_frame(&wnd);
                }
                // Important!
                Event::LoopDestroyed => {
                    unsafe { self.env.device().device_wait_idle().unwrap(); }
                }
                _ => (),
            }
        })
    }

    fn draw_frame(&mut self, wnd: &winit::window::Window) {
        let wait_fences = [self.sync.inflight_fences[self.current_frame]];

        let (image_index, _is_sub_optimal) = unsafe {
            self.env.device()
                .wait_for_fences(&wait_fences, true, u64::MAX)
                .expect("Failed to wait for Fence!");

            let result = self.swapchain_stuff.swapchain_api
                .acquire_next_image(
                    self.swapchain_stuff.swapchain,
                    u64::MAX,
                    self.sync.image_available_semaphores[self.current_frame],
                    vk::Fence::null(),
                );
            match result {
                Ok(image_index) => image_index,
                Err(vk_result) => match vk_result {
                    vk::Result::ERROR_OUT_OF_DATE_KHR => {
                        self.recreate_swapchain(&wnd);
                        return;
                    }
                    _ => panic!("Failed to acquire Swap Chain Image!"),
                },
            }
        };
        let wait_semaphores = [self.sync.image_available_semaphores[self.current_frame]];
        let wait_stages = [vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];
        let first_pass_finished = [self.sync.render_finished_semaphores[self.current_frame]];
        let second_pass_finished = [self.sync.render_quad_semaphore];

        let clear_values = vec![
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [self.clear_color[0], self.clear_color[1], self.clear_color[2], 1.0],
                },
            },
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [0.0, 0.0, 0.0, 1.0],
                },
            },
            vk::ClearValue {
                depth_stencil: vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                }
            },
        ];

        let shadow_map_clear = vec![vk::ClearValue {
            depth_stencil: vk::ClearDepthStencilValue {
                depth: 1.0,
                stencil: 0,
            }
        }];
        let mesh_shadowmap_draw = self.mesh_shadow_map_renderer.draw(&self.camera, self.light_vp);
        let shadowmap_pass_cmd = self.shadowmap_pass_draw_command.execute_secondary(
            shadow_map_clear,
            self.shadow_map_fb.frambuffer(0),
            self.shadow_map_fb.render_pass(),
            &[mesh_shadowmap_draw],
        );

        self.quad_renderer.update_shadows(self.camera.view_matrix(), self.light_vp);
        let mesh_draw = self.mesh_renderer.draw(self.camera.view_matrix(), self.camera.proj_matrix());
        let terrain_draw = self.terrain_renderer.draw(self.camera.view_matrix(), self.camera.proj_matrix());
        let skybox_draw = self.skybox_renderer.draw(self.camera.skybox_view_matrix(), self.camera.proj_matrix());

        let geometry_pass_cmd = self.geometry_pass_draw_command.execute_secondary(
            clear_values,
            self.offscreen_buffer.framebuffer.unwrap(),
            self.offscreen_buffer.render_pass,
            &[terrain_draw, mesh_draw, skybox_draw]);

        self.egui.begin_frame();
        self.render_gui();
        let gui_render_op = self.egui.end_frame(wnd);

        let clear_values = vec![
            vk::ClearValue {
                color: vk::ClearColorValue {
                    float32: [self.clear_color[0], self.clear_color[1], self.clear_color[2], 1.0],
                },
            },
        ];

        let quad_cmd_buf = self.final_pass_draw_command.execute_secondary(
            clear_values,
            self.swapchain_stuff.framebuffers[image_index as usize],
            self.quad_renderer.render_pass,
            &[self.quad_renderer.second_buffer, gui_render_op],
        );

        let mrt_pass = [geometry_pass_cmd, shadowmap_pass_cmd];
        let composite_pass = [quad_cmd_buf];

        let submit_infos = [
            vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: ptr::null(),
                wait_semaphore_count: wait_semaphores.len() as u32,
                p_wait_semaphores: wait_semaphores.as_ptr(),
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: mrt_pass.len() as u32,
                p_command_buffers: mrt_pass.as_ptr(),
                signal_semaphore_count: first_pass_finished.len() as u32,
                p_signal_semaphores: first_pass_finished.as_ptr(),
            },
            vk::SubmitInfo {
                s_type: vk::StructureType::SUBMIT_INFO,
                p_next: ptr::null(),
                wait_semaphore_count: first_pass_finished.len() as u32,
                p_wait_semaphores: first_pass_finished.as_ptr(),
                p_wait_dst_stage_mask: wait_stages.as_ptr(),
                command_buffer_count: 1,
                p_command_buffers: composite_pass.as_ptr(),
                signal_semaphore_count: second_pass_finished.len() as u32,
                p_signal_semaphores: second_pass_finished.as_ptr(),
            },
        ];

        unsafe {
            self.env.device()
                .reset_fences(&wait_fences)
                .expect("Failed to reset Fence!");

            self.env.device()
                .queue_submit(
                    self.env.queue(),
                    &submit_infos,
                    self.sync.inflight_fences[self.current_frame],
                )
                .expect("Failed to execute queue submit.");
        }
        let swapchains = [self.swapchain_stuff.swapchain];

        let present_info = vk::PresentInfoKHR {
            s_type: vk::StructureType::PRESENT_INFO_KHR,
            p_next: ptr::null(),
            wait_semaphore_count: 1,
            p_wait_semaphores: second_pass_finished.as_ptr(),
            swapchain_count: 1,
            p_swapchains: swapchains.as_ptr(),
            p_image_indices: &image_index,
            p_results: ptr::null_mut(),
        };

        let result = unsafe {
            self.swapchain_stuff.swapchain_api
                .queue_present(self.env.queue(), &present_info)
        };

        let is_resized = match result {
            Ok(_) => self.is_window_resized,
            Err(vk_result) => match vk_result {
                vk::Result::ERROR_OUT_OF_DATE_KHR | vk::Result::SUBOPTIMAL_KHR => true,
                _ => panic!("Failed to execute queue present"),
            }
        };

        if is_resized {
            self.recreate_swapchain(wnd);
            self.is_window_resized = false;
        }

        // FIXME: если раскомментировать - то теневые карты начнут дрожжать при передвижении
        // self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;
    }

    fn render_gui(&mut self) {
        egui::SidePanel::left("my_side_panel").show(&self.egui.context(), |ui| {
            ui.heading("Hello");
            ui.separator();

            // let mut rgb: [f32; 3] = [0.0, 0.0, 0.0];
            ui.color_edit_button_rgb(&mut self.clear_color);

            ui.separator();

            let camera_pos = self.camera.position();
            let view_dir = self.camera.view_dir();
            ui.label(format!("X: {:.2}, Y: {:.2}, Z: {:.2}", camera_pos.x, camera_pos.y, camera_pos.z));

            ui.label(format!("X: {:.2}, Y: {:.2}, Z: {:.2}", view_dir.x, view_dir.y, view_dir.z));
            ui.label(format!("FPS: {:.2}", self.tick_counter.fps()));
            ui.image(egui::TextureId::User(1), [200.0, 200.0]);

            let resp = ui.add(egui::DragValue::new(&mut self.cascade_split_lambda).speed(0.01).clamp_range(RangeInclusive::new(0.1, 1.0)));
            if resp.changed() {
                self.light_vp = self.shadow_map_fb.update_cascades(&self.camera, self.cascade_split_lambda);
            }
        });
    }

    fn recreate_swapchain(&mut self, wnd: &winit::window::Window) {
        unsafe {
            self.env.device()
                .device_wait_idle()
                .expect("Failed to wait device idle!")
        };
        self.cleanup_swapchain();

        self.swapchain_stuff = ash_render_env::swapchain::SwapChain::new(&self.env, wnd.inner_size());
        self.swapchain_stuff.create_framebuffers(self.env.device(), self.final_render_pass);

        let dimensions = [self.swapchain_stuff.size.width, self.swapchain_stuff.size.height];
        self.geometry_pass_draw_command.set_dimensions(dimensions);
        self.final_pass_draw_command.set_dimensions(dimensions);

        self.offscreen_buffer.resize_swapchain(dimensions);
        self.egui.set_dimensions(dimensions);
        self.egui.register_texture(0, self.offscreen_buffer.attachments[2].view, true);

        self.quad_renderer.update_framebuffer(&self.offscreen_buffer, self.shadow_map_fb.get_cascade_view(0), dimensions);
        self.mesh_renderer.resize_framebuffer(dimensions);
        self.skybox_renderer.resize_framebuffer(dimensions);
        self.terrain_renderer.resize_framebuffer(dimensions);
    }

    fn cleanup_swapchain(&mut self) {
        self.swapchain_stuff.destroy();
    }
}

impl Drop for HelloApplication {
    fn drop(&mut self) {
        unsafe {
            self.sync.destroy();
            self.cleanup_swapchain();

            self.offscreen_buffer.destroy();
            self.env.device().destroy_render_pass(self.final_render_pass, None);
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let wnd = winit::window::WindowBuilder::new()
        .with_title("test")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600))
        .build(&event_loop)
        .expect("Failed to create window");

    let mut app = HelloApplication::new(&wnd);
    app.run(event_loop, wnd);
}
