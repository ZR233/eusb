# Libusb Rust Bindings

The `libusb-src` crate provides declarations and linkage for the `libusb` C library.

libusb version: 1.0.26

tested on ubuntu and windows


# example

```rust
use libusb_src as ffi;

fn main(){
    unsafe {
        let mut context = std::mem::MaybeUninit::<*mut ffi::libusb_context>::uninit();
        match ffi::libusb_init(context.as_mut_ptr()){
            0 => (),
            err => panic!("Failed to init libusb {}", err),
        }
        let mut list = std::mem::MaybeUninit::<*const *mut ffi::libusb_device>::uninit();
        let list_size = ffi::libusb_get_device_list(context.assume_init(), list.as_mut_ptr());
        if list_size < 0  {
            panic!("Failed to get device list {} {:p}", -list_size, list.assume_init());
        }else { 
            println!("Usb device count: {}", list_size);
        }
        ffi::libusb_free_device_list(list.assume_init(), 1);
        ffi::libusb_exit(context.assume_init());
    }
}

```

# Cross Compile

support windows linux and android, not test ios and mac.