//! AMD FidelityFX Super Resolution 1.0 (FSR 1.0) spatial upscaling (EASU + RCAS).
//!
//! Provides [`FsrPipeline`] for Edge-Adaptive Spatial Upsampling (EASU) and Robust Contrast-Adaptive
//! Sharpening (RCAS), upscaling lower-resolution shader pack rendering to native display resolution.

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct FsrConstants {
    pub src_size: [f32; 2],
    pub dst_size: [f32; 2],
    pub sharpness: f32,
    pub _pad: f32,
}

pub struct FsrPipeline {
    pub format: wgpu::TextureFormat,
    pub easu_pipeline: wgpu::RenderPipeline,
    pub rcas_pipeline: wgpu::RenderPipeline,
    pub integer_pipeline: wgpu::RenderPipeline,
    pub sgsr_pipeline: wgpu::RenderPipeline,
    pub bind_group_layout: wgpu::BindGroupLayout,
    pub sampler: wgpu::Sampler,
    pub uniform_buffer: wgpu::Buffer,
    pub intermediate_texture: Option<wgpu::Texture>,
    pub intermediate_target: Option<wgpu::TextureView>,
    pub intermediate_size: [u32; 2],
    pub last_constants: Option<FsrConstants>,
    pub cached_easu_bind_group: Option<wgpu::BindGroup>,
    pub cached_rcas_bind_group: Option<wgpu::BindGroup>,
    pub cached_rcas_standalone_bind_group: Option<wgpu::BindGroup>,
    pub cached_integer_bind_group: Option<wgpu::BindGroup>,
    pub cached_sgsr_bind_group: Option<wgpu::BindGroup>,
}

