use crate::kv::Vector;

pub trait EntityClass: serde::Deserialize<'static> {
    const CLASS_NAME: &'static str;
}

macro_rules! entity_class {
    (pub struct $name:ident($classname:expr) {
        $($field:ident: $type:ty),+ $(,)?
    }) => {
        #[derive(Debug, Clone, serde::Deserialize)]
        pub struct $name {
            $(pub $field: $type),+
        }

        impl EntityClass for $name {
            const CLASS_NAME: &'static str = $classname;
        }
    };
}

entity_class! {
    pub struct SkyCamera("sky_camera") {
        origin: Vector,
        scale: f32,
    }
}
