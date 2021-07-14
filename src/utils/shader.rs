use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::ptr;

use ash::version::DeviceV1_0;
use ash::vk;
use ash::vk::{DescriptorSetLayout, DescriptorSetLayoutBinding};
use spirv_reflect::ShaderModule;
use spirv_reflect::types::{ReflectDescriptorType, ReflectShaderStageFlags};

pub struct Shader {
    shader_module: vk::ShaderModule,

    // map[set][binding] = DescriptorSetLayoutBinding
    descriptor_sets: HashMap<u32, HashMap<u32, DescriptorSetLayoutBinding>>,
    entry_point_name: String,

    stage_flags: vk::ShaderStageFlags,
}

fn get_shader_stage_flags(flags: ReflectShaderStageFlags) -> vk::ShaderStageFlags {
    let mapping = [
        (ReflectShaderStageFlags::VERTEX, vk::ShaderStageFlags::VERTEX),
        (ReflectShaderStageFlags::FRAGMENT, vk::ShaderStageFlags::FRAGMENT),
        (ReflectShaderStageFlags::TESSELLATION_CONTROL, vk::ShaderStageFlags::TESSELLATION_CONTROL),
        (ReflectShaderStageFlags::TESSELLATION_EVALUATION, vk::ShaderStageFlags::TESSELLATION_EVALUATION),
        (ReflectShaderStageFlags::GEOMETRY, vk::ShaderStageFlags::GEOMETRY),
        (ReflectShaderStageFlags::FRAGMENT, vk::ShaderStageFlags::FRAGMENT),
        (ReflectShaderStageFlags::COMPUTE, vk::ShaderStageFlags::COMPUTE),
    ];

    let mut ret: vk::ShaderStageFlags = vk::ShaderStageFlags::empty();
    for (reflected, target) in mapping {
        if flags.contains(reflected) {
            ret |= target;
        }
    }

    ret
}

fn get_descriptor_type(reflected_type: ReflectDescriptorType) -> Option<vk::DescriptorType> {
    let mapping = [
        (ReflectDescriptorType::Sampler, vk::DescriptorType::SAMPLER),
        (ReflectDescriptorType::CombinedImageSampler, vk::DescriptorType::COMBINED_IMAGE_SAMPLER),
        (ReflectDescriptorType::SampledImage, vk::DescriptorType::SAMPLED_IMAGE),
        (ReflectDescriptorType::StorageImage, vk::DescriptorType::STORAGE_IMAGE),
        (ReflectDescriptorType::UniformTexelBuffer, vk::DescriptorType::UNIFORM_TEXEL_BUFFER),
        (ReflectDescriptorType::StorageTexelBuffer, vk::DescriptorType::STORAGE_TEXEL_BUFFER),
        (ReflectDescriptorType::UniformBuffer, vk::DescriptorType::UNIFORM_BUFFER),
        (ReflectDescriptorType::StorageBuffer, vk::DescriptorType::STORAGE_BUFFER),
        (ReflectDescriptorType::UniformBufferDynamic, vk::DescriptorType::UNIFORM_BUFFER_DYNAMIC),
        (ReflectDescriptorType::StorageBufferDynamic, vk::DescriptorType::STORAGE_BUFFER_DYNAMIC),
        (ReflectDescriptorType::InputAttachment, vk::DescriptorType::INPUT_ATTACHMENT),
        (ReflectDescriptorType::AccelerationStructureNV, vk::DescriptorType::ACCELERATION_STRUCTURE_NV),
    ];

    for (reflected, target) in mapping {
        if reflected == reflected_type {
            return Some(target);
        }
    }
    None
}

