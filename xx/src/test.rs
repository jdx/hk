// use std::path::PathBuf;

// pub struct TestContext {
//     dir: tempfile::TempDir,
// }
// static CTX: std::sync::Mutex<Option<TestContext>> = std::sync::Mutex::new(None);
//
// pub fn setup() {
//     let dir = tempfile::tempdir().unwrap();
//     let context = TestContext { dir };
//     *CTX.lock().unwrap() = Some(context);
// }
//
// pub fn get_tmpd() -> PathBuf {
//     CTX.lock()
//         .unwrap()
//         .as_ref()
//         .unwrap()
//         .dir
//         .path()
//         .to_path_buf()
// }
//
// pub fn teardown() {
//     *CTX.lock().unwrap() = None;
// }

pub fn tempdir() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}
