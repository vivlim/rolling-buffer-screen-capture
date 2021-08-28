# rolling-buffer-screen-capture

utility which, while running, will attempt to capture the last 30 seconds of your primary display.

press shift+super+R while running and it will write an mp4 of the available frames to the current directory.

## building

you need to have libjpeg-turbo and ffmpeg available to build.

### windows

1. install [vcpkg](https://vcpkg.io/en/getting-started.html)
2. install `ffmpeg` via vcpkg
3. install https://sourceforge.net/projects/libjpeg-turbo/files/2.1.1/libjpeg-turbo-2.1.1-vc64.exe/download (or newer from [libjpeg-turbo's website](https://libjpeg-turbo.org/)) (todo: see if this can be installed via vcpkg too)
4. copy turbojpeg.lib into the current directory, i.e. `cp "C:\libjpeg-turbo64\lib\turbojpeg.lib" .`
