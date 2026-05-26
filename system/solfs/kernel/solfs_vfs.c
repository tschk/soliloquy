#include <linux/buffer_head.h>
#include <linux/fs.h>
#include <linux/module.h>
#include <linux/pagemap.h>
#include <linux/slab.h>
#include <linux/statfs.h>
#include <linux/string.h>
#include <linux/time.h>
#include <linux/uio.h>

#include "solfs_format.h"

#define SOLFS_MAX_ENTRIES 65536
#define SOLFS_MAX_NAMES_SIZE (16 * 1024 * 1024)

extern int solfs_rust_validate_header(struct solfs_disk_header header);

struct solfs_entry {
	u64 inode;
	u64 parent;
	u64 name_offset;
	u64 data_offset;
	u64 size;
	u32 name_len;
	u32 kind;
	u32 mode;
	u32 uid;
	u32 gid;
	u8 digest[32];
	const char *name;
};

struct solfs_sb_info {
	u32 entry_count;
	u64 names_size;
	struct solfs_entry *entries;
	char *names;
};

static const struct inode_operations solfs_dir_inode_ops;
static const struct file_operations solfs_dir_ops;
static const struct file_operations solfs_file_ops;

static struct solfs_sb_info *solfs_sbi(struct super_block *sb)
{
	return sb->s_fs_info;
}

static int solfs_read_bytes(struct super_block *sb, u64 offset, void *dst, size_t len)
{
	u8 *out = dst;
	u64 block;
	u32 block_offset;
	u32 chunk;
	struct buffer_head *bh;

	while (len > 0) {
		block = offset >> sb->s_blocksize_bits;
		block_offset = offset & (sb->s_blocksize - 1);
		chunk = min_t(size_t, len, sb->s_blocksize - block_offset);
		bh = sb_bread(sb, block);
		if (!bh)
			return -EIO;
		memcpy(out, bh->b_data + block_offset, chunk);
		brelse(bh);
		out += chunk;
		offset += chunk;
		len -= chunk;
	}
	return 0;
}

static struct solfs_entry *solfs_find_inode(struct super_block *sb, u64 inode)
{
	struct solfs_sb_info *sbi = solfs_sbi(sb);
	u32 i;

	for (i = 0; i < sbi->entry_count; i++) {
		if (sbi->entries[i].inode == inode)
			return &sbi->entries[i];
	}
	return NULL;
}

static struct solfs_entry *solfs_find_child(struct super_block *sb, struct solfs_entry *parent, const struct qstr *name)
{
	struct solfs_sb_info *sbi = solfs_sbi(sb);
	u32 i;

	for (i = 0; i < sbi->entry_count; i++) {
		struct solfs_entry *entry = &sbi->entries[i];

		if (entry->parent != parent->inode || entry->inode == parent->inode)
			continue;
		if (entry->name_len != name->len)
			continue;
		if (!memcmp(entry->name, name->name, name->len))
			return entry;
	}
	return NULL;
}

static struct inode *solfs_make_inode(struct super_block *sb, struct solfs_entry *entry)
{
	struct inode *inode = iget_locked(sb, entry->inode);
	umode_t mode;

	if (!inode)
		return NULL;
	if (!(inode->i_state & I_NEW))
		return inode;

	mode = entry->mode & 0777;
	if (entry->kind == SOLFS_KIND_DIR)
		mode |= S_IFDIR;
	else
		mode |= S_IFREG;

	inode->i_ino = entry->inode;
	inode->i_mode = mode;
	inode->i_uid = make_kuid(&init_user_ns, entry->uid);
	inode->i_gid = make_kgid(&init_user_ns, entry->gid);
	inode->i_size = entry->size;
	inode->i_private = entry;
	inode->i_atime = inode->i_mtime = inode->i_ctime = current_time(inode);

	if (entry->kind == SOLFS_KIND_DIR) {
		inode->i_op = &solfs_dir_inode_ops;
		inode->i_fop = &solfs_dir_ops;
		set_nlink(inode, 2);
	} else {
		inode->i_fop = &solfs_file_ops;
		set_nlink(inode, 1);
	}

	unlock_new_inode(inode);
	return inode;
}

static struct dentry *solfs_lookup(struct inode *dir, struct dentry *dentry, unsigned int flags)
{
	struct solfs_entry *parent = dir->i_private;
	struct solfs_entry *entry;
	struct inode *inode = NULL;

	if (dentry->d_name.len > 255)
		return ERR_PTR(-ENAMETOOLONG);

	entry = solfs_find_child(dir->i_sb, parent, &dentry->d_name);
	if (entry) {
		inode = solfs_make_inode(dir->i_sb, entry);
		if (!inode)
			return ERR_PTR(-ENOMEM);
	}

	d_add(dentry, inode);
	return NULL;
}

