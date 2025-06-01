use std::env;

fn main() {
    if let Some(lib_path) = std::env::var_os("DEP_TCH_LIBTORCH_LIB") {
        println!(
            "cargo:rustc-link-arg=-Wl,-rpath={}",
            lib_path.to_string_lossy()
        );
    }
    println!("cargo:rustc-link-arg=-Wl,--no-as-needed");
    println!("cargo:rustc-link-arg=-Wl,--copy-dt-needed-entries");
    println!("cargo:rustc-link-arg=-ltorch");

    if std::env::var("TARGET").unwrap() == "i686-pc-windows-gnu" {
        let base_openslide_dir = env::current_dir().unwrap().join("deps/openslide-win32");
        let openslide_dll_dir = base_openslide_dir.join("bin");
        let python_dll_dir = env::current_dir().unwrap().join("deps/python3.11-win32");
        // Print cargo metadata for linking
        //println!("cargo:rerun-if-changed=deps/openslide-win32");
        // Print cargo metadata for including headers
        //let include_dir = base_openslide_dir.join("include/openslide");
        //println!("cargo:include={}", include_dir.display());
        println!(
            "cargo:rustc-link-search=native={}",
            openslide_dll_dir.display()
        );
        println!(
            "cargo:rustc-link-search=native={}",
            python_dll_dir.display()
        );
        println!("cargo:rustc-link-lib=dylib=openslide");
        println!("cargo:rustc-link-lib=dylib=python311");
    }
    if std::env::var("TARGET").unwrap() == "x86_64-pc-windows-gnu" {
        let base_openslide_dir = env::current_dir().unwrap().join("deps/openslide-win64");
        let openslide_dll_dir = base_openslide_dir.join("bin");
        let python_dll_dir = env::current_dir().unwrap().join("deps/python3.11-win64");
        // Print cargo metadata for linking
        //println!("cargo:rerun-if-changed=deps/openslide-win32");
        // Print cargo metadata for including headers
        //let include_dir = base_openslide_dir.join("include/openslide");
        //println!("cargo:include={}", include_dir.display());
        println!(
            "cargo:rustc-link-search=native={}",
            openslide_dll_dir.display()
        );
        println!(
            "cargo:rustc-link-search=native={}",
            python_dll_dir.display()
        );
        println!("cargo:rustc-link-lib=dylib=openslide");
        println!("cargo:rustc-link-lib=dylib=python311");
    }
    if std::env::var("TARGET").unwrap() == "x86_64-pc-windows-msvc" {
        let openslide_dll_dir = env::current_dir().unwrap().join("deps/openslide-win64/bin");
        let vips_dll_dir = env::current_dir().unwrap().join("deps/vips-dev-8.15_w64/bin");
        let torch_dll_dir = env::current_dir().unwrap().join("deps/libtorch_-2.7.0_w64/lib");
        println!(
            "cargo:rustc-link-search=native={}",
            openslide_dll_dir.display()
        );
        println!(
            "cargo:rustc-link-search=native={}",
            vips_dll_dir.display()
        );
        println!(
            "cargo:rustc-link-search=native={}",
            torch_dll_dir.display()
        );
        //println!("cargo:rustc-link-lib=dylib=libopenslide");
        //println!("cargo:rustc-link-lib=dylib=libvips");
    }
}
