# Waterbug Rust -- Using Namada SDK from Kotlin
*TODO: add better instructions*  

### Usage
**Prerequisites:**
- Android NDK
- OpenSSL for Android; see this handy repo for prebuilt: https://github.com/KDAB/android_openssl

**Build:**
1. `source exports.env` to set the proper env variables
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
