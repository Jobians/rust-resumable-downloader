use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

use jni::JNIEnv;
use jni::objects::{GlobalRef, JObject, JString, JValue};
use jni::sys::{jint, jlong};
use tokio_util::sync::CancellationToken;

use crate::{DownloadError, Progress, download};

const STATUS_OK: jint = 0;
const STATUS_CANCELLED: jint = 1;
const STATUS_ERROR: jint = 2;

static REGISTRY: OnceLock<Mutex<HashMap<jlong, CancellationToken>>> = OnceLock::new();
static NEXT_HANDLE: AtomicI64 = AtomicI64::new(1);

fn registry() -> MutexGuard<'static, HashMap<jlong, CancellationToken>> {
    REGISTRY
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn call_progress(env: &mut JNIEnv, callback: &GlobalRef, progress: &Progress) {
    let eta = progress
        .eta_secs
        .map(|seconds| seconds.round() as i64)
        .unwrap_or(-1);
    let size = match env.new_string(progress.formatted_size()) {
        Ok(value) => JObject::from(value),
        Err(_) => return,
    };
    let speed = match env.new_string(progress.formatted_speed()) {
        Ok(value) => JObject::from(value),
        Err(_) => return,
    };
    let eta_text = match env.new_string(progress.formatted_eta()) {
        Ok(value) => JObject::from(value),
        Err(_) => return,
    };
    let args = [
        JValue::Long(progress.downloaded as i64),
        JValue::Long(progress.total as i64),
        JValue::Double(progress.speed_bps),
        JValue::Long(eta),
        JValue::Object(&size),
        JValue::Object(&speed),
        JValue::Object(&eta_text),
    ];

    if env
        .call_method(
            callback.as_obj(),
            "onProgress",
            "(JJDJLjava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
            &args,
        )
        .is_err()
    {
        let _ = env.exception_clear();
    }
}

fn call_string_method(env: &mut JNIEnv, callback: &JObject, method: &str, message: &str) {
    let message = match env.new_string(message) {
        Ok(message) => JObject::from(message),
        Err(_) => return,
    };
    let args = [JValue::Object(&message)];

    if env
        .call_method(callback, method, "(Ljava/lang/String;)V", &args)
        .is_err()
    {
        let _ = env.exception_clear();
    }
}

fn call_terminal(env: &mut JNIEnv, callback: &JObject, method: &str) {
    if env.call_method(callback, method, "()V", &[]).is_err() {
        let _ = env.exception_clear();
    }
}

fn call_complete(
    env: &mut JNIEnv,
    callback: &JObject,
    output_path: &str,
    filename: &str,
    content_type: &str,
) {
    let output_path = match env.new_string(output_path) {
        Ok(value) => JObject::from(value),
        Err(_) => return,
    };
    let filename = match env.new_string(filename) {
        Ok(value) => JObject::from(value),
        Err(_) => return,
    };
    let content_type = match env.new_string(content_type) {
        Ok(value) => JObject::from(value),
        Err(_) => return,
    };
    let args = [
        JValue::Object(&output_path),
        JValue::Object(&filename),
        JValue::Object(&content_type),
    ];

    if env
        .call_method(
            callback,
            "onComplete",
            "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V",
            &args,
        )
        .is_err()
    {
        let _ = env.exception_clear();
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jt_rget_RgetNative_nativeCreateHandle(
    _env: JNIEnv,
    _this: JObject,
) -> jlong {
    let handle = NEXT_HANDLE.fetch_add(1, Ordering::Relaxed);
    registry().insert(handle, CancellationToken::new());
    handle
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jt_rget_RgetNative_nativeCancel(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    if let Some(cancel) = registry().get(&handle).cloned() {
        cancel.cancel();
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jt_rget_RgetNative_nativeFreeHandle(
    _env: JNIEnv,
    _this: JObject,
    handle: jlong,
) {
    registry().remove(&handle);
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_dev_jt_rget_RgetNative_nativeDownload(
    mut env: JNIEnv,
    _this: JObject,
    handle: jlong,
    url: JString,
    output_path: JString,
    callback: JObject,
) -> jint {
    let cancel = match registry().get(&handle).cloned() {
        Some(cancel) => cancel,
        None => {
            call_string_method(
                &mut env,
                &callback,
                "onError",
                "invalid or expired download handle",
            );
            return STATUS_ERROR;
        }
    };

    let url = match env.get_string(&url) {
        Ok(url) => String::from(url),
        Err(err) => {
            call_string_method(
                &mut env,
                &callback,
                "onError",
                &format!("invalid download URL: {err}"),
            );
            return STATUS_ERROR;
        }
    };

    let output_path = match env.get_string(&output_path) {
        Ok(output_path) => String::from(output_path),
        Err(err) => {
            call_string_method(
                &mut env,
                &callback,
                "onError",
                &format!("invalid output path: {err}"),
            );
            return STATUS_ERROR;
        }
    };

    let callback = match env.new_global_ref(&callback) {
        Ok(callback) => callback,
        Err(err) => {
            call_string_method(
                &mut env,
                &callback,
                "onError",
                &format!("could not retain callback: {err}"),
            );
            return STATUS_ERROR;
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            call_string_method(
                &mut env,
                callback.as_obj(),
                "onError",
                &format!("could not start async runtime: {err}"),
            );
            return STATUS_ERROR;
        }
    };

    let path = PathBuf::from(&output_path);
    let env_ref = &mut env;
    let callback_ref = &callback;
    let result = runtime.block_on(download(&url, &path, cancel, |progress| {
        call_progress(env_ref, callback_ref, &progress);
    }));

    match result {
        Ok(outcome) => {
            let content_type = outcome.content_type.unwrap_or_default();
            call_complete(
                &mut env,
                callback.as_obj(),
                &output_path,
                &outcome.filename,
                &content_type,
            );
            STATUS_OK
        }
        Err(DownloadError::Cancelled) => {
            call_terminal(&mut env, callback.as_obj(), "onCancelled");
            STATUS_CANCELLED
        }
        Err(err) => {
            call_string_method(&mut env, callback.as_obj(), "onError", &err.to_string());
            STATUS_ERROR
        }
    }
}
