use std::collections::HashMap;

use serde::{Deserialize, Deserializer, de::DeserializeOwned};

use crate::{fs::SharedFilesystem, util::ensure_path_has_extension};

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
        Material::LightmappedGeneric { basetexture }
        | Material::UnlitGeneric { basetexture }
        | Material::VertexLitGeneric { basetexture }
        | Material::WorldVertexTransition { basetexture }
        | Material::UnlitTwoTexture { basetexture, .. } => Ok(Some(basetexture)),
        Material::Patch { include } => get_basetexture_for_vmt(fs, &include),
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Material {
    LightmappedGeneric {
        #[serde(rename = "$basetexture")]
        basetexture: String,
    },
    UnlitGeneric {
        #[serde(rename = "$basetexture")]
        basetexture: String,
    },
    VertexLitGeneric {
        #[serde(rename = "$basetexture")]
        basetexture: String,
    },
    WorldVertexTransition {
        #[serde(rename = "$basetexture")]
        basetexture: String,
    },
    UnlitTwoTexture {
        #[serde(rename = "$basetexture")]
        basetexture: String,
        #[serde(rename = "$texture2")]
        texture2: String,
    },
    Patch {
        include: String,
    },
}

fn deserialize_kv_case_insensitive<'de, T, D>(deserializer: D) -> Result<T, D::Error>
where
    T: DeserializeOwned,
    D: Deserializer<'de>,
{
    use vdf_reader::entry::Entry;

    use std::collections::BTreeMap as Map;

    let map = Map::<String, Entry>::deserialize(deserializer)?;
    let lower: HashMap<String, Entry> = map
        .into_iter()
        .map(|(k, v)| (k.to_lowercase(), json_value_lowercase_keys(v)))
        .collect();
    T::deserialize(Entry::Table(lower.into())).map_err(serde::de::Error::custom)
}

fn json_value_lowercase_keys(v: vdf_reader::entry::Entry) -> vdf_reader::entry::Entry {
    use vdf_reader::entry::Entry;
    match v {
        Entry::Table(map) => Entry::Table(
            map.into_iter()
                .map(|(k, v)| (k.to_lowercase(), json_value_lowercase_keys(v)))
                .collect::<HashMap<String, Entry>>()
                .into(),
        ),
        _ => v,
    }
}
