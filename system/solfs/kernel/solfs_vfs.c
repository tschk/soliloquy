#include <linux/buffer_head.h>
#include <linux/fs.h>
#include <linux/module.h>
#include <linux/pagemap.h>
#include <linux/statfs.h>
#include <linux/time.h>

#include "solfs_format.h"

extern int solfs_rust_validate_header(struct solfs_disk_header header);

static struct inode *solfs_make_inode(struct super_block *sb, umode_t mode)
{
	struct inode *inode = new_inode(sb);

	if (!inode)
		return NULL;

	inode->i_ino = 1;
	inode->i_mode = mode;
	inode->i_uid = GLOBAL_ROOT_UID;
	inode->i_gid = GLOBAL_ROOT_GID;
	inode->i_atime = inode->i_mtime = inode->i_ctime = current_time(inode);
	return inode;
}

static int solfs_statfs(struct dentry *dentry, struct kstatfs *buf)
{
	struct super_block *sb = dentry->d_sb;

	buf->f_type = SOLFS_SUPER_MAGIC;
	buf->f_bsize = sb->s_blocksize;
	buf->f_blocks = 1;
	buf->f_bfree = 0;
	buf->f_bavail = 0;
	buf->f_files = 1;
	buf->f_ffree = 0;
	buf->f_namelen = 255;
	return 0;
}

static const struct super_operations solfs_super_ops = {
	.statfs = solfs_statfs,
	.drop_inode = generic_delete_inode,
};

static int solfs_fill_super(struct super_block *sb, void *data, int silent)
{
	struct buffer_head *bh;
	struct inode *root_inode;
	struct solfs_disk_header header;
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

	sb->s_magic = SOLFS_SUPER_MAGIC;
	sb->s_op = &solfs_super_ops;
	sb->s_maxbytes = MAX_LFS_FILESIZE;
	sb->s_flags |= SB_RDONLY;

	root_inode = solfs_make_inode(sb, S_IFDIR | 0555);
	if (!root_inode)
		return -ENOMEM;

	root_inode->i_op = &simple_dir_inode_operations;
	root_inode->i_fop = &simple_dir_operations;
	sb->s_root = d_make_root(root_inode);
	if (!sb->s_root)
		return -ENOMEM;

	return 0;
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