static int solfs_iterate_shared(struct file *file, struct dir_context *ctx)
{
	struct inode *inode = file_inode(file);
	struct solfs_entry *parent = inode->i_private;
	struct solfs_sb_info *sbi = solfs_sbi(inode->i_sb);
	loff_t emitted = 2;
	u32 i;

	if (!dir_emit_dots(file, ctx))
		return 0;

	for (i = 0; i < sbi->entry_count; i++) {
		struct solfs_entry *entry = &sbi->entries[i];
		unsigned int type;

		if (entry->parent != parent->inode || entry->inode == parent->inode)
			continue;
		if (ctx->pos > emitted) {
			emitted++;
			continue;
		}
		type = entry->kind == SOLFS_KIND_DIR ? DT_DIR : DT_REG;
		if (!dir_emit(ctx, entry->name, entry->name_len, entry->inode, type))
			return 0;
		ctx->pos = ++emitted;
	}
	return 0;
}

static ssize_t solfs_read_iter(struct kiocb *iocb, struct iov_iter *to)
{
	struct inode *inode = file_inode(iocb->ki_filp);
	struct solfs_entry *entry = inode->i_private;
	struct super_block *sb = inode->i_sb;
	size_t wanted;
	size_t done = 0;
	u8 *buffer;

	if (iocb->ki_pos >= entry->size)
		return 0;

	wanted = min_t(u64, iov_iter_count(to), entry->size - iocb->ki_pos);
	buffer = kmalloc(PAGE_SIZE, GFP_KERNEL);
	if (!buffer)
		return -ENOMEM;

	while (done < wanted) {
		size_t chunk = min_t(size_t, PAGE_SIZE, wanted - done);
		int ret = solfs_read_bytes(sb, entry->data_offset + iocb->ki_pos + done, buffer, chunk);

		if (ret) {
			kfree(buffer);
			return ret;
		}
		if (copy_to_iter(buffer, chunk, to) != chunk) {
			kfree(buffer);
			return -EFAULT;
		}
		done += chunk;
	}

	kfree(buffer);
	iocb->ki_pos += done;
	return done;
}

static const struct inode_operations solfs_dir_inode_ops = {
	.lookup = solfs_lookup,
};

static const struct file_operations solfs_dir_ops = {
	.owner = THIS_MODULE,
	.iterate_shared = solfs_iterate_shared,
	.llseek = generic_file_llseek,
};

static const struct file_operations solfs_file_ops = {
	.owner = THIS_MODULE,
	.read_iter = solfs_read_iter,
	.llseek = generic_file_llseek,
};

static int solfs_load_entries(struct super_block *sb, struct solfs_disk_header *header)
{
	struct solfs_sb_info *sbi = solfs_sbi(sb);
	u64 entries_offset = le64_to_cpu(header->entries_offset);
	u64 names_offset = le64_to_cpu(header->names_offset);
	u64 data_offset = le64_to_cpu(header->data_offset);
	u64 image_size = le64_to_cpu(header->image_size);
	u32 entry_count = le32_to_cpu(header->entry_count);
	u64 names_size;
	u32 i;
	int ret;

	if (entry_count == 0 || entry_count > SOLFS_MAX_ENTRIES)
		return -EINVAL;
	names_size = data_offset - names_offset;
	if (names_size > SOLFS_MAX_NAMES_SIZE)
		return -EINVAL;

	sbi->entry_count = entry_count;
	sbi->names_size = names_size;
	sbi->entries = kcalloc(entry_count, sizeof(*sbi->entries), GFP_KERNEL);
	if (!sbi->entries)
		return -ENOMEM;

	sbi->names = kmalloc(names_size + 1, GFP_KERNEL);
	if (!sbi->names)
		return -ENOMEM;

	ret = solfs_read_bytes(sb, names_offset, sbi->names, names_size);
	if (ret)
		return ret;
	sbi->names[names_size] = '\0';

	for (i = 0; i < entry_count; i++) {
		struct solfs_disk_entry disk;
		struct solfs_entry *entry = &sbi->entries[i];

		ret = solfs_read_bytes(sb, entries_offset + i * sizeof(disk), &disk, sizeof(disk));
		if (ret)
			return ret;

		entry->inode = le64_to_cpu(disk.inode);
		entry->parent = le64_to_cpu(disk.parent);
		entry->name_offset = le64_to_cpu(disk.name_offset);
		entry->name_len = le32_to_cpu(disk.name_len);
		entry->kind = le32_to_cpu(disk.kind);
		entry->mode = le32_to_cpu(disk.mode);
		entry->uid = le32_to_cpu(disk.uid);
		entry->gid = le32_to_cpu(disk.gid);
		entry->data_offset = le64_to_cpu(disk.data_offset);
		entry->size = le64_to_cpu(disk.size);
		memcpy(entry->digest, disk.digest, sizeof(entry->digest));

		if (entry->name_offset > names_size || entry->name_len > names_size - entry->name_offset)
			return -EINVAL;
		if (entry->kind != SOLFS_KIND_DIR && entry->kind != SOLFS_KIND_FILE)
			return -EINVAL;
		if (entry->mode & 0222)
			return -EINVAL;
		if (entry->kind == SOLFS_KIND_DIR && (entry->size || entry->data_offset))
			return -EINVAL;
		if (entry->kind == SOLFS_KIND_FILE && (entry->data_offset > image_size || entry->size > image_size - entry->data_offset))
			return -EINVAL;
		if (entry->name_len && memchr(sbi->names + entry->name_offset, '/', entry->name_len))
			return -EINVAL;
		entry->name = sbi->names + entry->name_offset;
	}

	if (sbi->entries[0].inode != 1 || sbi->entries[0].parent != 1 || sbi->entries[0].kind != SOLFS_KIND_DIR)
		return -EINVAL;
	if (sbi->entries[0].name_len != 0)
		return -EINVAL;

	for (i = 0; i < entry_count; i++) {
		u32 j;
		bool parent_found = sbi->entries[i].inode == 1 && sbi->entries[i].parent == 1;

		for (j = 0; j < entry_count; j++) {
			if (i != j && sbi->entries[i].inode == sbi->entries[j].inode)
				return -EINVAL;
			if (sbi->entries[j].inode == sbi->entries[i].parent)
				parent_found = true;
		}
		if (!parent_found)
			return -EINVAL;
	}

	return 0;
}

