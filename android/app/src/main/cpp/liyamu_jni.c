/* JNI bridge between lk.arutha.liyamu.Engine and the C API of the engine (liyamu.h).
 * A response comes back as String[]: handled ("1"/"0"), commit, preedit, selected, then the candidates. */
#include <jni.h>
#include <stdio.h>
#include <stdlib.h>
#include "liyamu.h"

#define FN(name) Java_lk_arutha_liyamu_Engine_##name

static LiyamuOptions options(jint layout, jboolean order, jboolean pali, jboolean suggest, jboolean repaya,
                             jboolean conjuncts, jboolean pali_words) {
    LiyamuOptions o = {(uint32_t)layout, 0, order ? 0 : 1, pali, suggest, repaya, conjuncts, pali_words};
    return o;
}

static jobjectArray response(JNIEnv *env, LiyamuResponse *r) {
    jclass string = (*env)->FindClass(env, "java/lang/String");
    if (!r) return (*env)->NewObjectArray(env, 0, string, NULL);
    jobjectArray a = (*env)->NewObjectArray(env, 4 + (jsize)r->candidate_count, string, NULL);
    char n[16];
    snprintf(n, sizeof n, "%u", r->selected);
    const char *head[4] = {r->handled ? "1" : "0", r->commit, r->preedit, n};
    for (int i = 0; i < 4; i++) {
        jstring s = (*env)->NewStringUTF(env, head[i]);
        (*env)->SetObjectArrayElement(env, a, i, s);
        (*env)->DeleteLocalRef(env, s);
    }
    for (uint32_t i = 0; i < r->candidate_count; i++) {
        jstring s = (*env)->NewStringUTF(env, r->candidates[i]);
        (*env)->SetObjectArrayElement(env, a, 4 + (jsize)i, s);
        (*env)->DeleteLocalRef(env, s);
    }
    liyamu_response_free(r);
    return a;
}

JNIEXPORT jlong JNICALL FN(create)(JNIEnv *env, jclass c, jint layout, jboolean order, jboolean pali, jboolean suggest,
                                   jboolean repaya, jboolean conjuncts, jboolean pali_words) {
    LiyamuOptions o = options(layout, order, pali, suggest, repaya, conjuncts, pali_words);
    return (jlong)(intptr_t)liyamu_session_new(&o);
}

JNIEXPORT void JNICALL FN(destroy)(JNIEnv *env, jclass c, jlong s) {
    liyamu_session_free((LiyamuSession *)(intptr_t)s);
}

JNIEXPORT jobjectArray JNICALL FN(setOptions)(JNIEnv *env, jclass c, jlong s, jint layout, jboolean order, jboolean pali,
                                              jboolean suggest, jboolean repaya, jboolean conjuncts, jboolean pali_words) {
    LiyamuOptions o = options(layout, order, pali, suggest, repaya, conjuncts, pali_words);
    return response(env, liyamu_session_set_options((LiyamuSession *)(intptr_t)s, &o));
}

JNIEXPORT jobjectArray JNICALL FN(key)(JNIEnv *env, jclass c, jlong s, jint kind, jint ch, jboolean altgr) {
    return response(env, liyamu_session_key((LiyamuSession *)(intptr_t)s, (uint32_t)kind, (uint32_t)ch, altgr, false));
}

JNIEXPORT jobjectArray JNICALL FN(pick)(JNIEnv *env, jclass c, jlong s, jint index) {
    return response(env, liyamu_session_pick((LiyamuSession *)(intptr_t)s, (uint32_t)index));
}

JNIEXPORT jobjectArray JNICALL FN(commit)(JNIEnv *env, jclass c, jlong s) {
    return response(env, liyamu_session_commit((LiyamuSession *)(intptr_t)s));
}

JNIEXPORT void JNICALL FN(reset)(JNIEnv *env, jclass c, jlong s) {
    liyamu_session_reset((LiyamuSession *)(intptr_t)s);
}

JNIEXPORT void JNICALL FN(setShortcuts)(JNIEnv *env, jclass c, jbyteArray utf8) {
    /* real UTF-8 from Kotlin (not JNI's modified UTF-8), NUL-terminated here */
    jsize n = (*env)->GetArrayLength(env, utf8);
    char *text = malloc((size_t)n + 1);
    (*env)->GetByteArrayRegion(env, utf8, 0, n, (jbyte *)text);
    text[n] = 0;
    liyamu_set_shortcuts(text);
    free(text);
}

JNIEXPORT void JNICALL FN(setLearning)(JNIEnv *env, jclass c, jbyteArray utf8, jboolean on) {
    jsize n = (*env)->GetArrayLength(env, utf8);
    char *path = malloc((size_t)n + 1);
    (*env)->GetByteArrayRegion(env, utf8, 0, n, (jbyte *)path);
    path[n] = 0;
    liyamu_set_learning(path, on);
    free(path);
}

JNIEXPORT void JNICALL FN(forgetLearned)(JNIEnv *env, jclass c) {
    liyamu_forget_learned();
}

JNIEXPORT void JNICALL FN(warmUp)(JNIEnv *env, jclass c) {
    liyamu_warm_up();
}
