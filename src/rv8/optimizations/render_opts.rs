//! Rendering optimizations

/// Render optimization settings
pub struct RenderOptimizations {
    /// GPU rasterization
    pub gpu_raster: bool,
    /// Threaded scrolling
    pub threaded_scroll: bool,
    /// Number of raster threads
    pub raster_threads: usize,
    /// Layer caching
    pub layer_cache: bool,
}

impl Default for RenderOptimizations {
    fn default() -> Self {
        RenderOptimizations {
            gpu_raster: true,
            threaded_scroll: true,
            raster_threads: 4,
            layer_cache: true,
        }
    }
}
