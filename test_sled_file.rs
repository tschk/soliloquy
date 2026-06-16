use sled;
use tempfile::tempdir;

fn main() {
    let temp_dir = tempdir().unwrap();
    let db = sled::open(temp_dir.path()).unwrap();

    std::fs::remove_dir_all(temp_dir.path()).unwrap();
    std::fs::File::create(temp_dir.path()).unwrap();

    // will this fail?
    let res = db.insert(b"test", b"val");
    println!("res: {:?}", res);

    // what if we drop the db and open a new one on a file?
    let res2 = sled::open(temp_dir.path());
    println!("res2: {:?}", res2);
}
