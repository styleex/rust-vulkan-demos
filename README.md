rust vulkan tests (with ash)
============================

Plan:

* Some tools over ash for programming with Vulkan API:
    
    * (OK) Render env for central storage for main ASH objects (entry, instance, device, surface, pools etc.)
      
    * (OK) Runtime shader loading with SPIRV reflection ([spirv-reflect github](https://github.com/KhronosGroup/SPIRV-Reflect)) for descriptor layout sets creation
    
    * (OK) FPS camera for world view 
    
    * Framebuffer + attachment image for simplifying offscreen buffer creation.

* Deferred shading pipeline 

* HDRR pipeline

* Shadow mapping pipeline



# Requirements

* Linux or MacOS
* vulkan sdk
* rust stable
* glslc

# Run:

> ./download_assets.py
> 
> ./compile_shaders.sh
> 
> cargo run 
