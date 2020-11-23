extern crate anyhow;
extern crate glob;
extern crate shaderc;
extern crate fs_extra;

use anyhow::*;
use glob::glob;
use std::fs::{read_to_string, write};
use std::path::PathBuf;

struct ShaderData {
    src: String,
    src_path: PathBuf,
    spv_path: PathBuf,
    kind: shaderc::ShaderKind,
}

impl ShaderData {
    pub fn load(src_path: PathBuf) -> Result<Self> {
        // 拡張子
        let extension = src_path
            .extension()
            .context("Flie has no extension")?
            .to_str()
            .context("Extension cannot be converted to &str")?;
        let kind = match extension {
            "vert" | "glslv" => shaderc::ShaderKind::Vertex,
            "frag" | "glslf" => shaderc::ShaderKind::Fragment,
            "comp" => shaderc::ShaderKind::Compute,
            _ => bail!("Unsupported shader: {}", src_path.display()),
        };

        let src = read_to_string(src_path.clone())?;
        let spv_path = src_path.with_extension(format!("{}.spv", extension));

        Ok(Self {
            src,
            src_path,
            spv_path,
            kind
        })
    }
}

fn main() -> Result<()> {
    let mut shader_paths = [
        glob("./src/**/*.vert")?,
        glob("./src/**/*.frag")?,
        glob("./src/**/*.comp")?,
    ];

    let shaders = shader_paths
        .iter_mut()
        .flatten()
        .map(|glob_result| ShaderData::load(glob_result?))
        .collect::<Vec<Result<_>>>()
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .context("something wrong with shader_paths.")?;

    let mut compiler = shaderc::Compiler::new()
        .context("Unable to create shader compiler")?;

    // 並列化するのかしないのかとかなんとかかんとか
    // 変化があったときに実行させれば良くないかとのこと
    for shader in shaders {
        // 変化があればこのスクリプトをもう一度実行するように指定
        println!("cargo:rerun-if-changed={:?}", shader.src_path);
        let compiled = compiler.compile_into_spirv(
            &shader.src,
            shader.kind,
            &shader.src_path.to_str().unwrap(),
            "main",
            None
        )?;
        write(shader.spv_path, compiled.as_binary_u8())?;
    }

    println!("cargo:rerun-if-changed=assets/*");

    use fs_extra::{
        dir::CopyOptions,
        copy_items,
    };

    let out_dir = std::env::var("OUT_DIR")?;
    let mut copy_options = CopyOptions::new();
    copy_options.overwrite = true;
    let mut paths_to_copy = Vec::new();
    paths_to_copy.push("assets/");
    copy_items(&paths_to_copy, out_dir, &copy_options)?;

    Ok(())
}