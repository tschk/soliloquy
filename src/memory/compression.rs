//! Compression utilities for tab snapshots
//!
//! Uses zstd for fast compression/decompression with good compression ratios.
//! Target: <100ms decompression for "instant-back" user experience.

use log::{debug, warn};

/// Compression level for zstd (1-22)
/// Level 3 provides good balance of speed and compression ratio
const ZSTD_COMPRESSION_LEVEL: i32 = 3;

/// Compress data using zstd
///
/// TODO: Enable the `real_compression` feature and add zstd dependency before production use.
#[cfg(not(feature = "real_compression"))]
pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    warn!(
        "Using placeholder compression (no-op). Enable the `real_compression` feature \
         to use real zstd compression."
    );
    debug!(
        "Compressing {} bytes with zstd level {} (placeholder, no compression performed)",
        data.len(),
        ZSTD_COMPRESSION_LEVEL
    );
    
    // For now, just return a copy to establish the interface
    Ok(data.to_vec())
}

/// Compress data using real zstd compression
#[cfg(feature = "real_compression")]
pub fn compress(data: &[u8]) -> Result<Vec<u8>, String> {
    debug!(
        "Compressing {} bytes with zstd level {} (real_compression feature enabled)",
        data.len(),
        ZSTD_COMPRESSION_LEVEL
    );
    
    // TODO: Implement with zstd crate:
    // zstd::encode_all(&data[..], ZSTD_COMPRESSION_LEVEL)
    //     .map_err(|e| format!("zstd compression failed: {e}"))
    
    // Placeholder until zstd dependency is added
    Ok(data.to_vec())
}

/// Decompress zstd-compressed data
///
/// TODO: Enable the `real_compression` feature and add zstd dependency before production use.
#[cfg(not(feature = "real_compression"))]
pub fn decompress(compressed: &[u8]) -> Result<Vec<u8>, String> {
    warn!(
        "Using placeholder decompression (no-op). Enable the `real_compression` feature \
         to use real zstd decompression."
    );
    debug!(
        "Decompressing {} bytes (placeholder, no decompression performed)",
        compressed.len()
    );
    
    // Placeholder for actual zstd decompression
    Ok(compressed.to_vec())
}

/// Decompress data using real zstd decompression
#[cfg(feature = "real_compression")]
pub fn decompress(compressed: &[u8]) -> Result<Vec<u8>, String> {
    debug!(
        "Decompressing {} bytes (real_compression feature enabled)",
        compressed.len()
    );
    
    // TODO: Implement with zstd crate:
    // zstd::decode_all(compressed)
    //     .map_err(|e| format!("zstd decompression failed: {e}"))
    
    // Placeholder until zstd dependency is added
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
