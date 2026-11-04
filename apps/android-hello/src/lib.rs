//! Android entry point: the APK's host loads `libandroid_hello.so` and calls
//! the `rustolonia_android_main` export this macro declares.

pub mod ui;

rustolonia::android_main!(ui::build);
