package lk.arutha.liyamu

import android.app.Activity
import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.content.res.ColorStateList
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.view.WindowInsets
import android.widget.ImageView
import android.graphics.Typeface
import android.os.Bundle
import android.provider.Settings
import android.text.InputType
import android.text.SpannableString
import android.text.Spanned
import android.text.TextPaint
import android.text.style.MetricAffectingSpan
import android.text.style.RelativeSizeSpan
import android.view.Gravity
import android.view.View
import android.view.ViewGroup.LayoutParams.MATCH_PARENT
import android.view.ViewGroup.LayoutParams.WRAP_CONTENT
import android.view.inputmethod.InputMethodManager
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
    private lateinit var page: LinearLayout
    /** the card being filled */
    private lateinit var col: LinearLayout
    private val dp get() = resources.displayMetrics.density
    private fun px(v: Int) = (v * dp).toInt()
    private fun colour(id: Int) = getColor(id)
    private var layoutListener: SharedPreferences.OnSharedPreferenceChangeListener? = null
    private var layoutListener2: SharedPreferences.OnSharedPreferenceChangeListener? = null

    override fun onDestroy() {
        prefs.p.unregisterOnSharedPreferenceChangeListener(layoutListener)
        prefs.p.unregisterOnSharedPreferenceChangeListener(layoutListener2)
        super.onDestroy()
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        prefs = Prefs(getSharedPreferences(Prefs.NAME, Context.MODE_PRIVATE))
        sinhala = Typeface.createFromAsset(assets, "abhaya.ttf")
        page = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            setPadding(px(16), px(12), px(16), px(28))
        }
        val scroll = ScrollView(this).apply { addView(page); isVerticalScrollBarEnabled = false }
        // the page runs under the status bar and above the keyboard: keep its content clear of both
        scroll.setOnApplyWindowInsetsListener { v, insets ->
            val bars = if (Build.VERSION.SDK_INT >= 30) insets.getInsets(WindowInsets.Type.systemBars() or WindowInsets.Type.ime())
            else @Suppress("DEPRECATION") android.graphics.Insets.of(0, insets.systemWindowInsetTop, 0, insets.systemWindowInsetBottom)
            v.setPadding(0, bars.top, 0, bars.bottom)
            insets
        }
        setContentView(scroll)
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager

        header()

        card("Turn it on")
        button("1  Turn on Liyamu in the keyboard list") { startActivity(Intent(Settings.ACTION_INPUT_METHOD_SETTINGS)) }
        button("2  Choose Liyamu as the keyboard") { imm.showInputMethodPicker() }
        note("Android warns that any keyboard could collect what you type. Liyamu has no internet permission, so it cannot send anything anywhere.")

        card("Try it here")
        col.addView(field("mama gedara yanawa", 20f, 2), LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))

        card("Layout")
        val group = RadioGroup(this)
        Prefs.LAYOUTS.forEachIndexed { i, name ->
            group.addView(RadioButton(this).apply {
                text = name
                id = 100 + i
                textSize = 16f
                setTextColor(colour(R.color.ink))
                buttonTintList = ColorStateList.valueOf(colour(R.color.accent))
                setPadding(px(6), px(7), 0, px(7))
            })
        }
        group.check(100 + prefs.layout)
        group.setOnCheckedChangeListener { _, id -> if (prefs.layout != id - 100) prefs.layout = id - 100 }
        col.addView(group)
        // the 🌐 key changes the layout too
        layoutListener = SharedPreferences.OnSharedPreferenceChangeListener { _, key -> if (key == "layout") group.check(100 + prefs.layout) }
        prefs.p.registerOnSharedPreferenceChangeListener(layoutListener)
        note("The globe key on the keyboard also changes the layout.")

        card("Suggestions")
        switch("Word suggestions", "suggestions", true, "A row of words above the keys; tap one to take it. Space takes the first.")
        switch("Learn from the words I pick", "learn", true, "A word you pick twice for the same letters is offered first from then on. Kept only on this phone.")
        prefs.applyLearning(this)
        layoutListener2 = SharedPreferences.OnSharedPreferenceChangeListener { _, key -> if (key == "learn") prefs.applyLearning(this) }
        prefs.p.registerOnSharedPreferenceChangeListener(layoutListener2)
        link("Forget what was learned") {
            Engine.forgetLearned()
            Toast.makeText(this, "Forgotten", Toast.LENGTH_SHORT).show()
        }
        switch("Suggest Pali words", "paliWords", false, "Also offer words found only in Pali: ධම්මං, භික්ඛූනං.")

        card("Letters")
        switch("Repaya", "repaya", false, "ධර්ම → ධර්‍ම,  කර්ම → කර්‍ම")
        switch("Pali joined letters (බැඳි අකුරු)", "conjuncts", false, "බුද්ධ → බුද්‍ධ,  තත්ථ → තත්‍ථ,  නන්ද → නන්‍ද")
        switch("Pali touching letters", "touching", false, "ධම්ම → ධම‍්ම,  සඤ්ඤා → සඤ‍්ඤා")
        switch("Wijesekara: kombuva before the letter", "traditional", true, "ෙ then ක for කෙ, as on paper. Off: ක then ෙ.")

        card("Your shortcuts")
        note("One on a line: the keys you type, =, what they stand for. Typing exactly those keys offers the words first.")
        val shortcuts = field("bs=බුදු සරණයි", 17f, 4).apply { setText(prefs.shortcuts) }
        col.addView(shortcuts, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT))
        button("Save shortcuts") {
            prefs.p.edit().putString("shortcuts", shortcuts.text.toString()).apply()
            Toast.makeText(this, "Saved", Toast.LENGTH_SHORT).show()
        }

        card("Help")
        note(HELP).apply { textSize = 15f; alpha = 1f; setTextColor(colour(R.color.ink)); setLineSpacing(0f, 1.25f) }
    }

    // ---- the pieces

    /** Sinhala is always set in Abhaya, a little larger so that it sits level with the Latin beside it. */
    private fun si(s: String): CharSequence {
        val out = SpannableString(s)
        var i = 0
        while (i < s.length) {
            if (s[i] !in '\u0D80'..'\u0DFF') { i++; continue }
            var j = i
            while (j < s.length && (s[j] in '\u0D80'..'\u0DFF' || s[j] == '\u200D' || s[j] == '\u200C')) j++
            out.setSpan(object : MetricAffectingSpan() {
                override fun updateMeasureState(p: TextPaint) { p.typeface = sinhala }
                override fun updateDrawState(p: TextPaint) { p.typeface = sinhala }
            }, i, j, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
            out.setSpan(RelativeSizeSpan(1.12f), i, j, Spanned.SPAN_EXCLUSIVE_EXCLUSIVE)
            i = j
        }
        return out
    }

    private fun rounded(fill: Int, radius: Int, stroke: Int = 0) = GradientDrawable().apply {
        setColor(fill)
        cornerRadius = radius * dp
        if (stroke != 0) setStroke(px(1), stroke)
    }

    /** The icon, the name and what the keyboard promises. */
    private fun header() {
        val row = LinearLayout(this).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(px(4), px(14), 0, px(6))
        }
        row.addView(ImageView(this).apply { setImageResource(R.drawable.logo) }, LinearLayout.LayoutParams(px(68), px(68)))
        val names = LinearLayout(this).apply { orientation = LinearLayout.VERTICAL; setPadding(px(14), 0, 0, 0) }
        names.addView(TextView(this).apply {
            text = "ලියමු"
            typeface = sinhala
            textSize = 38f
            setTextColor(colour(R.color.accent))
            includeFontPadding = false
        })
        names.addView(TextView(this).apply {
            text = "The private Sinhala keyboard"
            textSize = 15f
            setTextColor(colour(R.color.muted))
        })
        row.addView(names)
        page.addView(row)
        val chips = LinearLayout(this).apply { setPadding(0, px(8), 0, px(2)) }
        for (c in listOf("No internet access", "No ads", "Open source")) {
            chips.addView(TextView(this).apply {
                text = c
                textSize = 12.5f
                setTextColor(colour(R.color.accent))
                background = rounded(0, 20, colour(R.color.accent))
                setPadding(px(11), px(5), px(11), px(5))
            }, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT).apply { marginEnd = px(7) })
        }
        page.addView(chips)
    }

    /** Starts a card; what is added next goes into it. */
    private fun card(title: String) {
        col = LinearLayout(this).apply {
            orientation = LinearLayout.VERTICAL
            background = rounded(colour(R.color.card), 18, colour(R.color.line))
            setPadding(px(16), px(14), px(16), px(14))
        }
        page.addView(col, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = px(14) })
        col.addView(TextView(this).apply {
            text = title.uppercase()
            textSize = 12.5f
            letterSpacing = 0.09f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(colour(R.color.accent))
            setPadding(0, 0, 0, px(8))
        })
    }

    private fun note(s: String): TextView {
        val t = TextView(this).apply {
            text = si(s)
            textSize = 13.5f
            setTextColor(colour(R.color.muted))
            setPadding(0, px(2), 0, px(6))
        }
        col.addView(t)
        return t
    }

    private fun field(hintText: String, size: Float, lines: Int) = EditText(this).apply {
        hint = hintText
        typeface = sinhala
        textSize = size
        minLines = lines
        gravity = Gravity.TOP
        setTextColor(colour(R.color.ink))
        setHintTextColor(colour(R.color.muted) and 0x00FFFFFF or 0x77000000)
        background = rounded(colour(R.color.page), 12, colour(R.color.line))
        setPadding(px(12), px(10), px(12), px(10))
        inputType = InputType.TYPE_CLASS_TEXT or InputType.TYPE_TEXT_FLAG_MULTI_LINE or InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS
    }

    private fun button(s: String, action: (View) -> Unit) {
        col.addView(TextView(this).apply {
            text = s
            textSize = 15.5f
            setTypeface(typeface, Typeface.BOLD)
            setTextColor(colour(R.color.on_accent))
            background = rounded(colour(R.color.accent), 14)
            setPadding(px(16), px(13), px(16), px(13))
            isClickable = true
            setOnClickListener(action)
        }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = px(4); bottomMargin = px(6) })
    }

    /** A quiet action inside a card: outlined, not filled. */
    private fun link(s: String, action: (View) -> Unit) {
        col.addView(TextView(this).apply {
            text = s
            textSize = 14f
            setTextColor(colour(R.color.accent))
            background = rounded(0, 12, colour(R.color.accent))
            setPadding(px(14), px(8), px(14), px(8))
            isClickable = true
            setOnClickListener(action)
        }, LinearLayout.LayoutParams(WRAP_CONTENT, WRAP_CONTENT).apply { topMargin = px(2); bottomMargin = px(10) })
    }

    private fun switch(title: String, key: String, default: Boolean, example: String) {
        val states = arrayOf(intArrayOf(android.R.attr.state_checked), intArrayOf())
        col.addView(Switch(this).apply {
            text = si(title)
            textSize = 16f
            setTextColor(colour(R.color.ink))
            thumbTintList = ColorStateList(states, intArrayOf(colour(R.color.accent), colour(R.color.muted)))
            trackTintList = ColorStateList(states, intArrayOf(colour(R.color.accent) and 0x00FFFFFF or 0x66000000, colour(R.color.line)))
            isChecked = prefs.p.getBoolean(key, default)
            setOnCheckedChangeListener { _, on -> prefs.p.edit().putBoolean(key, on).apply() }
        }, LinearLayout.LayoutParams(MATCH_PARENT, WRAP_CONTENT).apply { topMargin = px(4) })
        if (example.any { it in '\u0D80'..'\u0DFF' }) note(example).apply { textSize = 15.5f; setTextColor(colour(R.color.ink)) } else note(example)
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

Suggestions: tap a word to take it. Space takes the highlighted one. A word you pick twice for the same letters comes first from then on; this is kept only on your phone, and Settings can forget it or turn it off.

Voice typing: the microphone opens your phone's own voice typing (Google voice typing), and you come back here when you finish. Choose Sinhala in its languages. The sound goes to that service, not to Liyamu, which cannot use the microphone or the internet.

Passwords, e-mail addresses and web addresses are always typed as plain letters and are never looked at.

arutha.lk"""
    }
}
