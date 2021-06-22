use anyhow::*;
use half::f16;
use image::GenericImageView;
use std::num::NonZeroU32;
use wgpu::*;

pub struct Tex {
    pub texture: Texture,
    pub view: TextureView,
    pub sampler: Sampler,
    pub format: TextureFormat,
}

impl Tex {
    pub const DEPTH_FORMAT: TextureFormat = TextureFormat::Depth32Float; // need when creating render pipeline depth stage and create texture

    pub fn from_bytes(device: &Device, queue: &Queue, bytes: &[u8], label: &str) -> Result<Self> {
        let img = image::load_from_memory(bytes)?;
        Self::from_image(device, queue, &img, Some(label))
    }

    pub fn create_1d_texture_rgba8(
        data: &Vec<cgmath::Vector4<u8>>,
        device: &Device,
        queue: &Queue,
        label: &str,
    ) -> Self {
        let format = TextureFormat::Rgba8UnormSrgb;
        let length = data.len() as u32;
        let flatten_data = data
            .iter()
            .flat_map(|v| vec![v.x, v.y, v.z, v.w])
            .collect::<Vec<u8>>();
        let size = Extent3d {
            width: length,
            height: 1,
            depth_or_array_layers: 1,
        };
        let desc = TextureDescriptor {
            label: Some(label),
            size: size.clone(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D1,
            format,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        };
        let texture = device.create_texture(&desc);
        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            bytemuck::cast_slice(flatten_data.as_slice()),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(length * 4).unwrap()),
                rows_per_image: Some(NonZeroU32::new(1).unwrap()),
            },
            size.clone(),
        );
        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        Tex {
            texture,
            view,
            sampler,
            format,
        }
    }

    pub fn create_3d_texture_red_f16(
        size: &Extent3d,
        data: &Vec<f16>,
        device: &Device,
        queue: &Queue,
        label: &str,
    ) -> Self {
        let format = TextureFormat::R16Float;
        let desc = TextureDescriptor {
            label: Some(label),
            size: size.clone(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D3,
            format,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        };
        let texture = device.create_texture(&desc);
        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            bytemuck::cast_slice(data.as_slice()),
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(2 * size.width).unwrap()),
                rows_per_image: Some(NonZeroU32::new(size.height).unwrap()),
            },
            size.clone(),
        );
        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Tex {
            texture,
            view,
            sampler,
            format,
        }
    }

    pub fn create_depth_texture(device: &Device, width: u32, height: u32, label: &str) -> Self {
        let size = Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let format = Self::DEPTH_FORMAT;
        let desc = TextureDescriptor {
            label: Some(label),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: format.clone(),
            usage: TextureUsage::RENDER_ATTACHMENT | TextureUsage::SAMPLED,
        };
        let texture = device.create_texture(&desc);
        let view = texture.create_view(&TextureViewDescriptor::default());
        // We technically don't need a sampler for a depth texture,
        // but our Texture struct requires it, and we need one if we ever want to sample it.
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            compare: Some(CompareFunction::LessEqual), // highlight: If we do decide to render our depth texture, we need to use CompareFunction::LessEqual. This is due to how the samplerShadow and sampler2DShadow() interacts with the texture() function in GLSL
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            ..Default::default()
        });
        Self {
            texture,
            view,
            sampler,
            format,
        }
    }

    pub fn create_render_buffer(
        dimensions: (u32, u32),
        device: &Device,
        label: Option<&str>,
    ) -> Self {
        let size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let format = TextureFormat::Rgba16Float;
        let texture = device.create_texture(&TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: format.clone(),
            usage: TextureUsage::SAMPLED | TextureUsage::RENDER_ATTACHMENT,
        });
        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });
        Self {
            texture,
            view,
            format,
            sampler,
        }
    }

    pub fn from_image(
        device: &Device,
        queue: &Queue,
        img: &image::DynamicImage,
        label: Option<&str>,
    ) -> Result<Self> {
        let rgba = img.as_rgba8().unwrap();
        let dimensions = img.dimensions();

        let size = Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1, //HIGHLIGHT: why?
        };
        let format = TextureFormat::Rgba8UnormSrgb;
        let texture = device.create_texture(&TextureDescriptor {
            label,
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: format.clone(),
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        });

        queue.write_texture(
            ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: Origin3d::ZERO,
            },
            rgba,
            ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(NonZeroU32::new(4 * dimensions.0).unwrap()),
                rows_per_image: Some(NonZeroU32::new(dimensions.1).unwrap()),
            },
            size,
        );

        let view = texture.create_view(&TextureViewDescriptor::default());
        let sampler = device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            ..Default::default()
        });

        Ok(Self {
            texture,
            view,
            format,
            sampler,
        })
    }
}
