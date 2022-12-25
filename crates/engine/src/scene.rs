use math::prelude::*;
use crate::render::{Vertex, Index};

pub type Vertices = Vec<Vertex>,
pub type Indices = Vec<Index>,

pub struct Scene {
    models: collections::HashMap<String, Model>,
}

impl Scene {
    
}

pub struct Model {
    batch_settings: BatchSettings,
    vertices: Vertices,
    indices: Option<Indices>,
    position: Vec<f32, 3>,
    rotation: Vec<f32, 3>,
    scale: Vec<f32, 3>,
}

pub struct BatchSettings {
    r#type: BatchType,
}

pub enum BatchType {
    Static {
        dirty: bool,
    },
    Dynamic,
}
