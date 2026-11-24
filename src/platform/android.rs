//! JNI bridge: private storage directory and runtime microphone permission.
//!
//! The Activity is published by `tao` through `ndk-context` when the WebView
//! starts, so every call below can reach it without any Kotlin glue.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use jni::objects::{JObject, JValue};
use jni::JavaVM;

const RECORD_AUDIO: &str = "android.permission.RECORD_AUDIO";
const PERMISSION_GRANTED: i32 = 0;
const REQUEST_CODE: i32 = 41;

static REQUESTED: AtomicBool = AtomicBool::new(false);

/// Runs `body` with the current Activity. Returns `None` when the runtime is
/// not reachable, which happens on the very first frames of the process.
macro_rules! with_activity {
    (|$env:ident, $activity:ident| $body:block) => {{
        let context = ndk_context::android_context();
        let vm = unsafe { JavaVM::from_raw(context.vm().cast()) }.ok();
        vm.and_then(|vm| {
            vm.attach_current_thread().ok().and_then(|mut $env| {
                let $activity = unsafe { JObject::from_raw(context.context().cast()) };
                $body
            })
        })
    }};
}

pub fn app_files_dir() -> Option<PathBuf> {
    with_activity!(|env, activity| {
        let file = env
            .call_method(activity, "getFilesDir", "()Ljava/io/File;", &[])
            .ok()?
            .l()
            .ok()?;
        let path = env
            .call_method(&file, "getAbsolutePath", "()Ljava/lang/String;", &[])
            .ok()?
            .l()
            .ok()?;
        let path: jni::objects::JString = path.into();
        let path = env.get_string(&path).ok()?;
        Some(PathBuf::from(String::from(path)))
    })
}

pub fn has_microphone_permission() -> bool {
    with_activity!(|env, activity| {
        let permission = env.new_string(RECORD_AUDIO).ok()?;
        let permission: JObject = permission.into();
        let granted = env
            .call_method(
                activity,
                "checkSelfPermission",
                "(Ljava/lang/String;)I",
                &[JValue::Object(&permission)],
            )
            .ok()?
            .i()
            .ok()?;
        Some(granted == PERMISSION_GRANTED)
    })
    .unwrap_or(false)
}

pub fn request_microphone_permission() {
    REQUESTED.store(true, Ordering::SeqCst);
    with_activity!(|env, activity| {
        let permission = env.new_string(RECORD_AUDIO).ok()?;
        let permission: JObject = permission.into();
        let permissions = env
            .new_object_array(1, "java/lang/String", &permission)
            .ok()?;
        let permissions: JObject = permissions.into();
        env.call_method(
            activity,
            "requestPermissions",
            "([Ljava/lang/String;I)V",
            &[JValue::Object(&permissions), JValue::Int(REQUEST_CODE)],
        )
        .ok()?;
        Some(())
    });
}

/// True when the user refused and checked "don't ask again", so the only way
/// forward is the Android settings screen.
pub fn is_permission_blocked() -> bool {
    if has_microphone_permission() || !REQUESTED.load(Ordering::SeqCst) {
        return false;
    }
    let rationale = with_activity!(|env, activity| {
        let permission = env.new_string(RECORD_AUDIO).ok()?;
        let permission: JObject = permission.into();
        let shown = env
            .call_method(
                activity,
                "shouldShowRequestPermissionRationale",
                "(Ljava/lang/String;)Z",
                &[JValue::Object(&permission)],
            )
            .ok()?
            .z()
            .ok()?;
        Some(shown)
    });
    matches!(rationale, Some(false))
}
