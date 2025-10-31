# ffengine
A W.I.P. game engine written in rust and using Vulkan through the Ash crate. Goal is support of basic game engine features and allowing for extremely customizable rendering pipelines through high level abstractions.


## Current Target Features
- Full vulkan abstractions, with builder options exposing all features
- Custom GUI
- Full PBR rendering preset
- Proper screenshotting support
- Physics engine (auto-gen world hitboxes, player controller, other basic features)
## Current Features
- Full vulkan abstraction structs (limited builder options)
- gLTF loader
- Full Scene abstraction allowing for easy runtime loading of models, skeletal animation, and management of models.
- MSDF text rendering, including efficiently updating text per frame.
- SSAO
- Geometry pass
- Frames in flight
- Frametime profiler
