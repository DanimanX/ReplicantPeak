//! Loads the built plugin library through CLAP and checks what a host sees.
//!
//! Unit tests cannot reach this: `Buffer` and the wrapper's event plumbing are
//! private to nih-plug, so "the host never sends me notes" is invisible from
//! inside the crate. This test does what a DAW does — `dlopen` the cdylib,
//! call `clap_entry.init`, instantiate the plugin from the factory and ask its
//! `clap.note-ports` extension how many note inputs it declares.
//!
//! It needs the library to have been built:
//!
//! ```text
//! cargo build -p peak-plugin && cargo test -p peak-plugin --test clap_probe
//! ```
//!
//! Set `PEAK_PLUGIN_LIB` to point at a specific library (for example a bundle
//! produced by `cargo xtask bundle`).

use std::ffi::{c_void, CStr, CString};
use std::path::{Path, PathBuf};

use clap_sys::entry::clap_plugin_entry;
use clap_sys::ext::audio_ports::clap_plugin_audio_ports;
use clap_sys::ext::note_ports::clap_plugin_note_ports;
use clap_sys::factory::plugin_factory::{clap_plugin_factory, CLAP_PLUGIN_FACTORY_ID};
use clap_sys::host::clap_host;
use clap_sys::plugin::{clap_plugin, clap_plugin_descriptor};
use clap_sys::version::clap_version;
use libloading::Library;

