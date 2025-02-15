use std::mem;
use std::num::NonZeroU32;
use std::ops::Mul;

use super::extract::EntityStore;
use super::grass_pipeline::GrassPipeline;
use crate::grass_spawner::{GrassSpawner, GrassSpawnerFlags, HeightRepresentation};
use crate::render::cache::GrassCache;
use crate::GrassConfiguration;
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use bevy::render::render_asset::RenderAssets;
use bevy::render::render_resource::{
    BindGroupDescriptor, BindGroupEntry, BindingResource, BufferBinding, BufferInitDescriptor,
    BufferUsages, Extent3d, ImageCopyTexture, ImageDataLayout, Origin3d, ShaderType, TextureAspect,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor, TextureViewDimension, TextureViewId,
};
use bevy::render::renderer::{RenderDevice, RenderQueue};
use bevy::render::texture::FallbackImage;
use bytemuck::{Pod, Zeroable};

pub(crate) fn prepare_explicit_xz_buffer(
    mut cache: ResMut<GrassCache>,
    pipeline: Res<GrassPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut inserted_grass: Query<(&mut GrassSpawner, &EntityStore)>,
) {
    for (mut spawner, EntityStore(id)) in inserted_grass.iter_mut() {
        if !spawner.flags.contains(GrassSpawnerFlags::XZ_DEFINED) {
            panic!("Cannot spawn grass without the xz-positions defined");
        }

        if let Some(chunk) = cache.get_mut(id) {
            chunk.instance_count = spawner.positions_xz.len();
            let view = prepare_texture_from_data(
                &mut spawner.positions_xz,
                &render_device,
                &render_queue,
                TextureFormat::Rg32Float,
            );
            let layout = pipeline.explicit_xz_layout.clone();
            let bind_group_descriptor = BindGroupDescriptor {
                label: Some("grass explicit y positions bind group"),
                layout: &layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                }],
            };
            let bind_group = render_device.create_bind_group(&bind_group_descriptor);
            chunk.explicit_xz_buffer = Some(bind_group);

            chunk.flags = spawner.flags;
        } else {
            warn!(
                "Tried to prepare a entity buffer for a grass chunk which wasn't registered before"
            );
        }
    }
}

pub(crate) fn prepare_height_buffer(
    mut cache: ResMut<GrassCache>,
    pipeline: Res<GrassPipeline>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut inserted_grass: Query<(&mut GrassSpawner, &EntityStore)>,
) {
    for (mut spawner, EntityStore(id)) in inserted_grass.iter_mut() {
        if let Some(chunk) = cache.get_mut(id) {
            let view = match &mut spawner.heights {
                HeightRepresentation::Uniform(height) => {
                    let mut heights = vec![*height; spawner.positions_xz.len()];
                    prepare_texture_from_data(
                        &mut heights,
                        &render_device,
                        &render_queue,
                        TextureFormat::R32Float,
                    )
                }
                HeightRepresentation::PerBlade(heights) => prepare_texture_from_data(
                    heights,
                    &render_device,
                    &render_queue,
                    TextureFormat::R32Float,
                ),
            };
            let layout = pipeline.explicit_xz_layout.clone();
            let bind_group_descriptor = BindGroupDescriptor {
                label: Some("grass height bind group"),
                layout: &layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                }],
            };
            let bind_group = render_device.create_bind_group(&bind_group_descriptor);
            chunk.height_buffer = Some(bind_group);

            chunk.flags = spawner.flags;
        } else {
            warn!(
                "Tried to prepare a entity buffer for a grass chunk which wasn't registered before"
            );
        }
    }
}
pub(crate) fn prepare_explicit_y_buffer(
    mut cache: ResMut<GrassCache>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    pipeline: Res<GrassPipeline>,
    mut inserted_grass: Query<(&mut GrassSpawner, &EntityStore)>,
) {
    for (mut spawner, EntityStore(id)) in inserted_grass.iter_mut() {
        if !spawner.flags.contains(GrassSpawnerFlags::Y_DEFINED) {
            panic!("Cannot spawn grass without the y-positions defined");
        }
        if spawner.flags.contains(GrassSpawnerFlags::HEIGHT_MAP) {
            continue;
        }
        if let Some(chunk) = cache.get_mut(id) {
            let view = prepare_texture_from_data(
                &mut spawner.positions_y,
                &render_device,
                &render_queue,
                TextureFormat::R32Float,
            );
            let layout = pipeline.explicit_y_layout.clone();
            let bind_group_descriptor = BindGroupDescriptor {
                label: Some("grass explicit y positions bind group"),
                layout: &layout,
                entries: &[BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(&view),
                }],
            };
            let bind_group = render_device.create_bind_group(&bind_group_descriptor);
            chunk.explicit_y_buffer = Some(bind_group);
        } else {
            warn!(
                "Tried to prepare a entity buffer for a grass chunk which wasn't registered before"
            );
        }
    }
}

