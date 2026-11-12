//! JNI bootstrap for CPAL and refresh-only Android Keystore persistence.
use jni::{
    JNIEnv, JavaVM,
    objects::{GlobalRef, JClass, JObject, JString, JValue},
};
use std::sync::OnceLock;
use xcloud_api::token_store::{StreamTokens, TokenStore};
struct Context {
    vm: JavaVM,
    class: GlobalRef,
    _application: GlobalRef,
}
static CONTEXT: OnceLock<Context> = OnceLock::new();
pub fn ready() -> bool {
    CONTEXT.get().is_some()
}
// Called by the template Activity before Godot's main loop; never touches Godot.
pub fn initialize(mut env: JNIEnv, class: JClass, app: JObject, codec: JString) {
    let result = (|| -> jni::errors::Result<Context> {
        let vm = env.get_java_vm()?;
        let application = env.new_global_ref(&app)?;
        let class = env.new_global_ref(class)?;
        let name: String = env.get_string(&codec)?.into();
        if !name.is_empty() {
            xcloud_media::android::set_decoder_name(name);
        }
        Ok(Context {
            vm,
            class,
            _application: application,
        })
    })();
    if let Ok(context) = result {
        if CONTEXT.get().is_none() {
            // SAFETY: these global VM/context references remain valid for process lifetime.
            unsafe {
                ndk_context::initialize_android_context(
                    context.vm.get_java_vm_pointer().cast(),
                    context._application.as_obj().as_raw().cast(),
                );
            }
            let _ = CONTEXT.set(context);
        }
    } else {
        let _ = env.exception_clear();
    }
}
fn credential(operation: i32, value: &str) -> xcloud_api::Result<String> {
    let call = (|| -> jni::errors::Result<String> {
        let context = CONTEXT
            .get()
            .ok_or(jni::errors::Error::NullPtr("Android context"))?;
        let mut env = context.vm.attach_current_thread()?;
        let class = JClass::from(env.new_local_ref(context.class.as_obj())?);
        let value = env.new_string(value)?;
        let result = env.call_static_method(
            class,
            "credential",
            "(ILjava/lang/String;)Ljava/lang/String;",
            &[JValue::Int(operation), JValue::Object(value.as_ref())],
        );
        match result {
            Ok(result) => {
                let string = JString::from(result.l()?);
                Ok(env.get_string(&string)?.into())
            }
            Err(error) => {
                let _ = env.exception_clear();
                Err(error)
            }
        }
    })();
    call.map_err(|_| {
        xcloud_api::Error::TokenStore("Android secure credential operation failed".into())
    })
}
pub struct SecureStore;
impl TokenStore for SecureStore {
    fn save_refresh_token(&self, value: &str) -> xcloud_api::Result<()> {
        credential(1, value).map(|_| ())
    }
    fn load_refresh_token(&self) -> xcloud_api::Result<String> {
        credential(0, "")
    }
    fn clear_all(&self) -> xcloud_api::Result<()> {
        credential(2, "").map(|_| ())
    }
    // Wrapped by SessionTokenStore; temporary credentials never reach durable storage.
    fn save_msa_token(&self, _: &str) -> xcloud_api::Result<()> {
        Ok(())
    }
    fn load_msa_token(&self) -> xcloud_api::Result<String> {
        Ok(String::new())
    }
    fn save_lpt_token(
        &self,
        _: &str,
        _: Option<chrono::DateTime<chrono::Utc>>,
    ) -> xcloud_api::Result<()> {
        Ok(())
    }
    fn load_lpt_token(&self) -> xcloud_api::Result<String> {
        Ok(String::new())
    }
    fn lpt_token_is_fresh(&self) -> xcloud_api::Result<bool> {
        Ok(false)
    }
    fn save_stream_tokens(&self, _: &StreamTokens) -> xcloud_api::Result<()> {
        Ok(())
    }
    fn load_stream_tokens(&self) -> xcloud_api::Result<StreamTokens> {
        Ok(Default::default())
    }
}
