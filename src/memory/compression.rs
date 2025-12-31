//! Compression utilities for tab snapshots
//!
//! Uses zstd for fast compression/decompression with good compression ratios.
//! Target: <100ms decompression for "instant-back" user experience.

use log::{debug, error};

/// Compression level for zstd (1-22)
/// Level 3 provides good balance of speed and compression ratio
const ZSTD_COMPRESSION_LEVEL: i32 = 3;

/// Compress data using zstd
pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    // Placeholder for actual zstd compression
    // In production, this would use the zstd crate
    debug!("Compressing {} bytes with zstd level {}", data.len(), ZSTD_COMPRESSION_LEVEL);
    
    // For now, just return a copy to establish the interface
    Ok(data.to_vec())
}

/// Decompress zstd-compressed data
pub fn decompress(compressed: &[u8]) -> Result<Vec<u8>, String> {
    debug!("Decompressing {} bytes", compressed.len());
    
    // Placeholder for actual zstd decompression
    Ok(compressed.to_vec())
}

/// Estimate compression ratio for given data type
pub fn estimate_compression_ratio(data_type: DataType) -> f32 {
    match data_type {
        DataType::Dom => 0.25,        // DOM text compresses very well (~75% reduction)
        DataType::RenderTree => 0.30,  // Render tree has repetitive structures
        DataType::V8Heap => 0.40,      // JavaScript heap has mixed compressibility
        DataType::ImageData => 0.85,   // Already compressed in most cases
    }
}

/// Type of data being compressed
#[derive(Debug, Clone, Copy)]
pub enum DataType {
    /// DOM tree snapshot
    Dom,
    /// Render tree snapshot
    RenderTree,
    /// V8 heap snapshot
    V8Heap,
    /// Image/texture data
    ImageData,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress() {
        let original = b"Hello, world!";
        let compressed = compress(original).unwrap();
        let decompressed = decompress(&compressed).unwrap();
        assert_eq!(original, decompressed.as_slice());
    }

    #[test]
    fn test_compression_ratios() {
        assert!(estimate_compression_ratio(DataType::Dom) < 0.5);
        assert!(estimate_compression_ratio(DataType::ImageData) > 0.5);
    }
}
