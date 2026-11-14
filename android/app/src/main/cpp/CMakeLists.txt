cmake_minimum_required(VERSION 3.22.1)
project(liyamu_jni C)

if(ANDROID_ABI STREQUAL "arm64-v8a")
    set(TRIPLE aarch64-linux-android)
elseif(ANDROID_ABI STREQUAL "armeabi-v7a")
    set(TRIPLE armv7-linux-androideabi)
elseif(ANDROID_ABI STREQUAL "x86_64")
    set(TRIPLE x86_64-linux-android)
endif()
set(REPO ${CMAKE_SOURCE_DIR}/../../../../..)

add_library(liyamu STATIC IMPORTED)
set_target_properties(liyamu PROPERTIES IMPORTED_LOCATION ${REPO}/target/${TRIPLE}/release/libliyamu.a)

add_library(liyamu_jni SHARED liyamu_jni.c)
target_include_directories(liyamu_jni PRIVATE ${REPO}/core/liyamu-ffi/include)
target_link_libraries(liyamu_jni liyamu log dl m)
target_link_options(liyamu_jni PRIVATE -Wl,--gc-sections -Wl,-z,max-page-size=16384)
