use chroma_dbg::ChromaDebug;
use serde::Deserialize;

use crate::{
    fs::SharedFilesystem, kv::deserialize_kv_case_insensitive, util::ensure_path_has_extension,
};

pub fn get_basetexture_for_vmt(
    fs: &SharedFilesystem,
    path: &str,
) -> anyhow::Result<Option<String>> {
    let path = ensure_path_has_extension(path, "vmt");
    let data = fs.lock().read_path(&path)?;
    let Some(data) = data else {
        return Ok(None);
    };
    let data_str = String::from_utf8_lossy(&data);
    let mat: Material =
        deserialize_kv_case_insensitive(&mut vdf_reader::serde::Deserializer::from_str(&data_str))?;

    match mat {
        Material::LightmappedGeneric { basetexture, .. }
        | Material::UnlitGeneric { basetexture, .. }
        | Material::VertexLitGeneric { basetexture, .. }
        | Material::WorldVertexTransition { basetexture, .. }
        | Material::UnlitTwoTexture { basetexture, .. }
        | Material::Water {
            normalmap: basetexture,
            ..
        } => Ok(Some(basetexture)),
        Material::Patch { include, .. } => get_basetexture_for_vmt(fs, &include),
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    LightmappedGeneric {
        #[serde(rename = "$basetexture")]
        basetexture: String,

        #[serde(flatten)]
        remaining_properties: vdf_reader::entry::Entry,
    },
    UnlitGeneric {
        #[serde(rename = "$basetexture")]
        basetexture: String,

        #[serde(flatten)]
        remaining_properties: vdf_reader::entry::Entry,
    },
    VertexLitGeneric {
        #[serde(rename = "$basetexture")]
        basetexture: String,

        #[serde(flatten)]
        remaining_properties: vdf_reader::entry::Entry,
    },
    WorldVertexTransition {
        #[serde(rename = "$basetexture")]
        basetexture: String,
        #[serde(rename = "$basetexture2")]
        basetexture2: String,

        #[serde(flatten)]
        remaining_properties: vdf_reader::entry::Entry,
    },
    UnlitTwoTexture {
        #[serde(rename = "$basetexture")]
        basetexture: String,
        #[serde(rename = "$texture2")]
        texture2: String,

        #[serde(flatten)]
        remaining_properties: vdf_reader::entry::Entry,
    },
    Patch {
        include: String,

        #[serde(flatten)]
        remaining_properties: vdf_reader::entry::Entry,
    },
    Water {
        #[serde(rename = "$normalmap")]
        normalmap: String,

        #[serde(flatten)]
        remaining_properties: vdf_reader::entry::Entry,
    },
}
