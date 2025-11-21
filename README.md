# ffengine
A W.I.P. game engine written in rust and using Vulkan through the Ash crate, with Lua scripting support. Goal is support of basic game engine features and allowing for extremely customizable rendering pipelines through high level abstractions.

## Current Features
- Scripting
  - API exposing the engine to lua (expanded as needed)
  - lifecycle functions (Update, Awake, Start)
- Deferred rendering pipeline
  - Geometry pass
  - CSM shadow pass
  - SSAO generation + geometry-aware bilateral upscale
  - Lighting pass
- Full vulkan abstraction structs (limited builder options)
- Fully functional custom .gui file format
  - Parser
  - Specification
  - Corresponding GUI struct with rendering/action-handler methods
- gLTF parser
- Scene system
  - Skeletal animation
  - Runtime model loading + transforming
  - Light system
  - Ability to link scene nodes to physics bodies
- MSDF text rendering, including efficiently updating text per frame
- Physics engine 
  - OBB, capsule, mesh, and sphere hitboxes
  - Collision detection with manifolds
  - Collision resolution
  - Newtonian physics tick integration

## Features In Development
- GUI-node-based runtime rendering pipeline editor (long term)
- Full PBR rendering preset
- Physics engine (finishing support for all collision cases)
- Scene editor
