@echo off
cd /d "%~dp0"

cargo build --release

mkdir ffengine_build

copy "target\release\ffengine.exe" "ffengine_build\"
copy "target\release\ffeditor.exe" "ffengine_build\"
xcopy "engine\resources" "ffengine_build\engine\resources" /E /I /Y
xcopy "editor\resources" "ffengine_build\editor\resources" /E /I /Y

echo Build package created in ffengine_build\
pause