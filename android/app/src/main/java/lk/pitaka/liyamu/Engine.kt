package lk.pitaka.liyamu

import android.content.SharedPreferences

/** What the engine answers to a key. */
class Response(a: Array<String>) {
    val handled = a.getOrNull(0) == "1"
    val commit = a.getOrElse(1) { "" }
    val preedit = a.getOrElse(2) { "" }
    val selected = a.getOrNull(3)?.toIntOrNull() ?: 0
    val candidates: List<String> = a.drop(4)
}

/** The typing engine (Rust, through liyamu_jni.c). One session per text field. */
object Engine {
    const val CHAR = 0
    const val SPACE = 1
    const val BACKSPACE = 2
    const val ENTER = 3

    init {
        System.loadLibrary("liyamu_jni")
    }

    @JvmStatic external fun create(layout: Int, traditional: Boolean, pali: Boolean, suggest: Boolean, repaya: Boolean, conjuncts: Boolean, paliWords: Boolean): Long
    @JvmStatic external fun destroy(session: Long)
    @JvmStatic external fun setOptions(session: Long, layout: Int, traditional: Boolean, pali: Boolean, suggest: Boolean, repaya: Boolean, conjuncts: Boolean, paliWords: Boolean): Array<String>
    @JvmStatic external fun key(session: Long, kind: Int, ch: Int, altgr: Boolean): Array<String>
    @JvmStatic external fun pick(session: Long, index: Int): Array<String>
    @JvmStatic external fun commit(session: Long): Array<String>
    @JvmStatic external fun reset(session: Long)
    @JvmStatic external fun setShortcuts(utf8: ByteArray)
    @JvmStatic external fun warmUp()
}

/** The settings, kept in the app's private preferences. */
class Prefs(val p: SharedPreferences) {
    /** 0 Wijesekara, 1 Easy Singlish, 2 Helakuru keys, 3 English (plain letters) */
    var layout: Int
        get() = p.getInt("layout", 1)
        set(v) = p.edit().putInt("layout", v).apply()
    val suggestions get() = p.getBoolean("suggestions", true)
    val paliWords get() = p.getBoolean("paliWords", false)
    val repaya get() = p.getBoolean("repaya", false)
    val conjuncts get() = p.getBoolean("conjuncts", false)
    val touching get() = p.getBoolean("touching", false)
    val traditional get() = p.getBoolean("traditional", true)
    val shortcuts get() = p.getString("shortcuts", "") ?: ""

    fun create() = Engine.create(layout.coerceAtMost(2), traditional, touching, suggestions, repaya, conjuncts, paliWords)
    fun apply(session: Long) = Engine.setOptions(session, layout.coerceAtMost(2), traditional, touching, suggestions, repaya, conjuncts, paliWords)

    companion object {
        const val NAME = "liyamu"
        val LAYOUTS = listOf("Wijesekara", "Easy Singlish", "Helakuru keys", "English")
    }
}
