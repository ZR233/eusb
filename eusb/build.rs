use std::env;

fn main(){
    let target_env = env::var("CARGO_CFG_TARGET_ENV").unwrap();
    if target_env.as_str() == "ohos" {
        println!("cargo:rustc-cfg=ohos");
    }else {
        println!("cargo:rustc-cfg=libusb");
    }
}