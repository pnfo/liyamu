package lk.pitaka.liyamu

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.graphics.Typeface
import android.os.Bundle
import android.provider.Settings
import android.text.InputType
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.inputmethod.InputMethodManager
import android.widget.Button
import android.widget.EditText
import android.widget.LinearLayout
import android.widget.RadioButton
import android.widget.RadioGroup
import android.widget.ScrollView
import android.widget.Switch
import android.widget.TextView
import android.widget.Toast

/** The app: how to turn the keyboard on, a place to try it, the settings, your shortcuts and help. */
class MainActivity : Activity() {
    private lateinit var prefs: Prefs
    private lateinit var sinhala: Typeface
    private lateinit var col: LinearLayout
    private val dp get() = resources.displayMetrics.density
    private var layoutListener: SharedPreferences.OnSharedPreferenceChangeListener? = null

    override fun onDestroy() {
        prefs.p.unregisterOnSharedPreferenceChangeListener(layoutListener)
        super.onDestroy()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        prefs = Prefs(getSharedPreferences(Prefs.NAME, Context.MODE_PRIVATE))
        sinhala = Typeface.createFromAsset(assets, "abhaya.ttf")
        col = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            val p = (18 * dp).toInt()
            setPadding(p, p, p, p * 2)
        }
        setContentView(ScrollView(this).apply { addView(col); fitsSystemWindows = true })
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager

        text("ලියමු", 34f, bold = true).typeface = sinhala
        text("Liyamu — a Sinhala keyboard", 16f)
        note("Private: this app has no internet permission, so nothing you type can leave your phone. Free and open source.")

        heading("Turn it on")
        button("1. Turn on Liyamu in the keyboard list") { startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)) }
        button("2. Choose Liyamu as the keyboard") { imm.showInputMethodPicker() }
        note("Android warns that any keyboard could collect what you type. Liyamu cannot send it anywhere.")

        heading("Try it here")
        col.addView(EditText(this).apply {
            hint = "mama gedara yanawa"
            typeface = sinhala
            textSize = 20f
            minLines = 2
            gravity = Gravity.TOP
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
        }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))

        heading("Layout")
        val group = RadioGroup(this)
        Prefs.LAYOUTS.forEachIndexed { i, name ->
            group.addView(RadioButton(this).apply { text = name; id = 100 + i; textSize = 16f })
        }
        group.check(100 + prefs.layout)
        group.setOnCheckedChangeListener { _, id -> if (prefs.layout != id - 100) prefs.layout = id - 100 }
        col.addView(group)
        // the 🌐 key changes the layout too
        layoutListener = SharedPreferences.OnSharedPreferenceChangeListener { _, key -> if (key == "layout") group.check(100 + prefs.layout) }
        prefs.p.registerOnSharedPreferenceChangeListener(layoutListener)
        note("The 🌐 key on the keyboard also changes the layout.")

        heading("Suggestions")
        switch("Word suggestions", "suggestions", true, "A row of words above the keys; tap one to take it. Space takes the first.")
        switch("Suggest Pali words", "paliWords", false, "Also offer words found only in Pali: ධම්මං, භික්ඛූනං.")

        heading("Letters")
        switch("Repaya", "repaya", false, "ධර්ම → ධර්‍ම, කර්ම → කර්‍ම")
        switch("Pali joined letters (බැඳි අකුරු)", "conjuncts", false, "බුද්ධ → බුද්‍ධ, තත්ථ → තත්‍ථ, නන්ද → නන්‍ද")
        switch("Pali touching letters", "touching", false, "ධම්ම → ධම‍්ම, සඤ්ඤා → සඤ‍්ඤා")
        switch("Wijesekara: kombuva before the letter", "traditional", true, "ෙ then ක for කෙ, as on paper. Off: ක then ෙ.")

        heading("Your shortcuts")
        note("One on a line: the keys you type, =, what they stand for. Typing exactly those keys offers the words first.")
        val shortcuts = EditText(this).apply {
            setText(prefs.shortcuts)
            hint = "bs=බුදු සරණයි"
            typeface = sinhala
            textSize = 17f
            minLines = 4
            gravity = Gravity.TOP
            inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
        }
        col.addView(shortcuts, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        button("Save shortcuts") {
            prefs.p.edit().putString("shortcuts", shortcuts.text.toString()).apply()
            Toast.makeText(this, "Saved", Toast.LENGTH_SHORT).show()
        }

        heading("Help")
        note(HELP).textSize = 15f
    }

    private fun text(s: String, size: Float, bold: Boolean = false): TextView {
        val t = TextView(this).apply {
            text = s
            textSize = size
            if (bold) setTypeface(typeface, Typeface.BOLD)
        }
        col.addView(t)
        return t
    }

    private fun heading(s: String) {
        text(s, 19f, bold = true).setPadding(0, (26 * dp).toInt(), 0, (6 * dp).toInt())
    }

    private fun note(s: String): TextView = text(s, 14f).apply {
        alpha = 0.75f
        setPadding(0, (2 * dp).toInt(), 0, (8 * dp).toInt())
    }

    private fun button(s: String, action: (View) -> Unit) {
        col.addView(Button(this).apply {
            text = s
            isAllCaps = false
            setOnClickListener(action)
        }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
    }

    private fun switch(title: String, key: String, default: Boolean, example: String) {
        col.addView(Switch(this).apply {
            text = title
            textSize = 16f
            isChecked = prefs.p.getBoolean(key, default)
            setOnCheckedChangeListener { _, on -> prefs.p.edit().putBoolean(key, on).apply() }
        }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        note(example)
    }

    companion object {
        const val HELP = """Easy Singlish — type the way you would in a chat and pick the word.
  mama gedara yanawa → මම ගෙදර යනවා
  Vowels may be left out: hri ennm → හරි එන්නම්, gdr → ගෙදර
  Small slips are corrected: gedar, gedra → ගෙදර
  Places and names are known: homagama → හෝමාගම, col → කොළඹ
  A capital leans to the heavy letter: T ඨ, D ඩ, N ණ, L ළ, S ෂ

Wijesekara — the standard Sri Lankan layout. ⇧ shows the second letters. Hold a key with a small letter in its corner to type that letter (ඳ ඟ ඬ …).

Helakuru keys — exact phonetic typing: a අ, aa ආ, A ඇ, th ත, t ට, dh ද, d ඩ, sh ශ, x ං

English — plain letters, so you do not have to change keyboards.

Suggestions: tap a word to take it. Space takes the highlighted one.

Passwords, e-mail addresses and web addresses are always typed as plain letters and are never looked at.

arutha.lk"""
    }
}
