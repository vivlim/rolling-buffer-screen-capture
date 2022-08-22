# rolling-buffer-screen-capture

utility which, while running, will attempt to capture the last 30 seconds of your primary display.

press shift+super+R while running and it will write an mp4 of the available frames to the current directory.

## building

you need to have libjpeg-turbo and ffmpeg available to build.

### windows

0. open a developer powershell :)
1. install [vcpkg](https://vcpkg.io/en/getting-started.html) and run `vcpkg integrate install`
2. `cargo install cargo-vcpkg`
3. `cargo vcpkg build`
4. in powershell, `$env:CARGO_FEATURE_STATIC=1` because otherwise ffmpeg tries to link dynamically
4. `cargo build --release`
