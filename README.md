source exports.env
cargo ndk -t armeabi-v7a -t arm64-v8a -o ./jniLibs build --release
cargo run --bin uniffi-bindgen generate --library target/aarch64-linux-android/release/libwaterbugrs.so --language kotlin --out-dir out

copy jniLibs folder to app/src/main/
copy the .kt file to the same folder as your project's other .kt files

needed in build.gradle.kts
    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
    implementation("net.java.dev.jna:jna:5.9.0@aar")