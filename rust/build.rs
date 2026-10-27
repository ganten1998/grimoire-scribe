// build.rs: what ../Makefile does besides compiling Grimoire's own C.
//
//   GRIMOIRE_VERSION    ../VERSION, as -DGRIMOIRE_VERSION (env!("GRIMOIRE_VERSION") in util.rs)
//   vendored C          stb_truetype + stb_image_write (../src/stb_impl.c as it is), stb_image (as
//                       ../src/bgimage.c configures it: csrc/stb_image_impl.c), qrcodegen, and on the
//                       Kindle FBInk (FBINK_FOR_KINDLE, FBINK_MINIMAL) with csrc/fbink_shim.c
//   build/gen/l10n.c    the catalogs of ../po/*.mo, in name order: $OUT_DIR/l10n.rs (assets.rs
//                       includes it); the fonts, the eraser sample and web/* need no step, assets.rs
//                       takes them with include_bytes!, the same bytes xxd -i embeds
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

// ../Makefile's FBINK_DEF.
const FBINK_VERSION: &str = "master-886f25f";

fn main() {
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let top = manifest.parent().unwrap().to_path_buf();
    let third = top.join("third_party");

    // ---- the version ----
    let version_file = top.join("VERSION");
    println!("cargo:rerun-if-changed={}", version_file.display());
    let version = fs::read_to_string(&version_file).map(|s| s.trim().to_string()).unwrap_or_else(|_| "0.1.0".into());
    println!("cargo:rustc-env=GRIMOIRE_VERSION={version}");

    let target = env::var("TARGET").unwrap();
    let kindle = env::var_os("CARGO_FEATURE_KINDLE").is_some();
    let sim = env::var_os("CARGO_FEATURE_SIM").is_some();
    if kindle && sim {
        panic!("the kindle and sim features are two builds of the app: enable one");
    }
    if kindle && !(target.contains("linux") && target.starts_with("arm")) {
        panic!("the kindle feature builds for the Kindle: --target armv7-unknown-linux-musleabihf (got {target})");
    }

    // ---- stb_truetype and stb_image_write: ../src/stb_impl.c, the C build's own unit ----
    let stb_impl = top.join("src/stb_impl.c");
    let stb_image = manifest.join("csrc/stb_image_impl.c");
    println!("cargo:rerun-if-changed={}", stb_impl.display());
    println!("cargo:rerun-if-changed={}", stb_image.display());
    for h in ["stb_truetype.h", "stb_image.h", "stb_image_write.h"] {
        println!("cargo:rerun-if-changed={}", third.join(h).display());
    }
    let mut stb = c_build(&target);
    stb.include(&third).file(&stb_impl).file(&stb_image).warnings(false);
    stb.compile("grimoire_stb");

    // ---- qrcodegen (../Makefile's UI_SRC) ----
    let qr = third.join("qrcodegen/qrcodegen.c");
    println!("cargo:rerun-if-changed={}", qr.display());
    let mut q = c_build(&target);
    q.include(third.join("qrcodegen")).file(&qr).warnings(false);
    q.compile("grimoire_qrcodegen");

    // ---- FBInk, the Kindle only: built on its own with warnings muted, as the Makefile does ----
    if kindle {
        let fb = third.join("FBInk");
        let shim = manifest.join("csrc/fbink_shim.c");
        println!("cargo:rerun-if-changed={}", shim.display());
        let mut f = c_build(&target);
        f.include(&fb)
            .define("FBINK_FOR_KINDLE", None)
            .define("FBINK_MINIMAL", None)
            .define("FBINK_VERSION", Some(format!("\"{FBINK_VERSION}\"").as_str()))
            .opt_level(2)
            .warnings(false)
            .flag_if_supported("-w");
        for s in ["fbink.c", "cutef8/utf8.c", "cutef8/dfa.c"] {
            println!("cargo:rerun-if-changed={}", fb.join(s).display());
            f.file(fb.join(s));
        }
        f.file(&shim);
        f.compile("grimoire_fbink");
    }

    // ---- the catalogs: build/gen/l10n.c's table, from ../po/*.mo in name order ----
    let po = top.join("po");
    println!("cargo:rerun-if-changed={}", po.display());
    let mut mos: Vec<PathBuf> = fs::read_dir(&po)
        .map(|d| d.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|x| x == "mo")).collect())
        .unwrap_or_default();
    mos.sort();
    let mut l10n = String::from(
        "// The catalogs of po/*.mo, embedded by build.rs (../Makefile's build/gen/l10n.c).\n\
         pub static I18N_CATALOGS: &[crate::i18n::I18nCatalog] = &[\n",
    );
    for m in &mos {
        println!("cargo:rerun-if-changed={}", m.display());
        let code = m.file_stem().unwrap().to_string_lossy();
        l10n.push_str(&format!(
            "    crate::i18n::I18nCatalog {{ code: {code:?}, mo: include_bytes!({:?}) }},\n",
            m.display().to_string()
        ));
    }
    l10n.push_str("];\n");
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    write_if_changed(&out.join("l10n.rs"), &l10n);
}

// The flags every vendored C file gets in ../Makefile: -std=gnu11 -D_GNU_SOURCE, and on the Kindle
// -mcpu=cortex_a7 (KCC). The C build's -DGRIMOIRE_VERSION is not needed: no vendored file reads it.
fn c_build(target: &str) -> cc::Build {
    let mut b = cc::Build::new();
    b.flag_if_supported("-std=gnu11").define("_GNU_SOURCE", None);
    if target.starts_with("armv7") {
        b.flag_if_supported("-mcpu=cortex-a7");
    }
    b
}

fn write_if_changed(path: &Path, text: &str) {
    if fs::read_to_string(path).map(|old| old == text).unwrap_or(false) {
        return;
    }
    fs::write(path, text).unwrap();
}
