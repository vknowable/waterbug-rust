# Waterbug Rust -- Using Namada SDK from Kotlin
*TODO: add better instructions*  

### Usage
**Prerequisites:**
- Android NDK and Cmake
- OpenSSL for Android; see this handy repo for prebuilt: https://github.com/KDAB/android_openssl
- any [prerequisites](https://docs.namada.net/introduction/install/source/pre-requisites) you would normally need to build the Namada Sdk (e.g. protobuf)
- Cargo-ndk (follow the installation instructions here: https://github.com/bbqsrc/cargo-ndk)

The easiest way to install the Android NDK/Cmake is from within Android Studio. With any project open, select Tools -> Sdk Manager. Then, from the 'Sdk Tools' tab, check the boxes for 'NDK (side by side)' and 'CMake' and select OK to download and install.  

If you're not using Android Studio, you can install the NDK and Cmake from the command line using the `sdkmanager` tool. See here for instructions: https://developer.android.com/tools/sdkmanager

**Build:**
1. set the needed env variables to specify the locations of the NDK, Cmake, and OpenSSL. See `exports.env` for an example, adjusting directories as needed for your system. *Note: if you installed the NDK and Cmake through Android Studio, you should comment out the first three lines (`ANDROID_NDK_HOME`, `CMAKE`, `PATH`) as these will already have been automatically configured for you. Only set these manually if you've installed the Sdk via the command line tool.*
2. `cargo ndk -t armeabi-v7a -t arm64-v8a -o ./jniLibs build --release` to build the libraries; these will be output to the 'jniLibs' directory
3. `cargo run --bin uniffi-bindgen generate --library target/aarch64-linux-android/release/libwaterbugrs.so --language kotlin --out-dir out` to generate the Kotlin bindings; this will create the file 'out/uniffi/waterbugrs/waterbugrs.kt'

**Copy to your Android project:**
- copy the entire jniLibs dir to your project's `app/src/main/`
- copy the waterbugrs.kt file to a directory containing your project's Kotlin source code (eg: `app/src/main/java/com/example/{project name}`)

You will also need to add the following to `app/build.gradle.kts`:
```
android {
    namespace = "com.example.waterbug"
    compileSdk = 35

    ... // project build config here

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

dependencies {
    
    ... // other project dependencies here

    implementation("net.java.dev.jna:jna:5.9.0@aar")
}
```
