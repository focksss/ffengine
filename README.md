# ffengine
A W.I.P. game engine written in rust and using Vulkan through the Ash crate, with Lua scripting support. Goal is support of basic game engine features and allowing for extremely customizable rendering pipelines through high level abstractions.

## Current Features
- Scripting
  - API exposing the full engine to lua (expanded as needed)
  - lifecycle functions (Update, Awake, Start)
- Scene editor
  - Written entirely using the scripting system 
  - Scene graph
  - Component display and editor (currently transforms + render components)
- Deferred+Forward rendering pipeline
  - Geometry pass
  - CSM shadow pass
  - SSAO generation + geometry-aware bilateral upscale
  - Lighting pass
  - Forward support to allow for transparent materials, materials with custom shaders, and more.
- Full vulkan abstraction structs (builder options expanded as needed)
- Fully functional custom .gui file format
  - Parser
  - Specification
  - Corresponding GUI struct with rendering/action-handler methods
  - Hierarchy-based
    - Allows for nested definitions or indexed references 
- gLTF parser
- Scene system
  - Skeletal animation
  - Runtime model loading + transforming
  - Light system
  - Ability to link scene nodes to physics bodies
  - (Mostly) ECS architecture, allowing for complete control over the scene in lua scripts
- MSDF text rendering, including efficiently updating text per frame
- Physics engine
  - Convex hull, OBB, capsule, mesh, and sphere hitboxes
  - Collision detection with manifolds
  - Collision resolution
  - Newtonian physics tick integration

## Features In Development
- GUI-node-based runtime rendering pipeline editor (long term)
- Full PBR rendering preset
- Physics engine (finishing support for all collision cases)
- Support for non-deferred-pipeline materials with custom shaders included in scene structure
- Scene editor