const CLAP_VERSION: clap_version = clap_version {
    major: 1,
    minor: 2,
    revision: 2,
};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn library_path() -> Option<PathBuf> {
    if let Some(explicit) = std::env::var_os("PEAK_PLUGIN_LIB") {
        // dlopen() does not search the working directory, and cargo runs tests
        // from the package directory, so a relative path is resolved against
        // the workspace root before it reaches the loader.
        let path = PathBuf::from(explicit);
        let path = if path.is_absolute() {
            path
        } else {
            workspace_root().join(path)
        };
        return Some(path.canonicalize().unwrap_or(path));
    }
    // target/<profile>/libpeak_plugin.so | .dylib | peak_plugin.dll
    let target = workspace_root().join("target");
    let names: &[&str] = if cfg!(target_os = "windows") {
        &["peak_plugin.dll"]
    } else if cfg!(target_os = "macos") {
        &["libpeak_plugin.dylib"]
    } else {
        &["libpeak_plugin.so"]
    };
    for profile in ["debug", "release"] {
        for name in names {
            let candidate = target.join(profile).join(name);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
    }
    None
}

unsafe extern "C" fn no_host_extension(
    _host: *const clap_host,
    _id: *const std::os::raw::c_char,
) -> *const c_void {
    std::ptr::null()
}

unsafe extern "C" fn no_request(_host: *const clap_host) {}

/// A minimal host: enough for the wrapper to instantiate, and nothing more.
///
/// The strings are owned by the caller so the pointers stay valid for as long
/// as the plugin can read them.
struct StubHost {
    _name: CString,
    _vendor: CString,
    _version: CString,
    host: clap_host,
}

fn stub_host() -> StubHost {
    let name = cstr("ReplicantPeak probe");
    let vendor = cstr("test");
    let version = cstr("0");
    let host = clap_host {
        clap_version: CLAP_VERSION,
        host_data: std::ptr::null_mut(),
        name: name.as_ptr(),
        vendor: vendor.as_ptr(),
        url: std::ptr::null(),
        version: version.as_ptr(),
        get_extension: Some(no_host_extension),
        request_restart: Some(no_request),
        request_process: Some(no_request),
        request_callback: Some(no_request),
    };
    StubHost {
        _name: name,
        _vendor: vendor,
        _version: version,
        host,
    }
}

fn cstr(text: &str) -> CString {
    CString::new(text).expect("no interior NUL")
}

#[test]
fn the_built_plugin_declares_a_note_input() {
    let Some(path) = library_path() else {
        // `cargo test` alone does not produce the cdylib, so a plain workspace
        // test run would fail for the wrong reason. CI sets
        // PEAK_REQUIRE_CLAP_PROBE=1 after building the library, so a missing
        // artifact cannot slip through there.
        let required = std::env::var_os("PEAK_REQUIRE_CLAP_PROBE").is_some();
        assert!(
            !required,
            "PEAK_REQUIRE_CLAP_PROBE is set but no plugin library was found; \
             run `cargo build -p peak-plugin` first"
        );
        eprintln!(
            "skipping the CLAP probe: run `cargo build -p peak-plugin` first, or set \
             PEAK_PLUGIN_LIB"
        );
        return;
    };

    // SAFETY: the plugin library follows the CLAP ABI; every pointer below is
    // checked for null before it is dereferenced, and the structures outlive
    // their use. `host` is kept alive for as long as the plugin instance.
    unsafe {
        let lib = Library::new(&path).expect("cannot dlopen the plugin");
        // libloading hands back the symbol's address; `clap_entry` is a struct
        // object, so dereference it into the ABI type.
        let entry_ptr: *const clap_plugin_entry = *lib
            .get::<*const clap_plugin_entry>(b"clap_entry\0")
            .expect("the library does not export clap_entry; is this the CDylib artifact?");
        let entry: &clap_plugin_entry = &*entry_ptr;

        let init = entry.init.expect("clap_entry.init");
        assert!(init(cstr(&path.to_string_lossy()).as_ptr()), "init failed");

        let get_factory = entry.get_factory.expect("clap_entry.get_factory");
        let factory_ptr = get_factory(CLAP_PLUGIN_FACTORY_ID.as_ptr());
        assert!(!factory_ptr.is_null(), "no plugin factory");
        let factory: &clap_plugin_factory = &*factory_ptr.cast::<clap_plugin_factory>();

        let count_plugins = factory.get_plugin_count.expect("get_plugin_count");
        assert_eq!(count_plugins(factory), 1, "expected exactly one plugin");

        let get_descriptor = factory
            .get_plugin_descriptor
            .expect("get_plugin_descriptor");
        let descriptor: &clap_plugin_descriptor = &*get_descriptor(factory, 0);
        let id = CStr::from_ptr(descriptor.id);
        assert_eq!(id.to_str().unwrap(), "com.danimanx.replicant-peak");

        let host = stub_host();
        let create = factory.create_plugin.expect("create_plugin");
        let plugin_ptr = create(factory, &host.host, descriptor.id);
        assert!(!plugin_ptr.is_null(), "create_plugin returned null");
        let plugin: &clap_plugin = &*plugin_ptr;

        let plugin_init = plugin.init.expect("plugin.init");
        assert!(plugin_init(plugin), "plugin init failed");

        // The regression this test exists for: no note input means the host
        // never sends a note, which is exactly the silent-plugin report.
        let ports_ptr = plugin.get_extension.expect("plugin.get_extension")(
            plugin,
            cstr("clap.note-ports").as_ptr(),
        );
        assert!(!ports_ptr.is_null(), "the plugin has no clap.note-ports");
        let ports: &clap_plugin_note_ports = &*ports_ptr.cast::<clap_plugin_note_ports>();
        let count = ports.count.expect("note_ports.count");
        let inputs = count(plugin, true);
        assert!(
            inputs >= 1,
            "the plugin declares {inputs} note inputs: a host will never send it MIDI"
        );

        let audio_ptr = plugin.get_extension.expect("plugin.get_extension")(
            plugin,
            cstr("clap.audio-ports").as_ptr(),
        );
        assert!(!audio_ptr.is_null(), "the plugin has no clap.audio-ports");
        let audio: &clap_plugin_audio_ports = &*audio_ptr.cast::<clap_plugin_audio_ports>();
        let count_audio = audio.count.expect("audio_ports.count");
        assert!(count_audio(plugin, false) >= 1, "no audio output port");

        plugin.destroy.expect("plugin.destroy")(plugin);
        entry.deinit.expect("clap_entry.deinit")();
    }
}
