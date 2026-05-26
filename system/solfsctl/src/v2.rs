use crate::{Entry, Header, KIND_FILE};

pub const V2_BLOCK_SIZE: u64 = 4096;
pub const V2_EXTENT_RECORD_LEN: u64 = 40;
pub const V2_JOURNAL_RECORD_LEN: u64 = 64;
pub const V2_DEFAULT_JOURNAL_RECORDS: u64 = 1024;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V2Layout {
    pub block_size: u64,
    pub bitmap_offset: u64,
    pub bitmap_len: u64,
    pub extent_table_offset: u64,
    pub extent_table_len: u64,
    pub journal_offset: u64,
    pub journal_len: u64,
    pub data_start: u64,
    pub total_blocks: u64,
    pub free_blocks: u64,
    pub extents: Vec<V2Extent>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct V2Extent {
    pub inode: u64,
    pub logical_block: u64,
    pub physical_block: u64,
    pub block_count: u64,
    pub flags: u64,
}

pub fn plan_v2_layout(
    header: &Header,
    entries: &[Entry],
    target_size: u64,
) -> crate::Result<V2Layout> {
    if target_size < header.image_size {
        return Err(crate::SolfsError::Invalid(
            "v2 target size is smaller than source image".to_string(),
        ));
    }
    let total_blocks = div_ceil(target_size, V2_BLOCK_SIZE);
    let bitmap_len = div_ceil(total_blocks, 8);
    let mut extents = file_extents(entries);
    let extent_table_len = extents.len() as u64 * V2_EXTENT_RECORD_LEN;
    let bitmap_offset = align_block(header.image_size);
    let extent_table_offset = align_block(bitmap_offset + bitmap_len);
    let journal_offset = align_block(extent_table_offset + extent_table_len);
    let journal_len = V2_DEFAULT_JOURNAL_RECORDS * V2_JOURNAL_RECORD_LEN;
    let data_start = align_block(journal_offset + journal_len);
    assign_physical_blocks(&mut extents, data_start / V2_BLOCK_SIZE);
    let metadata_blocks = div_ceil(data_start, V2_BLOCK_SIZE);
    let used_file_blocks = extents.iter().map(|extent| extent.block_count).sum::<u64>();
    let used_blocks = metadata_blocks.saturating_add(used_file_blocks);
    let free_blocks = total_blocks.saturating_sub(used_blocks);

    Ok(V2Layout {
        block_size: V2_BLOCK_SIZE,
        bitmap_offset,
        bitmap_len,
        extent_table_offset,
        extent_table_len,
        journal_offset,
        journal_len,
        data_start,
        total_blocks,
        free_blocks,
        extents,
    })
}

fn file_extents(entries: &[Entry]) -> Vec<V2Extent> {
    let mut extents = Vec::new();
    for entry in entries {
        if entry.kind != KIND_FILE || entry.size == 0 {
            continue;
        }
        extents.push(V2Extent {
            inode: entry.inode,
            logical_block: 0,
            physical_block: 0,
            block_count: div_ceil(entry.size, V2_BLOCK_SIZE),
            flags: 0,
        });
    }
    extents
}

fn assign_physical_blocks(extents: &mut [V2Extent], first_block: u64) {
    let mut cursor = first_block;
    for extent in extents {
        extent.physical_block = cursor;
        cursor += extent.block_count;
    }
}

fn div_ceil(value: u64, divisor: u64) -> u64 {
    if value == 0 {
        0
    } else {
        1 + (value - 1) / divisor
    }
}

fn align_block(value: u64) -> u64 {
    div_ceil(value, V2_BLOCK_SIZE) * V2_BLOCK_SIZE
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{build_image_with_mode, inspect_image, ImageMode};
    use std::fs;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn v2_layout_places_bitmap_extents_and_journal_after_v1_image() {
        let root = temp_dir("layout");
        fs::create_dir_all(root.join("var/lib/soliloquy")).unwrap();
        fs::write(root.join("var/lib/soliloquy/state.env"), vec![7_u8; 8192]).unwrap();
        let image_path = root.with_extension("solfs");
        build_image_with_mode(&root, &image_path, ImageMode::Mutable).unwrap();
        let image = inspect_image(&image_path).unwrap();

        let layout = plan_v2_layout(
            &image.header,
            &image.entries,
            image.header.image_size + 1024 * 1024,
        )
        .unwrap();

        assert_eq!(layout.block_size, V2_BLOCK_SIZE);
        assert!(layout.bitmap_offset >= image.header.image_size);
        assert!(layout.extent_table_offset > layout.bitmap_offset);
        assert!(layout.journal_offset > layout.extent_table_offset);
        assert_eq!(layout.extents.len(), 1);
        assert!(layout.free_blocks > 0);
        fs::remove_dir_all(&root).unwrap();
        fs::remove_file(&image_path).unwrap();
    }

    fn temp_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("solfsctl-v2-{name}-{}-{stamp}", std::process::id()))
    }
}
