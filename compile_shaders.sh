#!/bin/bash
mkdir -p shaders/spv
glslc -o shaders/spv/09-shader-base.vert.spv shaders/src/09-shader-base.vert
glslc -o shaders/spv/09-shader-base.frag.spv shaders/src/09-shader-base.frag
glslc -o shaders/spv/compose.vert.spv shaders/src/compose.vert
glslc -o shaders/spv/compose.frag.spv shaders/src/compose.frag
