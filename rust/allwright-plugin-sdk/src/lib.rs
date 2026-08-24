#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceFamily {
    Web,
    Mobile,
    Desktop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SurfacePluginDescriptor {
    pub id: &'static str,
    pub family: SurfaceFamily,
    pub version: &'static str,
    pub description: &'static str,
}

pub trait SurfacePlugin: Send + Sync {
    fn descriptor(&self) -> SurfacePluginDescriptor;
}
