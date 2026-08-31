use fs_extra::dir::{copy, remove, CopyOptions};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

macro_rules! std_env_expected {
    ($name:expr) => {{
        std::env::var($name).expect(&format!("{} expected", $name))
    }};
}

struct Artifacts {
    root: PathBuf,
    include_dir: PathBuf,
    lib_dir: PathBuf,
    libs: Vec<String>,
}

struct NativeSources {
    libevent: PathBuf,
    tor: PathBuf,
}

fn run_command(program: &str, args: &[&str], cwd: &Path) -> Result<(), String> {
    let output = Command::new(program)
        .current_dir(cwd)
        .args(args)
        .output()
        .map_err(|error| format!("failed to run {program}: {error}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

fn copy_source(name: &str) -> PathBuf {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("vendor")
        .join(name);
    let root = PathBuf::from(std_env_expected!("OUT_DIR")).join("native-sources");
    let destination = root.join(name);

    fs::create_dir_all(&root).expect("Cannot write native sources to OUT_DIR");
    if destination.exists() {
        remove(&destination).expect("Cannot replace staged native source");
    }
    copy(source, &root, &CopyOptions::new()).expect("Cannot stage native source");

    destination
}

fn patches(prefix: &str) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("patches");
    let mut patches = fs::read_dir(root)
        .expect("Cannot list native patches")
        .collect::<Result<Vec<_>, _>>()
        .expect("Cannot read native patch entry")
        .into_iter()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .expect("Native patch name must be UTF-8")
                .starts_with(prefix)
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    patches.sort();
    patches
}

fn apply_patches(prefix: &str, source: &Path) {
    for patch in patches(prefix) {
        let patch = patch.to_str().expect("Native patch path must be UTF-8");
        run_command("git", &["apply", "-p1", patch], source)
            .unwrap_or_else(|error| panic!("Cannot apply native patch {patch}: {error}"));
    }
}

fn prepare_native_sources() -> NativeSources {
    let libevent = copy_source("libevent");
    let tor = copy_source("tor");

    apply_patches("libevent", &libevent);
    apply_patches("tor", &tor);
    run_command("sh", &["autogen.sh"], &libevent).expect("Cannot configure libevent sources");
    run_command("sh", &["autogen.sh"], &tor).expect("Cannot configure Tor sources");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=patches");
    println!("cargo:rerun-if-changed=vendor/libevent");
    println!("cargo:rerun-if-changed=vendor/tor");

    NativeSources { libevent, tor }
}

impl Artifacts {
    fn print_cargo_metadata(&self) {
        println!("cargo:rustc-link-search=native={}", self.lib_dir.display());
        for lib in &self.libs {
            println!("cargo:rustc-link-lib=static={lib}");
        }
        println!("cargo:include={}", self.include_dir.display());
        println!("cargo:lib={}", self.lib_dir.display());
    }
}

fn autotools_host(target: &str) -> String {
    target
        .replace("apple-ios-sim", "apple-darwin")
        .replace("apple-ios", "apple-darwin")
}

fn build_libevent(path: &Path) -> Artifacts {
    let target = std_env_expected!("TARGET");
    let host = std_env_expected!("HOST");
    let autotools_host = autotools_host(&target);

    let mut cc = cc::Build::new();
    cc.target(&target).host(&host);
    let compiler = cc.get_compiler();

    let root = PathBuf::from(env::var("OUT_DIR").expect("missing OUT_DIR")).join("libevent");
    fs::create_dir_all(&root).expect("Cannot write to OUT_DIR");

    let mut config = autotools::Config::new(path);
    config
        .out_dir(&root)
        .config_option("host", Some(&autotools_host))
        .env("CC", compiler.path())
        .env("CFLAGS", compiler.cflags_env())
        .enable_static()
        .disable_shared()
        .with("pic", None)
        .disable("samples", None)
        .disable("openssl", None)
        .disable("libevent-regress", None)
        .disable("debug-mode", None)
        .disable("dependency-tracking", None);

    let libevent = config.build();
    let mut libs = vec!["event".to_string()];
    if target.contains("windows") {
        println!("cargo:rustc-link-lib=static=ssp");
    } else {
        libs.push("event_pthreads".to_string());
    }

    let artifacts = Artifacts {
        lib_dir: libevent.join("lib"),
        include_dir: root.join("include"),
        libs,
        root,
    };
    artifacts.print_cargo_metadata();
    artifacts
}

fn build_tor(libevent: Artifacts, path: &Path) {
    let target = std_env_expected!("TARGET");
    let host = std_env_expected!("HOST");
    let autotools_host = autotools_host(&target);

    let mut cc = cc::Build::new();
    cc.target(&target).host(&host);
    let compiler = cc.get_compiler();

    let openssl_dir = env::var("DEP_OPENSSL_ROOT").ok().map(PathBuf::from);
    let lzma_dir = env::var("DEP_LZMA_ROOT").ok().map(PathBuf::from);
    let zstd_dir = env::var("DEP_ZSTD_ROOT").ok().map(PathBuf::from);
    let mut config = autotools::Config::new(path);
    config
        .config_option("host", Some(&autotools_host))
        .env("CC", compiler.path())
        .with("libevent-dir", libevent.root.to_str())
        .enable("pic", None)
        .enable("static-libevent", None)
        .enable("static-zlib", None)
        .disable("system-torrc", None)
        .disable("asciidoc", None)
        .disable("systemd", None)
        .disable("largefile", None)
        .disable("unittests", None)
        .disable("tool-name-check", None)
        .disable("manpage", None)
        .disable("html-manual", None)
        .disable("module-dirauth", None)
        .disable("module-relay", None)
        .disable("module-pow", None)
        .disable("seccomp", None)
        .disable("libscrypt", None);

    let mut cflags = format!(" {}", compiler.cflags_env().into_string().unwrap());

    if !cfg!(feature = "with-lzma") {
        config.disable("lzma", None);
    }
    if !cfg!(feature = "with-zstd") {
        config.disable("zstd", None);
    }
    if target.contains("windows") {
        config.env("LIBS", "-lcrypt32 -liphlpapi -lws2_32 -lgdi32");
    }
    if let Some(dir) = &openssl_dir {
        config
            .with("openssl-dir", dir.to_str())
            .enable("static-openssl", None);
    }
    if let Some(dir) = &lzma_dir {
        let include = std_env_expected!("DEP_LZMA_INCLUDE");
        config.env("LZMA_CFLAGS", format!("-I{include}"));
        config.env("LZMA_LIBS", dir.join("liblzma.a").to_str().unwrap());
        println!("cargo:rustc-link-lib=static=lzma");
    }
    if let Some(dir) = &zstd_dir {
        let include = std_env_expected!("DEP_ZSTD_INCLUDE");
        config.env("ZSTD_CFLAGS", format!("-I{include}"));
        config.env("ZSTD_LIBS", dir.join("libzstd.a").to_str().unwrap());
        println!("cargo:rustc-link-lib=static=zstd");
    }

    if target.contains("android") {
        let output = compiler
            .to_command()
            .args(["--print-file-name", "libz.a"])
            .output()
            .expect("Failed to run clang");
        assert!(
            output.status.success(),
            "clang did not complete successfully"
        );
        let libz_path = std::str::from_utf8(&output.stdout)
            .expect("Invalid path for libz.a")
            .trim();
        let libz_path = PathBuf::from(libz_path);
        let sysroot_lib = libz_path.parent().expect("Invalid path for libz.a");
        let sysroot_usr = sysroot_lib
            .parent()
            .and_then(|path| path.parent())
            .expect("Invalid Android sysroot layout");
        let zlib_root = PathBuf::from(std_env_expected!("OUT_DIR")).join("zlib");
        let zlib_include = zlib_root.join("include");
        let zlib_lib = zlib_root.join("lib");
        fs::create_dir_all(&zlib_include).expect("Cannot create zlib include directory");
        fs::create_dir_all(&zlib_lib).expect("Cannot create zlib library directory");
        for header in ["zlib.h", "zconf.h"] {
            let destination = zlib_include.join(header);
            if destination.exists() {
                fs::remove_file(&destination).expect("Cannot replace staged zlib header");
            }
            fs::copy(sysroot_usr.join("include").join(header), destination)
                .expect("Cannot stage zlib header");
        }
        let staged_libz = zlib_lib.join("libz.a");
        if staged_libz.exists() {
            fs::remove_file(&staged_libz).expect("Cannot replace staged libz.a");
        }
        fs::copy(&libz_path, staged_libz).expect("Cannot stage libz.a");
        config
            .enable("android", None)
            .with("zlib-dir", zlib_root.to_str());
        println!("cargo:rustc-link-search=native={}", zlib_lib.display());
    } else {
        let mut zlib_dir = PathBuf::from(std_env_expected!("DEP_Z_ROOT"));
        cflags += &format!(" -I{}", zlib_dir.join("include").display());
        zlib_dir.push("lib");
        config.with("zlib-dir", zlib_dir.to_str());
        println!("cargo:rustc-link-search=native={}", zlib_dir.display());
    }

    let tor = config.env("CFLAGS", cflags).build();
    if let Some(dir) = &openssl_dir {
        println!(
            "cargo:rustc-link-search=native={}",
            dir.join("lib").display()
        );
    }
    println!(
        "cargo:rustc-link-search=native={}",
        tor.join("build").display()
    );
    println!("cargo:rustc-link-lib=static=event");
    if !target.contains("windows") {
        println!("cargo:rustc-link-lib=static=event_pthreads");
    }
    println!("cargo:rustc-link-lib=static=z");
    println!("cargo:rustc-link-lib=static=tor");

    if openssl_dir.is_some() {
        println!("cargo:rustc-link-lib=static=crypto");
        println!("cargo:rustc-link-lib=static=ssl");
    } else {
        println!("cargo:rustc-link-lib=crypto");
        println!("cargo:rustc-link-lib=ssl");
    }

    if target.contains("windows") {
        let output = Command::new(compiler.path())
            .arg("-print-search-dirs")
            .output()
            .expect("CC does not accept -print-search-dirs");
        let output = std::str::from_utf8(&output.stdout).expect("Invalid compiler output");
        for line in output
            .lines()
            .filter_map(|line| line.strip_prefix("libraries: ="))
        {
            for path in line.split(':') {
                println!("cargo:rustc-link-search=native={path}");
            }
        }
        for lib in [
            "crypt32", "iphlpapi", "ws2_32", "gdi32", "shell32", "ssp", "shlwapi",
        ] {
            println!("cargo:rustc-link-lib={lib}");
        }
    }

    fs::create_dir_all(tor.join("include")).unwrap();
    fs::copy(
        path.join("src/feature/api/tor_api.h"),
        tor.join("include/tor_api.h"),
    )
    .unwrap();
    println!("cargo:include={}/include", tor.display());
}

fn main() {
    let sources = prepare_native_sources();
    let libevent = build_libevent(&sources.libevent);
    build_tor(libevent, &sources.tor);
}
