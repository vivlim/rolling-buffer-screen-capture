# rolling-buffer-screen-capture

utility which, while running, will attempt to capture the last 30 seconds of your primary display.

press shift+super+R while running and it will dump a lot of jpegs into a folder.

## building

you need to have libjpeg-turbo available to build.

### windows

1. install https://sourceforge.net/projects/libjpeg-turbo/files/2.1.1/libjpeg-turbo-2.1.1-vc64.exe/download (or newer from [libjpeg-turbo's website](https://libjpeg-turbo.org/))
2. copy turbojpeg.lib into the current directory, i.e. `cp "C:\libjpeg-turbo64\lib\turbojpeg.lib" .`