static void solfs_put_super(struct super_block *sb)
{
	struct solfs_sb_info *sbi = solfs_sbi(sb);

	if (!sbi)
		return;
	kfree(sbi->entries);
	kfree(sbi->names);
	kfree(sbi);
	sb->s_fs_info = NULL;
}

static int solfs_statfs(struct dentry *dentry, struct kstatfs *buf)
{
	struct super_block *sb = dentry->d_sb;
	struct solfs_sb_info *sbi = solfs_sbi(sb);

	buf->f_type = SOLFS_SUPER_MAGIC;
	buf->f_bsize = sb->s_blocksize;
	buf->f_blocks = 1;
	buf->f_bfree = 0;
	buf->f_bavail = 0;
	buf->f_files = sbi ? sbi->entry_count : 0;
	buf->f_ffree = 0;
	buf->f_namelen = 255;
	return 0;
}

static const struct super_operations solfs_super_ops = {
	.statfs = solfs_statfs,
	.put_super = solfs_put_super,
	.drop_inode = generic_delete_inode,
};

static int solfs_fill_super(struct super_block *sb, void *data, int silent)
{
	struct buffer_head *bh;
	struct inode *root_inode;
	struct solfs_disk_header header;
	struct solfs_sb_info *sbi;
	int ret;

	sb_set_blocksize(sb, 4096);
	bh = sb_bread(sb, 0);
	if (!bh)
		return -EINVAL;

	memcpy(&header, bh->b_data, sizeof(header));
	brelse(bh);

	ret = solfs_rust_validate_header(header);
	if (ret)
		return ret;

	sbi = kzalloc(sizeof(*sbi), GFP_KERNEL);
	if (!sbi)
		return -ENOMEM;

	sb->s_fs_info = sbi;
	sb->s_magic = SOLFS_SUPER_MAGIC;
	sb->s_op = &solfs_super_ops;
	sb->s_maxbytes = MAX_LFS_FILESIZE;
	sb->s_flags |= SB_RDONLY;

	ret = solfs_load_entries(sb, &header);
	if (ret)
		goto err;

	root_inode = solfs_make_inode(sb, solfs_find_inode(sb, 1));
	if (!root_inode) {
		ret = -ENOMEM;
		goto err;
	}

	sb->s_root = d_make_root(root_inode);
	if (!sb->s_root) {
		ret = -ENOMEM;
		goto err;
	}

	return 0;

err:
	solfs_put_super(sb);
	return ret;
}

static struct dentry *solfs_mount(struct file_system_type *fs_type, int flags, const char *dev_name, void *data)
{
	return mount_bdev(fs_type, flags | SB_RDONLY, dev_name, data, solfs_fill_super);
}

static struct file_system_type solfs_fs_type = {
	.owner = THIS_MODULE,
	.name = "solfs",
	.mount = solfs_mount,
	.kill_sb = kill_block_super,
	.fs_flags = FS_REQUIRES_DEV,
};

static int __init solfs_init(void)
{
	return register_filesystem(&solfs_fs_type);
}

static void __exit solfs_exit(void)
{
	unregister_filesystem(&solfs_fs_type);
}

module_init(solfs_init);
module_exit(solfs_exit);
MODULE_LICENSE("GPL");
MODULE_DESCRIPTION("Soliloquy filesystem VFS shim");
