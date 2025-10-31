@echo off
cd /d "%~dp0"

cargo build --release

mkdir ffengine_build

copy "target\release\ffengine.exe" "ffengine_build\"
xcopy "resources" "ffengine_build\resources" /E /I /Y

echo Build package created in ffengine_build\
pause