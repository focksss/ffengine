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
  - Corresponding GUI struct with rendering/action-handler methods
- gLTF parser
- Scene system
  - Skeletal animation
  - Runtime model loading + transforming
  - Light system
  - Ability to link scene nodes to physics bodies
- MSDF text rendering, including efficiently updating text per frame
- Physics engine
  - `bodyCast() -> Option<CastInformation>` function, capable of "ray" casting any body with any hitbox against all other physics objects
  - OBB, capsule, and mesh hitboxes
  - OBB-capsule, OBB-OBB collision detection
  - Player collision resolution

## Features In Development
- GUI-node-based runtime rendering pipeline editor (long term)
- Full PBR rendering preset
- Physics engine (auto-gen world hitboxes, player controller, other basic features)
