@echo off
cargo build --release
mkdir ffengine_build
copy target\release\ffengine.exe ffengine_build\
xcopy src\shaders\spv ffengine_build\shaders\spv /E /I /Y