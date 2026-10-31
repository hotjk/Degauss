include!("src/font_sizes.rs");
#[path = "build/font_data.rs"]
mod font_data;

fn valid_screenscraper_credential(value: Option<&str>, max: usize) -> bool {
    value.is_some_and(|value| {
        !value.is_empty() && value.chars().count() <= max && !value.chars().any(char::is_control)
    })
}

fn main() {
    println!("cargo:rerun-if-env-changed=DEGAUSS_SCREENSCRAPER_DEVID");
    println!("cargo:rerun-if-env-changed=DEGAUSS_SCREENSCRAPER_DEVPASSWORD");

    // Every distributable MiSTer build must contain the application
    // credentials ScreenScraper issued for Degauss. Host builds remain usable
    // for tests and development, but the release script cannot accidentally
    // produce an ARM package whose scraper is incomplete.
    let degauss_arm_release = std::env::var("PROFILE").as_deref() == Ok("release")
        && std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("arm");
    if degauss_arm_release {
        let degauss_developer_id = std::env::var("DEGAUSS_SCREENSCRAPER_DEVID").ok();
        let degauss_developer_password = std::env::var("DEGAUSS_SCREENSCRAPER_DEVPASSWORD").ok();
        if !valid_screenscraper_credential(degauss_developer_id.as_deref(), 128)
            || !valid_screenscraper_credential(degauss_developer_password.as_deref(), 256)
        {
            panic!("the MiSTer release build requires valid Degauss ScreenScraper credentials");
        }
    }

    // Resources and glyphs are baked into the binary: the software renderer
    // has no system font stack to fall back on, and the MiSTer has no fonts
    // installed that we would want to depend on anyway.
    //
    // The typeface is named, not inherited from whatever the machine doing
    // the build happens to have installed. Without this, Slint asks the host
    // for a default: a Mac produced Helvetica and the CI runner produced
    // DejaVu Sans, so the same commit shipped two different-looking builds
    // and a release never matched a local one.
    //
    // Naming it here also makes it the first font registered, which is the
    // one the renderer falls back to, so a build where something went wrong
    // is set in the right typeface rather than in whichever other one.
    println!("cargo:rerun-if-changed=src/font_sizes.rs");
    println!("cargo:rerun-if-changed=assets/fonts");
    std::env::set_var(
        "SLINT_DEFAULT_FONT",
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/DejaVuSans.ttf"),
    );

    // Glyphs are baked as bitmaps, one set per pixel size, and only the sizes
    // named here exist at runtime. There is one list for both typefaces, so
    // it is the union of the two ladders and each carries sizes it will never
    // be asked for; `Font::quantise` is what keeps each typeface to its own.
    let mut sizes: Vec<f32> = SMOOTH_SIZES
        .iter()
        .chain(PIXEL_SIZES.iter())
        .copied()
        .collect();
    sizes.sort_by(f32::total_cmp);
    sizes.dedup();
    let sizes: Vec<String> = sizes.iter().map(f32::to_string).collect();
    std::env::set_var("SLINT_FONT_SIZES", sizes.join(","));

    let out = std::path::PathBuf::from(std::env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    // Latin / Greek / Cyrillic for the main window and Text Size packs.
    std::fs::write(out.join("charset.slint"), charset_slint(&LATIN_SCRIPTS, None))
        .expect("writing charset.slint");
    // Han (plus Latin for mixed titles) only for the optional PuHui pack.
    // Charset uses 26px (list default); Slint still seeds 12px as well.
    std::fs::write(
        out.join("charset_han.slint"),
        charset_slint(&HAN_SCRIPTS, Some(26.0)),
    )
    .expect("writing charset_han.slint");

    // Slint's font-size list applies to every font in one compiled component.
    // Separate tiny components keep each optional preset to only the five
    // smooth or four pixel sizes it uses. Creating them once on Degauss's
    // existing software window registers their embedded bitmaps; there is no
    // runtime font loader or additional process.
    for (label, index) in [("smaller", 0), ("small", 1), ("large", 3), ("larger", 4)] {
        for kind in ["smooth", "pixel"] {
            let sizes = if kind == "smooth" {
                SMOOTH_TEXT_SIZES
                    .iter()
                    .map(|rung| rung[index])
                    .collect::<Vec<_>>()
            } else {
                (1..=PIXEL_SIZES.len())
                    .map(|multiple| PIXEL_TEXT_BASE_SIZES[index] * multiple as f32)
                    .collect::<Vec<_>>()
            };
            std::env::set_var(
                "SLINT_FONT_SIZES",
                sizes
                    .iter()
                    .map(f32::to_string)
                    .collect::<Vec<_>>()
                    .join(","),
            );
            let default_font = if kind == "smooth" {
                format!("DejaVuSans-{label}.ttf")
            } else {
                format!("Px437_JPN12-{}.ttf", PIXEL_TEXT_BASE_SIZES[index] as u8)
            };
            std::env::set_var(
                "SLINT_DEFAULT_FONT",
                format!("{}/assets/fonts/{default_font}", env!("CARGO_MANIFEST_DIR")),
            );
            let path = format!("ui/font-packs/{kind}_{label}.slint");
            let config = slint_build::CompilerConfiguration::new()
                .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer)
                .with_include_paths(vec![out.clone()]);
            slint_build::compile_with_config(&path, config)
                .unwrap_or_else(|error| panic!("compiling {path}: {error}"));
            font_data::compact_file(&out.join(format!("{kind}_{label}.rs")));
        }
    }

    // Personal CJK face: one family, 12 + 26px only, never mixed into Latin packs.
    {
        std::env::set_var(
            "SLINT_FONT_SIZES",
            PUHUI_SIZES
                .iter()
                .map(f32::to_string)
                .collect::<Vec<_>>()
                .join(","),
        );
        std::env::set_var(
            "SLINT_DEFAULT_FONT",
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/assets/fonts/AlibabaPuHuiTi-3-55-Regular.ttf"
            ),
        );
        let path = "ui/font-packs/puhui.slint";
        let config = slint_build::CompilerConfiguration::new()
            .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer)
            .with_include_paths(vec![out.clone()]);
        slint_build::compile_with_config(path, config)
            .unwrap_or_else(|error| panic!("compiling {path}: {error}"));
        font_data::compact_file(&out.join("puhui.rs"));
    }

    // slint::include_modules! reads the path emitted by the final compile,
    // so the main window must be compiled last.
    std::env::set_var("SLINT_FONT_SIZES", sizes.join(","));
    std::env::set_var(
        "SLINT_DEFAULT_FONT",
        concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts/DejaVuSans.ttf"),
    );
    let config = slint_build::CompilerConfiguration::new()
        .embed_resources(slint_build::EmbedResourcesKind::EmbedForSoftwareRenderer)
        .with_include_paths(vec![out.clone()]);
    slint_build::compile_with_config("ui/degauss.slint", config)
        .expect("compiling ui/degauss.slint");
    font_data::compact_file(&out.join("degauss.rs"));
}