pub(crate) fn prepare_height_map_buffer(
    mut cache: ResMut<GrassCache>,
    render_device: Res<RenderDevice>,
    pipeline: Res<GrassPipeline>,
    fallback_img: Res<FallbackImage>,
    images: Res<RenderAssets<Image>>,
    inserted_grass: Query<(&GrassSpawner, &EntityStore, &Aabb)>,
    mut local_height_map_buffer: Local<Vec<(EntityStore, Handle<Image>, Aabb)>>,
) {
    let mut to_remove = Vec::new();

    for (EntityStore(e), handle, aabb) in local_height_map_buffer.iter() {
        if let Some(tex) = images.get(handle) {
            to_remove.push(*e);
            let height_map_texture = &tex.texture_view;
            let aabb_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                label: Some("aabb buffer"),
                contents: bytemuck::bytes_of(&aabb.half_extents.mul(2.).as_dvec3().as_vec3()),
                usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            });
            let layout = pipeline.height_map_layout.clone();
            let bind_group_descriptor = BindGroupDescriptor {
                label: Some("grass height map bind group"),
                layout: &layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: BindingResource::TextureView(height_map_texture),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: BindingResource::Buffer(BufferBinding {
                            buffer: &aabb_buffer,
                            offset: 0,
                            size: None,
                        }),
                    },
                ],
            };

            let bind_group = render_device.create_bind_group(&bind_group_descriptor);
            if let Some(chunk) = cache.get_mut(e) {
                chunk.height_map = Some(bind_group);
            } else {
                warn!("Tried to prepare a buffer for a grass chunk which wasn't registered before");
            }
        }
    }
    local_height_map_buffer.retain(|map| !to_remove.contains(&map.0 .0));
    for (spawner, entity_store, aabb) in inserted_grass.iter() {
        let id = entity_store.0;
        if spawner.flags.contains(GrassSpawnerFlags::HEIGHT_MAP) {
            let handle = &spawner.height_map.as_ref().unwrap().height_map;
            if images.get(handle).is_none() {
                local_height_map_buffer.push((entity_store.clone(), handle.clone(), *aabb));
            }
        }
        let (height_map_texture, aabb_buffer) =
            if !spawner.flags.contains(GrassSpawnerFlags::HEIGHT_MAP) {
                let height_map_texture = &fallback_img.texture_view;
                let aabb_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("aabb buffer"),
                    contents: &[0],
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
                (height_map_texture, aabb_buffer)
            } else {
                let handle = spawner.height_map.as_ref().unwrap().height_map.clone();
                let height_map_texture = if let Some(tex) = images.get(&handle) {
                    &tex.texture_view
                } else {
                    &fallback_img.texture_view
                };

                let aabb_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
                    label: Some("aabb buffer"),
                    contents: bytemuck::bytes_of(&aabb.half_extents.mul(2.).as_dvec3().as_vec3()),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
                (height_map_texture, aabb_buffer)
            };
        let layout = pipeline.height_map_layout.clone();

        let bind_group_descriptor = BindGroupDescriptor {
            label: Some("grass height map bind group"),
            layout: &layout,
            entries: &[
                BindGroupEntry {
                    binding: 0,
                    resource: BindingResource::TextureView(height_map_texture),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: BindingResource::Buffer(BufferBinding {
                        buffer: &aabb_buffer,
                        offset: 0,
                        size: None,
                    }),
                },
            ],
        };

        let bind_group = render_device.create_bind_group(&bind_group_descriptor);
        if let Some(chunk) = cache.get_mut(&id) {
            chunk.height_map = Some(bind_group);
        } else {
            warn!("Tried to prepare a buffer for a grass chunk which wasn't registered before");
        }
    }
}
pub(crate) fn prepare_uniform_buffers(
    pipeline: Res<GrassPipeline>,
    mut cache: ResMut<GrassCache>,
    region_config: Res<GrassConfiguration>,
    fallback_img: Res<FallbackImage>,
    render_device: Res<RenderDevice>,
    images: Res<RenderAssets<Image>>,
    mut last_texture_id: Local<Option<TextureViewId>>,
) {
    let texture = &images
        .get(&region_config.wind_noise_texture)
        .unwrap_or(&fallback_img)
        .texture_view;
    if !region_config.is_changed() && Some(texture.id()) == *last_texture_id && !cache.is_changed()
    {
        return;
    }
    *last_texture_id = Some(texture.id());

    let shader_config = ShaderRegionConfiguration::from(region_config.as_ref());
    let config_buffer = render_device.create_buffer_with_data(&BufferInitDescriptor {
        label: Some("region config buffer"),
        contents: bytemuck::bytes_of(&shader_config),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let layout = pipeline.region_layout.clone();
    let bind_group_descriptor = BindGroupDescriptor {
        label: Some("grass uniform bind group"),
        layout: &layout,
        entries: &[
            BindGroupEntry {
                binding: 0,
                resource: BindingResource::Buffer(BufferBinding {
                    buffer: &config_buffer,
                    offset: 0,
                    size: None,
                }),
            },
            BindGroupEntry {
                binding: 1,
                resource: BindingResource::TextureView(texture),
            },
        ],
    };
    let bind_group = render_device.create_bind_group(&bind_group_descriptor);

    for instance_data in cache.values_mut() {
        instance_data.uniform_bindgroup = Some(bind_group.clone());
    }
}

#[derive(Debug, Clone, Copy, Pod, Zeroable, ShaderType)]
#[repr(C)]
struct ShaderRegionConfiguration {
    main_color: Vec4,
    bottom_color: Vec4,
    wind: Vec2,
    /// Wasm requires shader uniforms to be aligned to 16 bytes
    _wasm_padding: Vec2,
}

impl From<&GrassConfiguration> for ShaderRegionConfiguration {
    fn from(config: &GrassConfiguration) -> Self {
        Self {
            main_color: config.main_color.into(),
            bottom_color: config.bottom_color.into(),
            wind: config.wind,
            _wasm_padding: Vec2::ZERO,
        }
    }
}
fn prepare_texture_from_data<T: Default + Clone + bytemuck::Pod>(
    data: &mut Vec<T>,
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
    format: TextureFormat,
) -> TextureView {
    let device = render_device.wgpu_device();

    // the dimensions of the texture are choosen to be nxn for the tiniest n which can contain the data
    let sqrt = (data.len() as f32).sqrt() as u32 + 1;
    let fill_data = vec![T::default(); (sqrt * sqrt) as usize - data.len()];
    data.extend(fill_data);
    let texture_size = Extent3d {
        width: sqrt,
        height: sqrt,
        depth_or_array_layers: 1,
    };
    // wgpu expects a byte array
    let data_slice = bytemuck::cast_slice(data.as_slice());
    // the texture is empty per default
    let texture = device.create_texture(&TextureDescriptor {
        size: texture_size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        label: None,
        view_formats: &[],
    });
    let t_size = mem::size_of::<T>();

    // write data to texture
    render_queue.write_texture(
        ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        },
        data_slice,
        ImageDataLayout {
            offset: 0,
            bytes_per_row: NonZeroU32::new(t_size as u32 * texture_size.width),
            rows_per_image: NonZeroU32::new(texture_size.height),
        },
        texture_size,
    );
    texture
        .create_view(&TextureViewDescriptor {
            label: None,
            format: Some(format),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: NonZeroU32::new(1),
            base_array_layer: 0,
            array_layer_count: NonZeroU32::new(1),
        })
        .into()
}
