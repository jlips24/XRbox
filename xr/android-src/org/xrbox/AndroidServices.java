package org.xrbox;

import android.content.Context;
import android.content.SharedPreferences;
import android.media.MediaCodecInfo;
import android.media.MediaCodecList;
import android.security.keystore.KeyGenParameterSpec;
import android.security.keystore.KeyProperties;
import android.util.Base64;
import java.security.KeyStore;
import javax.crypto.Cipher;
import javax.crypto.KeyGenerator;
import javax.crypto.SecretKey;
import javax.crypto.spec.GCMParameterSpec;

/** Platform services only. Xbox authentication/protocol code remains in Rust. */
public final class AndroidServices {
    private static Context context;
    private static final String ALIAS = "xrbox.refresh.v1";
    private static native void initializeNative(Context application, String codec);
    public static void initialize(Context application) {
        context = application.getApplicationContext();
        System.loadLibrary("xrbox_godot");
        String hardware = "";
        for (MediaCodecInfo codec : new MediaCodecList(MediaCodecList.ALL_CODECS).getCodecInfos()) {
            if (codec.isEncoder() || !codec.isHardwareAccelerated() || codec.isSoftwareOnly() || codec.getName().endsWith(".secure")) continue;
            for (String type : codec.getSupportedTypes()) {
                if (type.equalsIgnoreCase("video/avc") && !codec.getCapabilitiesForType(type).isFeatureRequired(MediaCodecInfo.CodecCapabilities.FEATURE_SecurePlayback)) { hardware = codec.getName(); break; }
            }
            if (!hardware.isEmpty()) break;
        }
        initializeNative(context, hardware);
    }
    private static SecretKey key() throws Exception {
        KeyStore store = KeyStore.getInstance("AndroidKeyStore");
        store.load(null);
        if (!store.containsAlias(ALIAS)) {
            KeyGenerator generator = KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore");
            generator.init(new KeyGenParameterSpec.Builder(ALIAS, KeyProperties.PURPOSE_ENCRYPT | KeyProperties.PURPOSE_DECRYPT)
                .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE).build());
            return generator.generateKey();
        }
        return (SecretKey) store.getKey(ALIAS, null);
    }
    /** Only encrypted refresh credentials are durable. Throws on any storage error. */
    public static synchronized String credential(int operation, String value) throws Exception {
        SharedPreferences prefs = context.getSharedPreferences("xrbox.auth", Context.MODE_PRIVATE);
        if (operation == 2) {
            if (!prefs.edit().clear().commit()) throw new IllegalStateException("credential clear failed");
            return "";
        }
        if (operation == 1) {
            Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
            cipher.init(Cipher.ENCRYPT_MODE, key());
            String iv = Base64.encodeToString(cipher.getIV(), Base64.NO_WRAP);
            String sealed = Base64.encodeToString(cipher.doFinal(value.getBytes(java.nio.charset.StandardCharsets.UTF_8)), Base64.NO_WRAP);
            if (!prefs.edit().putString("iv", iv).putString("sealed", sealed).commit()) throw new IllegalStateException("credential save failed");
            return "";
        }
        String sealed = prefs.getString("sealed", "");
        if (sealed.isEmpty()) return "";
        Cipher cipher = Cipher.getInstance("AES/GCM/NoPadding");
        cipher.init(Cipher.DECRYPT_MODE, key(), new GCMParameterSpec(128, Base64.decode(prefs.getString("iv", ""), Base64.NO_WRAP)));
        return new String(cipher.doFinal(Base64.decode(sealed, Base64.NO_WRAP)), java.nio.charset.StandardCharsets.UTF_8);
    }
}
