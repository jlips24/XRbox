#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub extern "system" fn Java_org_xrbox_AndroidServices_initializeNative(
    env: jni::JNIEnv,
    class: jni::objects::JClass,
    application: jni::objects::JObject,
    codec: jni::objects::JString,
) {
    xrbox_core::android::initialize(env, class, application, codec);
}