impl FsrPipeline {
    /// Builds the FSR 1.0 EASU and RCAS render pipelines for a render target of `format`.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("FSR 1.0 Shader Module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("fsr.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("FSR Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: Some(
                            std::num::NonZeroU64::new(std::mem::size_of::<FsrConstants>() as u64)
                                .expect("non-zero size"),
                        ),
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("FSR Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            ..Default::default()
        });

        let easu_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FSR EASU Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fsr"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_easu"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let rcas_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("FSR RCAS Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fsr"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_rcas"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let integer_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shader Integer Scaling Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fsr"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_integer"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let sgsr_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("SGSR v1 Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_fsr"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            primitive: Default::default(),
            depth_stencil: None,
            multisample: Default::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_sgsr"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("FSR Linear Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("FSR Constants Uniform Buffer"),
            size: std::mem::size_of::<FsrConstants>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            format,
            easu_pipeline,
            rcas_pipeline,
            integer_pipeline,
            sgsr_pipeline,
            bind_group_layout,
            sampler,
            uniform_buffer,
            intermediate_texture: None,
            intermediate_target: None,
            intermediate_size: [0, 0],
            last_constants: None,
            cached_easu_bind_group: None,
            cached_rcas_bind_group: None,
            cached_rcas_standalone_bind_group: None,
            cached_integer_bind_group: None,
            cached_sgsr_bind_group: None,
        }
    }

    /// Resizes the intermediate render target to `dst_size`.
    pub fn resize(&mut self, device: &wgpu::Device, dst_size: [u32; 2]) {
        if self.intermediate_size == dst_size && self.intermediate_target.is_some() {
            return;
        }
        self.cached_easu_bind_group = None;
        self.cached_rcas_bind_group = None;
        self.cached_rcas_standalone_bind_group = None;
        self.cached_integer_bind_group = None;
        self.cached_sgsr_bind_group = None;
        self.intermediate_size = dst_size;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("FSR intermediate target"),
            size: wgpu::Extent3d {
                width: dst_size[0].max(1),
                height: dst_size[1].max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        self.intermediate_target = Some(texture.create_view(&Default::default()));
        self.intermediate_texture = Some(texture);
    }

    /// Executes FSR 1.0 two-pass upscaling:
    /// 1. EASU: reconstructed upsampling from `src` at `src_size` into intermediate target at `dst_size`.
    /// 2. RCAS: contrast-adaptive sharpening from intermediate target directly into `dst` at `dst_size`.
    #[allow(clippy::too_many_arguments)]
    pub fn run(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        dst: &wgpu::TextureView,
        src_size: [u32; 2],
        dst_size: [u32; 2],
        sharpness: f32,
    ) {
        self.resize(device, dst_size);
        let intermediate = self
            .intermediate_target
            .as_ref()
            .expect("intermediate target allocated");

        let constants = FsrConstants {
            src_size: [src_size[0] as f32, src_size[1] as f32],
            dst_size: [dst_size[0] as f32, dst_size[1] as f32],
            sharpness: sharpness.clamp(0.0, 1.0),
            _pad: 0.0,
        };
        if self.last_constants != Some(constants) {
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&constants));
            self.last_constants = Some(constants);
        }

        // Pass 1: EASU (src -> intermediate)
        if self.cached_easu_bind_group.is_none() {
            self.cached_easu_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FSR EASU Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(src),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }));
        }
        let easu_bind_group = self.cached_easu_bind_group.as_ref().unwrap();

        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FSR EASU Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: intermediate,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            rp.set_pipeline(&self.easu_pipeline);
            rp.set_bind_group(0, easu_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }

        // Pass 2: RCAS (intermediate -> dst)
        if self.cached_rcas_bind_group.is_none() {
            let intermediate = self
                .intermediate_target
                .as_ref()
                .expect("intermediate target allocated");
            self.cached_rcas_bind_group = Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("FSR RCAS Bind Group"),
                layout: &self.bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(intermediate),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::Sampler(&self.sampler),
                    },
                ],
            }));
        }
        let rcas_bind_group = self.cached_rcas_bind_group.as_ref().unwrap();

        {
            let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("FSR RCAS Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dst,
                    resolve_target: None,
                    depth_slice: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            rp.set_pipeline(&self.rcas_pipeline);
            rp.set_bind_group(0, rcas_bind_group, &[]);
            rp.draw(0..3, 0..1);
        }
    }

    /// Executes standalone RCAS sharpening directly from `src` to `dst` at `size`.
    pub fn run_rcas(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        dst: &wgpu::TextureView,
        size: [u32; 2],
        sharpness: f32,
    ) {
        let constants = FsrConstants {
            src_size: [size[0] as f32, size[1] as f32],
            dst_size: [size[0] as f32, size[1] as f32],
            sharpness: sharpness.clamp(0.0, 1.0),
            _pad: 0.0,
        };
        if self.last_constants != Some(constants) {
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&constants));
            self.last_constants = Some(constants);
        }

        if self.cached_rcas_standalone_bind_group.is_none() {
            self.cached_rcas_standalone_bind_group =
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("FSR RCAS Standalone Bind Group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(src),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                }));
        }
        let rcas_bind_group = self.cached_rcas_standalone_bind_group.as_ref().unwrap();

        let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("FSR RCAS Standalone Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: dst,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        rp.set_pipeline(&self.rcas_pipeline);
        rp.set_bind_group(0, rcas_bind_group, &[]);
        rp.draw(0..3, 0..1);
    }

    /// Executes integer scaling directly from `src` at `src_size` to `dst` at `dst_size`.
    pub fn run_integer(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        dst: &wgpu::TextureView,
        src_size: [u32; 2],
        dst_size: [u32; 2],
    ) {
        let constants = FsrConstants {
            src_size: [src_size[0] as f32, src_size[1] as f32],
            dst_size: [dst_size[0] as f32, dst_size[1] as f32],
            sharpness: 0.0,
            _pad: 0.0,
        };
        if self.last_constants != Some(constants) {
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&constants));
            self.last_constants = Some(constants);
        }

        if self.cached_integer_bind_group.is_none() {
            self.cached_integer_bind_group =
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("Shader Integer Scaling Bind Group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(src),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                }));
        }
        let bind_group = self.cached_integer_bind_group.as_ref().unwrap();

        let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Shader Integer Scaling Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: dst,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        rp.set_pipeline(&self.integer_pipeline);
        rp.set_bind_group(0, bind_group, &[]);
        rp.draw(0..3, 0..1);
    }

    /// Executes Qualcomm SGSR v1 single-pass spatial upscaling from `src` at `src_size`
    /// directly to `dst` at `dst_size` without an intermediate render target.
    #[allow(clippy::too_many_arguments)]
    pub fn run_sgsr(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        src: &wgpu::TextureView,
        dst: &wgpu::TextureView,
        src_size: [u32; 2],
        dst_size: [u32; 2],
        sharpness: f32,
    ) {
        let constants = FsrConstants {
            src_size: [src_size[0] as f32, src_size[1] as f32],
            dst_size: [dst_size[0] as f32, dst_size[1] as f32],
            sharpness: sharpness.clamp(0.0, 1.0),
            _pad: 0.0,
        };
        if self.last_constants != Some(constants) {
            queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&constants));
            self.last_constants = Some(constants);
        }

        if self.cached_sgsr_bind_group.is_none() {
            self.cached_sgsr_bind_group =
                Some(device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("SGSR v1 Bind Group"),
                    layout: &self.bind_group_layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: self.uniform_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::TextureView(src),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::Sampler(&self.sampler),
                        },
                    ],
                }));
        }
        let bind_group = self.cached_sgsr_bind_group.as_ref().unwrap();

        let mut rp = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("SGSR v1 Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: dst,
                resolve_target: None,
                depth_slice: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        rp.set_pipeline(&self.sgsr_pipeline);
        rp.set_bind_group(0, bind_group, &[]);
        rp.draw(0..3, 0..1);
    }
}
