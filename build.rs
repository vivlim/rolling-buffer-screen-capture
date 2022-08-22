pub fn main() {
    let mut config = vcpkg::Config::new();
    config.target_triplet("x64-windows-static-md");
    config.find_package("ffmpeg").unwrap();
    config.find_package("libjpeg-turbo").unwrap();


    // https://github.com/zmwangx/rust-ffmpeg-sys/issues/28
    println!("cargo:rustc-link-lib=crypt32");
    println!("cargo:rustc-link-lib=Setupapi");
    println!("cargo:rustc-link-lib=winmm");
    println!("cargo:rustc-link-lib=Imm32");
    println!("cargo:rustc-link-lib=Version");
    println!("cargo:rustc-link-lib=mfplat");
    println!("cargo:rustc-link-lib=strmiids");
    println!("cargo:rustc-link-lib=mfuuid");
    println!("cargo:rustc-link-lib=Vfw32");
}
