package lk.arutha.liyamu

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
    @JvmStatic external fun setPredict(session: Long, on: Boolean)
    @JvmStatic external fun key(session: Long, kind: Int, ch: Int, altgr: Boolean): Array<String>
    @JvmStatic external fun pick(session: Long, index: Int): Array<String>
    @JvmStatic external fun commit(session: Long): Array<String>
    @JvmStatic external fun reset(session: Long)
    @JvmStatic external fun setShortcuts(utf8: ByteArray)
    @JvmStatic external fun setLearning(pathUtf8: ByteArray, on: Boolean)
    @JvmStatic external fun forgetLearned()
    @JvmStatic external fun warmUp()
}

/** The settings, kept in the app's private preferences. */
class Prefs(val p: SharedPreferences) {
    init {
        // English was a fourth layout until 0.4; it is now the other language of the globe key
        if (p.getInt("layout", 1) > 2) p.edit().putInt("layout", 1).putBoolean("english", true).apply()
    }

    /** The Sinhala layout, chosen in the settings: 0 Wijesekara, 1 Easy Singlish, 2 Helakuru keys */
    var layout: Int
        get() = p.getInt("layout", 1).coerceIn(0, 2)
        set(v) = p.edit().putInt("layout", v).apply()
    /** The globe key: plain English letters instead of Sinhala */
    var english: Boolean
        get() = p.getBoolean("english", false)
        set(v) = p.edit().putBoolean("english", v).apply()
    val suggestions get() = p.getBoolean("suggestions", true)
    val paliWords get() = p.getBoolean("paliWords", false)
    val repaya get() = p.getBoolean("repaya", false)
    val conjuncts get() = p.getBoolean("conjuncts", false)
    val touching get() = p.getBoolean("touching", false)
    val traditional get() = p.getBoolean("traditional", true)
    val shortcuts get() = p.getString("shortcuts", "") ?: ""
    val learn get() = p.getBoolean("learn", true)

    /** Tells the engine whether to learn from the words picked, and the private file that keeps them. */
    fun applyLearning(context: android.content.Context, allowed: Boolean = true) =
        Engine.setLearning(java.io.File(context.filesDir, "learned.tsv").path.toByteArray(), learn && allowed)

    fun create() = Engine.create(layout, traditional, touching, suggestions, repaya, conjuncts, paliWords).also { Engine.setPredict(it, true) }
    fun apply(session: Long) = Engine.setOptions(session, layout, traditional, touching, suggestions, repaya, conjuncts, paliWords)

    companion object {
        const val NAME = "liyamu"
        val LAYOUTS = listOf("Wijesekara", "Easy Singlish", "Helakuru keys")
    }
}
