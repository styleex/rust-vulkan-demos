rust vulkan tests (with ash)
============================

Plan:

* Some tools over ash for programming with Vulkan API:
    
    * (OK) Render env for central storage for main ASH objects (entry, instance, device, surface, pools etc.)
      
    * (OK) Runtime shader loading with SPIRV reflection ([spirv-reflect github](https://github.com/KhronosGroup/SPIRV-Reflect)) for descriptor layout sets creation
    
    * (OK) FPS camera for world view 
    
    * (OK) Framebuffer + attachment image for simplifying offscreen buffer creation.

* (OK) base skybox
  
* (OK) egui integration 
  
* Deferred shading pipeline 

* HDRR pipeline

* (partial) Shadow mapping pipeline



# Requirements

* Linux or MacOS
* [Vulkan SDK](https://vulkan.lunarg.com/doc/view/1.1.126.0/linux/getting_started.html#user-content-download-and-install-packages-for-building-binaries)
* rust stable
* glslc
* python 3.8 (compile_shaders.py)

# Run:

> ./download_assets.py
> 
> ./compile_shaders.py
> 
> cargo run --package ash-test --bin ash-test
