# Default proguard rules — kept permissive while the app is small.
# JNA + UniFFI both rely on reflection-y JNI lookups; keep their classes.
-keep class com.sun.jna.** { *; }
-keep class uniffi.** { *; }

# Media3 already ships with its own consumer proguard rules.
