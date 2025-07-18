use core::mem;
use std::{ffi, ptr};
use std::collections::HashMap;
use std::ffi::{CString};
use std::fs::File;
use std::io::Read;

use ash::version::DeviceV1_0;
use ash::vk;
use ash::vk::DescriptorSetLayoutBinding;
use spirv_reflect::ShaderModule;
use spirv_reflect::types::{ReflectDescriptorType, ReflectShaderStageFlags};


pub trait SpecializationConstants {
    fn entry_map() -> Vec<vk::SpecializationMapEntry>;
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
        (ReflectDescriptorType::AccelerationStructureKHR, vk::DescriptorType::ACCELERATION_STRUCTURE_NV),
    ];

    for (reflected, target) in mapping {
        if reflected == reflected_type {
            return Some(target);
        }
    }
    None
}

pub struct ConstantsBuilder {
    cur_constant: u32,
    cur_offset: u32,
    data: Vec<u8>,
    entry_map: Vec<vk::SpecializationMapEntry>,
}

impl ConstantsBuilder {
    pub fn new() -> ConstantsBuilder {
        ConstantsBuilder {
            cur_constant: 0,
            cur_offset: 0,
            data: vec![],
            entry_map: vec![],
        }
    }

    pub fn add_u32(mut self, val: u32) -> Self {
        let size = mem::size_of_val(&val);

        self.entry_map.push(
            vk::SpecializationMapEntry {
                constant_id: self.cur_constant,
                offset: self.cur_offset,
                size,
            }
        );

        self.cur_constant += 1;
        self.cur_offset += size as u32;
        self.data.extend(val.to_le_bytes());

        self
    }
}

pub struct Shader {
    device: ash::Device,
    shader_module: vk::ShaderModule,

    // descriptor_sets[set][binding] = DescriptorSetLayoutBinding
    descriptor_sets: HashMap<u32, HashMap<u32, DescriptorSetLayoutBinding>>,
    entry_point_name: CString,

    stage_flags: vk::ShaderStageFlags,

    constants: Option<ConstantsBuilder>,
    spec_info: Option<vk::SpecializationInfo>,
    pub push_constants_range: vk::PushConstantRange,
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

        let mut push_constants_range = vk::PushConstantRange {
            stage_flags: shader_stage_flags,
            offset: 0,
            size: 0
        };

        for block in module.enumerate_push_constant_blocks(None) {
            for var in block.iter() {
                push_constants_range.offset = var.offset.min(push_constants_range.offset);
                push_constants_range.size += var.size;
            }
        }

        Shader {
            shader_module,
            descriptor_sets: sets,
            entry_point_name: CString::new(module.get_entry_point_name()).unwrap(),
            stage_flags: shader_stage_flags,
            device: device.clone(),
            constants: None,
            spec_info: None,
            push_constants_range,
        }
    }

    pub fn specialize(mut self, constants: ConstantsBuilder) -> Shader{
        self.constants = Some(constants);

        let const_ref = self.constants.as_ref().unwrap();
        self.spec_info = Some(
            vk::SpecializationInfo {
                map_entry_count: const_ref.entry_map.len() as u32,
                p_map_entries: const_ref.entry_map.as_ptr(),
                data_size: const_ref.data.len(),
                p_data: const_ref.data.as_ptr() as *const _ as *const ffi::c_void,
            }
        );

        self
    }

    pub fn stage(&self) -> vk::PipelineShaderStageCreateInfo {
        if self.constants.is_none() {
            return vk::PipelineShaderStageCreateInfo {
                s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
                p_next: ptr::null(),
                flags: vk::PipelineShaderStageCreateFlags::empty(),
                module: self.shader_module,
                p_name: self.entry_point_name.as_ptr(),
                p_specialization_info: ptr::null(),
                stage: self.stage_flags,
            };
        };

        vk::PipelineShaderStageCreateInfo {
            s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
            p_next: ptr::null(),
            flags: vk::PipelineShaderStageCreateFlags::empty(),
            module: self.shader_module,
            p_name: self.entry_point_name.as_ptr(),
            p_specialization_info: self.spec_info.as_ref().unwrap(),
            stage: self.stage_flags,
        }
    }
}

impl Drop for Shader {
    fn drop(&mut self) {
        unsafe {
            self.device.destroy_shader_module(self.shader_module, None);
        }
    }
}


// mutual exclusive merge bindings of sets
fn _merge_layout_bindings(shaders: Vec<&Shader>) -> Vec<Vec<DescriptorSetLayoutBinding>> {
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


    // sort by SET number in asc order
    let mut sorted_sets: Vec<_> = total_sets.into_iter().collect();
    sorted_sets.sort_by(|x, y| x.0.cmp(&y.0));

    // convert hashmap to vector
    let mut ret = Vec::<Vec<DescriptorSetLayoutBinding>>::new();
    for (_set, bindings) in sorted_sets {
        let mut ret_bindings: Vec<_> = bindings.values().copied().collect();
        ret_bindings.sort_by(|x, y| x.binding.cmp(&y.binding));

        ret.push(ret_bindings);
    }

    ret
}

pub struct DescriptorSetLayout {
    pub layout: vk::DescriptorSetLayout,
    pub(super) binding_desc: Vec<vk::DescriptorSetLayoutBinding>,
}

// Merge descriptor information from shaders into general list of descriptor set layout
// (set = 0, binding = 0) + (set = 1, binding = 1) = Vec<vk::DescriptorSetLayout>.len() == 2;
pub fn create_descriptor_set_layout(device: &ash::Device, shaders: Vec<&Shader>) -> Vec<DescriptorSetLayout> {
    let total_sets = _merge_layout_bindings(shaders);

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

        ret_layouts.push(
            DescriptorSetLayout {
                layout,
                binding_desc: bindings,
            }
        );
    }

    ret_layouts
}
