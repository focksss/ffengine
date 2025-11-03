# ffengine
A W.I.P. game engine written in rust and using Vulkan through the Ash crate. Goal is support of basic game engine features and allowing for extremely customizable rendering pipelines through high level abstractions.

## Current Features
- Deferred rendering pipeline
  - Geometry pass
  - CSM shadow pass
  - SSAO generation + geometry-aware bilateral upscale
  - Lighting pass
- Full vulkan abstraction structs (limited builder options)
- Fully functional custom .gui file format
  - Parser
  - Specification
  - Corresponding GUI struct with rendering methods
- gLTF loader
- Scene system
  - Skeletal animation
  - Runtime model loading + transforming
  - Light system
- MSDF text rendering, including efficiently updating text per frame

## Features In Development
- GUI-node-based runtime rendering pipeline editor (long term)
- Full PBR rendering preset
- Physics engine (auto-gen world hitboxes, player controller, other basic features)
