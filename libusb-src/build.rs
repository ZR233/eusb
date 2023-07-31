use std::{env, fs};
use std::io::{Read, Write};
use std::path::PathBuf;
use regex::Regex;

fn main(){
    let mut c = cc::Build::new();
    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let config_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("include");
    let libusb_src = root.join("libusb").join("libusb");
    fs::create_dir_all(&config_dir).unwrap();
    let libusb_src_os = libusb_src.join("os");

    c.include(&config_dir);
    c.include(&libusb_src);
    c.include(&libusb_src_os);

    c.warnings(false);

    let src_files = vec![
        "core.c",
        "descriptor.c",
        "hotplug.c",
        "io.c",
        "strerror.c",
        "sync.c",
    ];

    for file in src_files.iter() {
        c.file(libusb_src.join(file));
    }
    let mut src_os_files:Vec<&str>;

    #[cfg(target_family = "unix")]
    {
        src_os_files=vec![
            "events_posix.c",
            "threads_posix.c",
        ];

        if env::var("CARGO_CFG_TARGET_OS") == Ok("linux".into())
            || env::var("CARGO_CFG_TARGET_OS") == Ok("android".into())
        {
            src_os_files.push("linux_netlink.c");
            src_os_files.push("linux_usbfs.c");
        }
    }
    if env::var("CARGO_CFG_TARGET_OS") == Ok("windows".into()) {
        #[cfg(target_env = "msvc")]
        c.flag("/source-charset:utf-8");

        c.warnings(false);
        c.define("OS_WINDOWS", Some("1"));
        src_os_files=vec![
            "events_windows.c",
            "threads_windows.c",
            "windows_common.c",
            "windows_usbdk.c",
            "windows_winusb.c",
        ];

        #[cfg(not(target_env = "msvc"))]
        {
            c.define("DEFAULT_VISIBILITY", Some(""));
            c.define("PLATFORM_WINDOWS", Some("1"));
        }
        println!("cargo:rustc-link-lib=dylib={}", "user32");
    }else {
        c.define(
            "DEFAULT_VISIBILITY",
            Some("__attribute__((visibility(\"default\")))"),
        );
    }

    for file in src_os_files.iter() {
        c.file(libusb_src_os.join(file));
    }


    let mut params = Params{
        c: &mut c,
        root: &root,
        config_dir: &config_dir,
        libusb_src_dir: &libusb_src,
        libusb_src_os_dir: &libusb_src_os,
    };
    gen_config_h(&mut params);



    let version = get_libusb_version(&libusb_src);

    println!("cargo:warning=version: {}", version);
    c.compile("usb");
}

fn get_libusb_version_one(src: &str, e: &str)->i32{
    let res = format!(r"#define LIBUSB_{} (\d+)\n#endif", e);
    let re = Regex::new(&res).unwrap();
    let caps = re.captures(&src).unwrap();
    let version = caps.get(1).unwrap().as_str();
    version.parse::<i32>().unwrap()
}
fn get_libusb_version(libusb_src: &PathBuf)->String{
    let mut f = fs::File::open(libusb_src.join("version.h")).unwrap();
    let mut h_str = String::new();
    let _ = f.read_to_string(&mut h_str).unwrap();

    let v1 = get_libusb_version_one(&h_str, "MAJOR");
    let v2 = get_libusb_version_one(&h_str, "MINOR");
    let v3 = get_libusb_version_one(&h_str, "MICRO");

    format!("{}.{}.{}", v1, v2, v3)
}
struct Params<'a>{
    c: &'a mut cc::Build,
    root: &'a PathBuf,
    config_dir: &'a PathBuf,
    libusb_src_dir: &'a PathBuf,
    libusb_src_os_dir: &'a PathBuf,
}


fn gen_config_h(params: &mut Params){
    if env::var("CARGO_CFG_TARGET_ENV") == Ok("msvc".into()) {
        fs::copy(
            params.libusb_src_dir.join("msvc").join("config.h"),
            params.config_dir.join("config.h"),
        )
            .unwrap();
    }else if  env::var("CARGO_CFG_TARGET_OS") == Ok("android".into()){
        fs::copy(
            params.libusb_src_dir.join("android").join("config.h"),
            params.config_dir.join("config.h"),
        )
            .unwrap();
    }else if  env::var("CARGO_CFG_TARGET_FAMILY") == Ok("unix".into()) {
        let mut config_h = fs::File::create(params.config_dir.join("config.h")).unwrap();
        write!(config_h, "{}", CONFIG_H_UNIX_CONTENT).unwrap();
        let version = get_libusb_version(&params.libusb_src_dir);
        let package_string = format!("libusb-1.0 {}", &version);

        params.c.define("PACKAGE_VERSION", Some(version.as_str()));
        params.c.define("PACKAGE_STRING",  Some(package_string.as_str()));
        params.c.define("VERSION", Some(version.as_str()));

    }
}



const CONFIG_H_UNIX_CONTENT: &str = r#"

// #define DEFAULT_VISIBILITY __attribute__ ((visibility ("default")))


/* Define to 1 to enable message logging. */
#define ENABLE_LOGGING 1