impl Shader {
    pub fn load(device: &ash::Device, path: &str) -> Shader {
        let spv_file = File::open(path)
            .expect(&format!("Failed to find spv file at {:?}", path));

        let code: Vec<u8> = spv_file.bytes().map(
            |byte| byte.unwrap()
        ).collect();

        let module = ShaderModule::load_u8_data(&code).unwrap();
        let reflected_descriptor_sets = module.enumerate_descriptor_sets(None).unwrap();
        let shader_stage_flags = get_shader_stage_flags(module.get_shader_stage());

        let mut sets = HashMap::<u32, HashMap<u32, DescriptorSetLayoutBinding>>::new();
        for ref_set in reflected_descriptor_sets.iter() {
            if !sets.contains_key(&ref_set.set) {
                sets.insert(ref_set.set, HashMap::<u32, vk::DescriptorSetLayoutBinding>::new());
            }

            let layout_bindings = sets.get_mut(&ref_set.set).unwrap();
            for ref_binding in ref_set.bindings.iter() {
                if layout_bindings.contains_key(&ref_binding.binding) {
                    panic!("Descriptor set {} already contains binding {}", ref_set.set,
                           ref_binding.binding);
                }

                layout_bindings.insert(
                    ref_binding.binding,
                    DescriptorSetLayoutBinding {
                        binding: ref_binding.binding,
                        descriptor_type: get_descriptor_type(ref_binding.descriptor_type).unwrap(),
                        descriptor_count: ref_binding.count,
                        stage_flags: shader_stage_flags,
                        p_immutable_samplers: ptr::null(),
                    },
                );
            }
        }

        let shader_module_create_info = vk::ShaderModuleCreateInfo {
            s_type: vk::StructureType::SHADER_MODULE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::ShaderModuleCreateFlags::empty(),
            code_size: code.len(),
            p_code: code.as_ptr() as *const u32,
        };

        let shader_module = unsafe {
            device
                .create_shader_module(&shader_module_create_info, None)
                .expect("Failed to create Shader Module!")
        };

        Shader {
            shader_module,
            descriptor_sets: sets,
            entry_point_name: module.get_entry_point_name(),
            stage_flags: shader_stage_flags,
        }
    }
}

fn merge_layout_bindings(shaders: Vec<&Shader>) -> Vec<Vec<DescriptorSetLayoutBinding>> {
    let mut total_sets = HashMap::<u32, HashMap<u32, DescriptorSetLayoutBinding>>::new();

    for shader in shaders {
        for (&set, shader_bindings) in shader.descriptor_sets.iter() {
            let target_bindings = total_sets.entry(set)
                .or_insert(HashMap::new());

            for (_, &shader_binding) in shader_bindings.iter() {
                if target_bindings.contains_key(&shader_binding.binding) {
                    panic!("Descriptor sets merge failed: binding {} in descriptor set {} already exists", set, shader_binding.binding);
                }

                target_bindings.insert(shader_binding.binding, shader_binding);
            }
        }
    }


    let mut sorted_sets: Vec<_> = total_sets.into_iter().collect();
    sorted_sets.sort_by(|x, y| x.0.cmp(&y.0));

    let mut ret = Vec::<Vec<DescriptorSetLayoutBinding>>::new();
    for (set, bindings) in sorted_sets {
        println!("set {}", set);
        let ret_bindings: Vec<_> = bindings.values().copied().collect();

        ret.push(ret_bindings);
    }

    ret
}

pub fn create_descriptor_set_layout(device: &ash::Device, shaders: Vec<&Shader>) -> Vec<vk::DescriptorSetLayout> {
    let mut total_sets = merge_layout_bindings(shaders);

    let mut ret_layouts = Vec::<DescriptorSetLayout>::new();
    for bindings in total_sets {
        let descriptor_layout_create_info = vk::DescriptorSetLayoutCreateInfo {
            s_type: vk::StructureType::DESCRIPTOR_SET_LAYOUT_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::DescriptorSetLayoutCreateFlags::empty(),
            binding_count: bindings.len() as u32,
            p_bindings: bindings.as_ptr(),
        };

        let layout = unsafe {
            device
                .create_descriptor_set_layout(&descriptor_layout_create_info, None)
                .expect("Failed to create Descriptor Set Layout!")
        };
        ret_layouts.push(layout);
    }

    ret_layouts
}