/// Latin / Greek / Cyrillic planes used by the shipped typefaces.
///
/// Japanese and Chinese stay out of this list on purpose: baking Han into
/// Smooth / Pixel / Smooth 2 / Pixel 2 would multiply binary size. Han is
/// offered only through the separate PuHui pack (`HAN_SCRIPTS`).
const LATIN_SCRIPTS: [(u32, u32); 16] = [
    (0x0020, 0x007E),
    (0x00A0, 0x00FF),
    (0x0100, 0x017F),
    (0x0180, 0x024F),
    (0x02B0, 0x02FF),
    (0x0300, 0x036F),
    (0x0370, 0x03FF),
    (0x0400, 0x04FF),
    (0x1E00, 0x1EFF),
    (0x2000, 0x206F),
    (0x2070, 0x209F),
    (0x20A0, 0x20BF),
    (0x2100, 0x214F),
    (0x2150, 0x218F),
    (0x2190, 0x21FF),
    (0x2200, 0x22FF),
];

/// Scripts for the PuHui pack: Latin (mixed EN/CN titles) plus Han ranges
/// covered by Alibaba PuHuiTi. Code points the face lacks are dropped.
const HAN_SCRIPTS: [(u32, u32); 21] = [
    (0x0020, 0x007E),
    (0x00A0, 0x00FF),
    (0x0100, 0x017F),
    (0x0180, 0x024F),
    (0x02B0, 0x02FF),
    (0x0300, 0x036F),
    (0x0370, 0x03FF),
    (0x0400, 0x04FF),
    (0x1E00, 0x1EFF),
    (0x2000, 0x206F),
    (0x2070, 0x209F),
    (0x20A0, 0x20BF),
    (0x2100, 0x214F),
    (0x2150, 0x218F),
    (0x2190, 0x21FF),
    (0x2200, 0x22FF),
    (0x3000, 0x303F),
    (0x3400, 0x4DBF),
    (0x4E00, 0x9FFF),
    (0xF900, 0xFAFF),
    (0xFF00, 0xFFEF),
];

fn charset_slint(scripts: &[(u32, u32)], font_size_px: Option<f32>) -> String {
    let mut characters = String::new();
    for &(first, last) in scripts {
        for code in first..=last {
            let Some(character) = char::from_u32(code) else {
                continue;
            };
            if character.is_control() || matches!(code, 0x2028 | 0x2029 | 0xFEFF) {
                continue;
            }
            if character == '\\' || character == '"' {
                characters.push('\\');
            }
            characters.push(character);
        }
    }
    let font_size_line = font_size_px
        .map(|size| format!("\n        font-size: {size}px;"))
        .unwrap_or_default();
    format!(
        "// Generated by build.rs. Every character the embedded typefaces can\n\
         // draw; the compiler drops the ones neither of them has.\n\
         export component Charset inherits Text {{\n\
        \x20   visible: false;\n\
        \x20   width: 0px;\n\
        \x20   height: 0px;{font_size_line}\n\
        \x20   text: \"{characters}\";\n\
         }}\n"
    )
}