/* Define to 1 if you have the <asm/types.h> header file. */
/* #undef HAVE_ASM_TYPES_H */

/* Define to 1 if you have the `clock_gettime' function. */
#define HAVE_CLOCK_GETTIME 1

/* Define to 1 if you have the declaration of `EFD_CLOEXEC', and to 0 if you
   don't. */
#define HAVE_DECL_EFD_CLOEXEC 1

/* Define to 1 if you have the declaration of `EFD_NONBLOCK', and to 0 if you
   don't. */
#define HAVE_DECL_EFD_NONBLOCK 1

/* Define to 1 if you have the declaration of `TFD_CLOEXEC', and to 0 if you
   don't. */
#define HAVE_DECL_TFD_CLOEXEC 1

/* Define to 1 if you have the declaration of `TFD_NONBLOCK', and to 0 if you
   don't. */
#define HAVE_DECL_TFD_NONBLOCK 1

/* Define to 1 if you have the <dlfcn.h> header file. */
#define HAVE_DLFCN_H 1

/* Define to 1 if the system has eventfd functionality. */
#define HAVE_EVENTFD 1

/* Define to 1 if you have the <inttypes.h> header file. */
#define HAVE_INTTYPES_H 1

/* Define to 1 if you have the <IOKit/usb/IOUSBHostFamilyDefinitions.h> header
   file. */
/* #undef HAVE_IOKIT_USB_IOUSBHOSTFAMILYDEFINITIONS_H */

/* Define to 1 if you have the `udev' library (-ludev). */
// #define HAVE_LIBUDEV 1

/* Define to 1 if the system has the type `nfds_t'. */
#define HAVE_NFDS_T 1

/* Define to 1 if you have the `pipe2' function. */
#define HAVE_PIPE2 1

/* Define to 1 if you have the `pthread_condattr_setclock' function. */
#define HAVE_PTHREAD_CONDATTR_SETCLOCK 1

/* Define to 1 if you have the `pthread_setname_np' function. */
#define HAVE_PTHREAD_SETNAME_NP 1

/* Define to 1 if you have the `pthread_threadid_np' function. */
/* #undef HAVE_PTHREAD_THREADID_NP */

/* Define to 1 if you have the <stdint.h> header file. */
#define HAVE_STDINT_H 1

/* Define to 1 if you have the <stdio.h> header file. */
#define HAVE_STDIO_H 1

/* Define to 1 if you have the <stdlib.h> header file. */
#define HAVE_STDLIB_H 1

/* Define to 1 if you have the <strings.h> header file. */
#define HAVE_STRINGS_H 1

/* Define to 1 if you have the <string.h> header file. */
#define HAVE_STRING_H 1

/* Define to 1 if the system has the type `struct timespec'. */
/* #undef HAVE_STRUCT_TIMESPEC */

/* Define to 1 if you have the `syslog' function. */
/* #undef HAVE_SYSLOG */

/* Define to 1 if you have the <sys/stat.h> header file. */
#define HAVE_SYS_STAT_H 1

/* Define to 1 if you have the <sys/time.h> header file. */
#define HAVE_SYS_TIME_H 1

/* Define to 1 if you have the <sys/types.h> header file. */
#define HAVE_SYS_TYPES_H 1

/* Define to 1 if the system has timerfd functionality. */
#define HAVE_TIMERFD 1

/* Define to 1 if you have the <unistd.h> header file. */
#define HAVE_UNISTD_H 1

/* Define to the sub-directory where libtool stores uninstalled libraries. */
#define LT_OBJDIR ".libs/"

/* Name of package */
#define PACKAGE "libusb-1.0"

/* Define to the address where bug reports for this package should be sent. */
#define PACKAGE_BUGREPORT "libusb-devel@lists.sourceforge.net"

/* Define to the full name of this package. */
#define PACKAGE_NAME "libusb-1.0"

/* Define to the one symbol short name of this package. */
#define PACKAGE_TARNAME "libusb-1.0"

/* Define to the home page for this package. */
#define PACKAGE_URL "http://libusb.info"

/* Define to 1 if compiling for a POSIX platform. */
#define PLATFORM_POSIX 1

/* Define to 1 if compiling for a Windows platform. */
/* #undef PLATFORM_WINDOWS */

/* Define to the attribute for enabling parameter checks on printf-like
   functions. */
#define PRINTF_FORMAT(a, b) __attribute__ ((__format__ (__printf__, a, b)))

/* Define to 1 if all of the C90 standard headers exist (not just the ones
   required in a freestanding environment). This macro is provided for
   backward compatibility; new code need not use it. */
#define STDC_HEADERS 1

/* UMockdev hotplug code is not racy */
/* #undef UMOCKDEV_HOTPLUG */

/* Define to 1 to output logging messages to the systemwide log. */
/* #undef USE_SYSTEM_LOGGING_FACILITY */


/* Enable GNU extensions. */
#define _GNU_SOURCE 1

/* Define to the oldest supported Windows version. */
/* #undef _WIN32_WINNT */

/* Define to `__inline__' or `__inline' if that's what the C compiler
   calls it, or to nothing if 'inline' is not supported under any name.  */
#ifndef __cplusplus
/* #undef inline */
#endif

        "#;
