import org.gradle.api.tasks.Exec

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
    id("org.jetbrains.kotlin.plugin.serialization")
}

android {
    namespace = "page.stephens.cascade"
    compileSdk = 35

    defaultConfig {
        applicationId = "page.stephens.cascade"
        minSdk = 26
        targetSdk = 35
        versionCode = 6
        versionName = "0.2.4"

        ndk {
            // Keep parity with cargo-ndk targets in the build script below.
            abiFilters += listOf("arm64-v8a", "armeabi-v7a", "x86_64")
        }
    }

    signingConfigs {
        // Release signing from the environment (set in CI from repo secrets).
        // Falls back to debug signing locally when the keystore env isn't present,
        // so a local `assembleRelease` still works without the production key.
        val ksFile = System.getenv("ANDROID_KEYSTORE_FILE")
        if (ksFile != null && file(ksFile).exists()) {
            create("release") {
                storeFile = file(ksFile)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD")
                keyAlias = System.getenv("ANDROID_KEY_ALIAS")
                keyPassword = System.getenv("ANDROID_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            // Production key in CI; debug key locally (and as a safety fallback).
            signingConfig = signingConfigs.findByName("release") ?: signingConfigs.getByName("debug")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions { jvmTarget = "17" }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    packaging {
        resources.excludes += "/META-INF/{AL2.0,LGPL2.1}"
    }

    sourceSets["main"].apply {
        // The Rust .so files are dropped here by `cargoBuildAndroid` (below).
        jniLibs.srcDir(layout.buildDirectory.dir("rust-jniLibs"))
        // The generated Kotlin bindings land here.
        java.srcDir(layout.buildDirectory.dir("generated/source/uniffi/kotlin"))
        // Reuse the web app's public/sounds dir as our assets dir instead of
        // committing a duplicate copy of waterfall.ogg (~8.6 MB).
        assets.srcDir(rootProject.projectDir.parentFile.resolve("web/public"))
    }
}

dependencies {
    val composeBom = platform("androidx.compose:compose-bom:2024.10.01")
    implementation(composeBom)
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.core:core-splashscreen:1.0.1")
    implementation("androidx.datastore:datastore-preferences:1.1.1")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.7.3")

    // Media3 for the background playback service.
    val media3 = "1.5.1"
    implementation("androidx.media3:media3-exoplayer:$media3")
    implementation("androidx.media3:media3-session:$media3")
    implementation("androidx.media3:media3-common:$media3")

    // UniFFI bindings rely on JNA at runtime.
    implementation("net.java.dev.jna:jna:5.15.0@aar")

    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

// ---------- Rust core integration ----------
//
// `cargo ndk` cross-compiles `cascade-uniffi` for every Android ABI and drops
// the resulting `lib*.so` files into the directory wired into `jniLibs` above.
// Then `uniffi-bindgen` reads one of those `.so` files to generate Kotlin
// bindings into the source set wired into `java.srcDir` above.
//
// Both tasks are wired into `preBuild` so a clean `./gradlew assembleDebug`
// builds the entire chain from `cargo` → bindings → APK.

val rustRoot = rootProject.projectDir.parentFile.parentFile  // → repo root
val ndkAbis = listOf("arm64-v8a", "armeabi-v7a", "x86_64")
val rustTargets = mapOf(
    "arm64-v8a" to "aarch64-linux-android",
    "armeabi-v7a" to "armv7-linux-androideabi",
    "x86_64" to "x86_64-linux-android",
)

val rustJniLibsDir = layout.buildDirectory.dir("rust-jniLibs")
val uniffiKotlinOut = layout.buildDirectory.dir("generated/source/uniffi/kotlin")

val cargoBuildAndroid by tasks.registering(Exec::class) {
    description = "Cross-compile cascade-uniffi for every Android ABI via cargo-ndk."
    group = "rust"

    workingDir = rustRoot
    val abiArgs = ndkAbis.flatMap { listOf("-t", it) }
    commandLine(listOf("cargo", "ndk") + abiArgs + listOf(
        "-o", rustJniLibsDir.get().asFile.absolutePath,
        "build", "--release", "-p", "cascade-uniffi",
    ))

    environment("ANDROID_NDK_HOME", System.getenv("ANDROID_NDK_HOME")
        ?: "${System.getProperty("user.home")}/Android/Sdk/ndk/26.3.11579264")

    inputs.dir(File(rustRoot, "crates/cascade-uniffi/src"))
    inputs.dir(File(rustRoot, "crates/cascade-core/src"))
    inputs.file(File(rustRoot, "crates/cascade-uniffi/Cargo.toml"))
    inputs.file(File(rustRoot, "crates/cascade-core/Cargo.toml"))
    inputs.file(File(rustRoot, "Cargo.lock"))
    outputs.dir(rustJniLibsDir)
}

val uniffiGenerateBindings by tasks.registering(Exec::class) {
    description = "Generate Kotlin bindings from the Rust .so file."
    group = "rust"
    dependsOn(cargoBuildAndroid)

    workingDir = rustRoot
    // Use the arm64 .so as the bindings source — same metadata is in every ABI.
    val soPath = rustJniLibsDir.get().file("arm64-v8a/libcascade_uniffi.so").asFile

    commandLine(
        "cargo", "run", "-p", "cascade-uniffi", "--bin", "uniffi-bindgen", "--release", "--",
        "generate",
        "--library", soPath.absolutePath,
        "--language", "kotlin",
        "--out-dir", uniffiKotlinOut.get().asFile.absolutePath,
    )

    inputs.file(soPath)
    outputs.dir(uniffiKotlinOut)
}

tasks.named("preBuild") { dependsOn(uniffiGenerateBindings) }
