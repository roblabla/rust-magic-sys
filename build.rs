fn env(name: &str) -> Option<std::ffi::OsString> {
    let target = std::env::var("TARGET").expect("Cargo didn't provide `TARGET` environment var");
    let target = target.to_uppercase().replace("-", "_");
    let prefixed_name = format!("{}_{}", target, name);
    println!("cargo:rerun-if-env-changed={}", prefixed_name);
    match std::env::var_os(prefixed_name) {
        Some(v) => Some(v),
        None => {
            println!("cargo:rerun-if-env-changed={}", name);
            std::env::var_os(name)
        }
    }
}

#[cfg(feature = "bundled")]
fn try_bundled() {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = std::path::Path::new(&out_dir);
    let target_vendor = std::env::var("CARGO_CFG_TARGET_VENDOR").unwrap();
    let include_dir = out_dir.join("include");

    // First, copy magic.h.in into out_dir/include/magic.h, replacing the X.YY
    // string with the actual versionversion.
    std::fs::create_dir_all(&include_dir).unwrap();
    let mut data = std::fs::read_to_string("file/src/magic.h.in").unwrap();
    data = data.replace("X.YY", "5.45");
    std::fs::write(include_dir.join("magic.h"), &data).unwrap();
    std::fs::write(include_dir.join("forcestrlcpyweak.h"), "#pragma weak strlcpy").unwrap();

    let mut build = cc::Build::new();

    build
        .include("file/src")
        .include(&include_dir)
        .flag(&format!("-include{}", include_dir.join("forcestrlcpyweak.h").display()))
        .define("HAVE_UNISTD_H", "1")
        .define("HAVE_INTTYPES_H", "1")
        .define("VERSION", "5.45")
        .file("file/src/buffer.c")
        .file("file/src/magic.c")
        .file("file/src/apprentice.c")
        .file("file/src/softmagic.c")
        .file("file/src/ascmagic.c")
        .file("file/src/encoding.c")
        .file("file/src/compress.c")
        .file("file/src/is_csv.c")
        .file("file/src/is_json.c")
        .file("file/src/is_simh.c")
        .file("file/src/is_tar.c")
        .file("file/src/readelf.c")
        .file("file/src/print.c")
        .file("file/src/fsmagic.c")
        .file("file/src/funcs.c")
        .file("file/src/apptype.c")
        .file("file/src/der.c")
        .file("file/src/cdf.c")
        .file("file/src/cdf_time.c")
        .file("file/src/readcdf.c")
        .file("file/src/fmtcheck.c");

    if target_vendor != "apple" {
        build.file("file/src/strlcpy.c");
    }

    build.compile("magic");
}

fn main() {
    #[cfg(feature = "bundled")]
    {
        let lib = try_bundled();
        return;
    }

    if let Some(magic_dir) = env("MAGIC_DIR").map(std::path::PathBuf::from) {
        if !std::path::Path::new(&magic_dir).exists() {
            panic!("Magic library directory {:?} does not exist", magic_dir);
        }
        println!(
            "cargo:rustc-link-search=native={}",
            magic_dir.to_string_lossy()
        );

        let static_lib = magic_dir.join("libmagic.a");
        let shared_lib = magic_dir.join("libmagic.so");
        match env("MAGIC_STATIC").as_ref().and_then(|s| s.to_str()) {
            Some("false") | Some("FALSE") | Some("0") => {
                if !shared_lib.exists() {
                    panic!("No libmagic.so found in {:?}", magic_dir);
                }
                println!("cargo:rustc-link-lib=dylib=magic");
            }
            Some(_) => {
                if !static_lib.exists() {
                    panic!("No libmagic.a found in {:?}", magic_dir);
                }
                println!("cargo:rustc-link-lib=static=magic");
            }
            None => {
                match (static_lib.exists(), shared_lib.exists()) {
                    (false, false) => panic!("Neither libmagic.so, nor libmagic.a was found in {:?}", magic_dir),
                    (true, false) => println!("cargo:rustc-link-lib=static=magic"),
                    (false, true) => println!("cargo:rustc-link-lib=dylib=magic"),
                    (true, true) => panic!("Both a static and a shared library were found in {:?}\nspecify a choice with `MAGIC_STATIC=true|false`", magic_dir),
                }
            }
        }
    } else {
        if let Err(err) = vcpkg::find_package("libmagic") {
            println!("Could not find vcpkg package: {}", err);
        } else if cfg!(windows) {
            // workaround, see https://github.com/robo9k/rust-magic-sys/pull/16#issuecomment-949094327
            println!("cargo:rustc-link-lib=shlwapi");

            // vcpkg was successful, don't print anything else
            return;
        }

        // default fall through: try linking dynamically to just `libmagic` without further config
        println!("cargo:rustc-link-lib=dylib=magic");
    }
}
