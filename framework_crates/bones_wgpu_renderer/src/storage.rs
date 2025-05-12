use bones_framework::prelude::*;
use serde::{de::Visitor, Deserialize, Serialize};

#[cfg(target_arch = "wasm32")]
pub use wasm::StorageBackend;
#[cfg(target_arch = "wasm32")]
mod wasm {
    use super::*;
    pub struct StorageBackend {
        storage_key: String,
    }

    impl StorageBackend {
        pub fn new(qualifier: &str, organization: &str, application: &str) -> Self {
            Self {
                storage_key: format!("{qualifier}.{organization}.{application}.storage"),
            }
        }
    }

    impl StorageApi for StorageBackend {
        fn save(&mut self, data: Vec<SchemaBox>) {
            let mut buffer = Vec::new();
            let mut serializer = serde_yaml::Serializer::new(&mut buffer);
            LoadedStorage(data)
                .serialize(&mut serializer)
                .expect("Failed to serialize to storage file.");
            let data = String::from_utf8(buffer).unwrap();
            let window = web_sys::window().unwrap();
            let storage = window.local_storage().unwrap().unwrap();
            storage.set_item(&self.storage_key, &data).unwrap();
        }

        fn load(&mut self) -> Vec<SchemaBox> {
            let window = web_sys::window().unwrap();
            let storage = window.local_storage().unwrap().unwrap();
            let Some(data) = storage.get_item(&self.storage_key).unwrap() else {
                return default();
            };

            let Ok(loaded) = serde_yaml::from_str::<LoadedStorage>(&data) else {
                return default();
            };
            loaded.0
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::StorageBackend;
#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;

    pub struct StorageBackend {
        storage_path: std::path::PathBuf,
    }

    impl StorageBackend {
        pub fn new(qualifier: &str, organization: &str, application: &str) -> Self {
            let project_dirs = directories::ProjectDirs::from(qualifier, organization, application)
                .expect("Identify system data dir path");
            Self {
                storage_path: project_dirs.data_dir().join("storage.yml"),
            }
        }
    }

    impl StorageApi for StorageBackend {
        fn save(&mut self, data: Vec<SchemaBox>) {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .truncate(true)
                .create(true)
                .open(&self.storage_path)
                .expect("Failed to open storage file");
            let mut serializer = serde_yaml::Serializer::new(file);
            LoadedStorage(data)
                .serialize(&mut serializer)
                .expect("Failed to serialize to storage file.");
        }

        fn load(&mut self) -> Vec<SchemaBox> {
            if self.storage_path.exists() {
                let result: anyhow::Result<LoadedStorage> = (|| {
                    let file = std::fs::OpenOptions::new()
                        .read(true)
                        .open(&self.storage_path)
                        .context("Failed to open storage file")?;
                    let loaded: LoadedStorage = serde_yaml::from_reader(file)
                        .context("Failed to deserialize storage file")?;

                    anyhow::Result::Ok(loaded)
                })();
                match result {
                    Ok(loaded) => loaded.0,
                    Err(e) => {
                        log::error!(
                            "Error deserializing storage file, ignoring file, \
                        data will be overwritten when saved: {e:?}"
                        );
                        default()
                    }
                }
            } else {
                std::fs::create_dir_all(self.storage_path.parent().unwrap()).unwrap();
                default()
            }
        }
    }
}

struct LoadedStorage(Vec<SchemaBox>);
impl Serialize for LoadedStorage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let data: HashMap<String, SchemaRef> = self
            .0
            .iter()
            .map(|x| (x.schema().full_name.to_string(), x.as_ref()))
            .collect();

        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(data.len()))?;

        for (key, value) in data {
            map.serialize_key(&key)?;
            map.serialize_value(&SchemaSerializer(value))?;
        }

        map.end()
    }
}
impl<'de> Deserialize<'de> for LoadedStorage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_map(LoadedStorageVisitor).map(Self)
    }
}
struct LoadedStorageVisitor;
impl<'de> Visitor<'de> for LoadedStorageVisitor {
    type Value = Vec<SchemaBox>;
    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "Mapping of string type names to type data.")
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut data = Vec::new();
        while let Some(type_name) = map.next_key::<String>()? {
            let Some(schema) = SCHEMA_REGISTRY
                .schemas
                .iter()
                .find(|schema| schema.full_name.as_ref() == type_name)
            else {
                log::error!(
                    "\n\nCannot find schema registration for `{}` while loading persisted \
                        storage. This means you that you need to call \
                        `{}::schema()` to register your persisted storage type before \
                        creating the `BonesWgpuRenderer` or that there is data from an old \
                        version of the app inside of the persistent storage file.\n\n",
                    type_name,
                    type_name,
                );
                continue;
            };

            data.push(map.next_value_seed(SchemaDeserializer(schema))?);
        }

        Ok(data)
    }
}
