use anyhow::{anyhow, Context};
use keyvalues_parser::Vdf;

use crate::fs::SharedFilesystem;

pub fn get_basetexture_for_vmt(
    fs: &SharedFilesystem,
    path: &str,
) -> anyhow::Result<Option<String>> {
    let data = fs.lock().read_path(path)?;
    let Some(data) = data else {
        return Ok(None);
    };
    let data_str = String::from_utf8_lossy(&data);
    let vdf = Vdf::parse(&data_str)?;
    let obj = vdf.value.clone().unwrap_obj();

    if let Some(basetexture) = obj.get("$basetexture").or_else(|| obj.get("$baseTexture")) {
        return Ok(Some(basetexture[0].clone().unwrap_str().to_string()));
    }

    if let Some(include) = obj.get("include") {
        return get_basetexture_for_vmt(fs, &include[0].clone().unwrap_str());
    }

    Err(anyhow!("Material has no basetexture"))
}
