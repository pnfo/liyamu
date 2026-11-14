package lk.pitaka.liyamu

import android.content.Context
import android.content.res.Configuration
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.RectF
import android.graphics.Typeface
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.view.WindowInsets

/** A key: what it shows and what it does. */
class Key(
    val label: String,
    /** the character sent to the engine (US-QWERTY key), or 0 for a special key */
    val ch: Char = 0.toChar(),
    val action: Int = TYPE,
    val weight: Float = 1f,
    /** small label in the corner: what a long press types (the layout's AltGr letter) */
    val alt: String = "",
) {
    val rect = RectF()

    companion object {
        const val TYPE = 0      // ch goes to the engine
        const val TEXT = 1      // label is inserted as it is (digits, punctuation)
        const val SHIFT = 2
        const val BACKSPACE = 3
        const val SYMBOLS = 4
        const val LAYOUT = 5
        const val SPACE = 6
        const val ENTER = 7
    }
}

interface KeyListener {
    fun onKey(key: Key, shifted: Boolean, long: Boolean)
    fun onPick(index: Int)
}

/** The keyboard: a row of suggestions, then the keys. Drawn by hand, one view. */
class KeyboardView(context: Context) : View(context) {
    var listener: KeyListener? = null
    private var rows: List<List<Key>> = emptyList()
    private var layout = 1
    private var symbols = false
    private var shift = false
    private var candidates: List<String> = emptyList()
    private var selected = 0
    private val candidateRects = ArrayList<RectF>()
    private var pressed: Key? = null
    private var longDone = false
    private val handler = Handler(Looper.getMainLooper())

    private val dp = resources.displayMetrics.density
    private val dark = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
    private val sinhala: Typeface = Typeface.createFromAsset(context.assets, "abhaya.ttf")
    private val bg = if (dark) 0xFF10262C.toInt() else 0xFFE6ECEC.toInt()
    private val keyBg = if (dark) 0xFF24424A.toInt() else 0xFFFFFFFF.toInt()
    private val specialBg = if (dark) 0xFF183238.toInt() else 0xFFC9D6D8.toInt()
    private val pressedBg = 0xFFF2A541.toInt()
    private val accent = 0xFF0F4C5C.toInt()
    private val ink = if (dark) 0xFFF6F1E4.toInt() else 0xFF14282D.toInt()
    private val faint = if (dark) 0xFF9DB4B8.toInt() else 0xFF5C7378.toInt()
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val text = Paint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }

    private val stripH = 46 * dp
    private val rowH = 52 * dp

    init {
        isHapticFeedbackEnabled = true
        setLayout(1)
    }

    fun setLayout(l: Int) {
        layout = l
        symbols = false
        shift = false
        build()
    }

    fun setCandidates(c: List<String>, sel: Int) {
        candidates = c
        selected = sel
        invalidate()
    }

    // ---- the layouts

    private fun letters(s: String) = s.map { Key(it.toString(), it) }

    private fun build() {
        val bottom = { name: String ->
            listOf(
                Key(if (symbols) "අආ" else "?123", action = Key.SYMBOLS, weight = 1.5f),
                Key("🌐", action = Key.LAYOUT, weight = 1.2f),
                Key(",", action = Key.TEXT),
                Key(name, action = Key.SPACE, weight = 4.3f),
                Key(".", action = Key.TEXT),
                Key("⏎", action = Key.ENTER, weight = 1.5f),
            )
        }
        val name = Prefs.LAYOUTS[layout]
        rows = when {
            symbols -> listOf(
                "1234567890".map { Key(it.toString(), action = Key.TEXT) },
                "@#%&-+()/?".map { Key(it.toString(), action = Key.TEXT) },
                listOf(Key("!", action = Key.TEXT), Key("\"", action = Key.TEXT), Key("'", action = Key.TEXT), Key(":", action = Key.TEXT),
                    Key(";", action = Key.TEXT), Key("*", action = Key.TEXT), Key("=", action = Key.TEXT), Key("෴", action = Key.TEXT),
                    Key("⌫", action = Key.BACKSPACE, weight = 1.5f)),
                bottom(name),
            )
            layout == 0 -> listOf(
                wije("qwertyuiop["),
                wije("asdfghjkl;`"),
                listOf(Key("⇧", action = Key.SHIFT, weight = 1.4f)) + wije("xcvbnm,.\\") + Key("⌫", action = Key.BACKSPACE, weight = 1.4f),
                bottom(name),
            )
            else -> listOf(
                letters(if (shift) "QWERTYUIOP" else "qwertyuiop"),
                letters(if (shift) "ASDFGHJKL" else "asdfghjkl"),
                listOf(Key("⇧", action = Key.SHIFT, weight = 1.5f)) + letters(if (shift) "ZXCVBNM" else "zxcvbnm") +
                    Key("⌫", action = Key.BACKSPACE, weight = 1.5f),
                bottom(name),
            )
        }
        requestLayout()
        invalidate()
    }

    /** Wijesekara keys: the label is what the key types, the character is its US-QWERTY key. */
    private fun wije(keys: String) = keys.map { k ->
        val c = if (shift) WIJE_SHIFT[k] ?: k.uppercaseChar() else k
        Key(WIJE[c] ?: c.toString(), c, alt = WIJE_ALT[c] ?: "")
    }

    // ---- measuring and drawing

    /** room under the keys for the system's gesture bar and keyboard-switch button */
    private var bottomInset = 0

    override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
        val b = if (Build.VERSION.SDK_INT >= 30) insets.getInsets(WindowInsets.Type.navigationBars()).bottom
        else @Suppress("DEPRECATION") insets.systemWindowInsetBottom
        if (b != bottomInset) {
            bottomInset = b
            requestLayout()
        }
        return super.onApplyWindowInsets(insets)
    }

    override fun onMeasure(w: Int, h: Int) {
        setMeasuredDimension(MeasureSpec.getSize(w), (stripH + rowH * 4 + 6 * dp).toInt() + bottomInset)
    }

    override fun onSizeChanged(w: Int, h: Int, ow: Int, oh: Int) = place()

    private fun place() {
        val gap = 3 * dp
        var y = stripH + 2 * dp
        for (row in rows) {
            val total = row.sumOf { it.weight.toDouble() }.toFloat()
            // short rows (asdfghjkl) are centred at the width of a key of the longest row
            val unit = width / maxOf(total, 10f)
            var x = (width - unit * total) / 2
            for (k in row) {
                k.rect.set(x + gap, y + gap, x + unit * k.weight - gap, y + rowH - gap)
                x += unit * k.weight
            }
            y += rowH
        }
    }

    override fun onDraw(c: Canvas) {
        c.drawColor(bg)
        place()
        drawCandidates(c)
        for (k in rows.flatten()) {
            val special = k.action != Key.TYPE && k.action != Key.TEXT && k.action != Key.SPACE
            paint.color = when {
                k === pressed -> pressedBg
                k.action == Key.SHIFT && shift -> accent
                k.action == Key.ENTER -> accent
                special -> specialBg
                else -> keyBg
            }
            c.drawRoundRect(k.rect, 7 * dp, 7 * dp, paint)
            val sinhalaLabel = k.label.any { it in '඀'..'෿' }
            text.typeface = if (sinhalaLabel) sinhala else Typeface.DEFAULT
            text.color = if (k.action == Key.ENTER || (k.action == Key.SHIFT && shift)) 0xFFFFFFFF.toInt() else if (k.action == Key.SPACE) faint else ink
            text.textSize = when {
                k.action == Key.SPACE -> 14 * dp
                k.action == Key.SYMBOLS -> 15 * dp
                sinhalaLabel -> (if (k.label.length > 2) 17 else 23) * dp
                else -> 21 * dp
            }
            // a sign on its own is drawn on a dotted circle by the font; ink centred vertically
            val fm = text.fontMetrics
            c.drawText(k.label, k.rect.centerX(), k.rect.centerY() - (fm.ascent + fm.descent) / 2, text)
            if (k.alt.length == 1) { // the joined letters (ද්‍ව ඤ්ච …) are too wide for a corner; a long press still types them
                text.typeface = sinhala
                text.textSize = 11 * dp
                text.color = faint
                c.drawText(k.alt, k.rect.right - 9 * dp, k.rect.top + 13 * dp, text)
            }
        }
    }

    private fun drawCandidates(c: Canvas) {
        candidateRects.clear()
        if (candidates.isEmpty()) {
            text.typeface = Typeface.DEFAULT
            text.textSize = 13 * dp
            text.color = faint
            c.drawText("ලියමු · " + Prefs.LAYOUTS[layout], width / 2f, stripH / 2 + 5 * dp, text)
            return
        }
        var x = 6 * dp
        text.textSize = 19 * dp
        for ((i, w) in candidates.withIndex()) {
            text.typeface = if (w.any { it in '඀'..'෿' }) sinhala else Typeface.DEFAULT
            val tw = text.measureText(w) + 22 * dp
            val r = RectF(x, 6 * dp, x + tw, stripH - 4 * dp)
            if (r.left > width) break
            candidateRects.add(r)
            if (i == selected) {
                paint.color = accent
                c.drawRoundRect(r, 8 * dp, 8 * dp, paint)
            }
            text.color = if (i == selected) 0xFFFFFFFF.toInt() else ink
            val fm = text.fontMetrics
            c.drawText(w, r.centerX(), r.centerY() - (fm.ascent + fm.descent) / 2, text)
            x += tw + 4 * dp
        }
    }

    // ---- touch

    private val repeat = object : Runnable {
        override fun run() {
            val k = pressed ?: return
            if (k.action == Key.BACKSPACE) {
                listener?.onKey(k, false, false)
                handler.postDelayed(this, 60)
            } else if (k.alt.isNotEmpty() && !longDone) {
                longDone = true
                performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
                listener?.onKey(k, shift, true)
                afterKey(k)
            }
        }
    }

    override fun onTouchEvent(e: MotionEvent): Boolean {
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN -> {
                if (e.y < stripH) {
                    val i = candidateRects.indexOfFirst { it.contains(e.x, e.y) }
                    if (i >= 0) listener?.onPick(i)
                    return true
                }
                pressed = rows.flatten().minByOrNull { k ->
                    val dx = maxOf(k.rect.left - e.x, 0f, e.x - k.rect.right)
                    val dy = maxOf(k.rect.top - e.y, 0f, e.y - k.rect.bottom)
                    dx * dx + dy * dy
                }
                longDone = false
                performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                pressed?.let {
                    if (it.action == Key.BACKSPACE) listener?.onKey(it, false, false)
                    handler.postDelayed(repeat, 380)
                }
                invalidate()
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> {
                handler.removeCallbacks(repeat)
                val k = pressed
                pressed = null
                if (k != null && e.actionMasked == MotionEvent.ACTION_UP && !longDone && k.action != Key.BACKSPACE) {
                    when (k.action) {
                        Key.SHIFT -> { shift = !shift; build() }
                        Key.SYMBOLS -> { symbols = !symbols; build() }
                        else -> { listener?.onKey(k, shift, false); afterKey(k) }
                    }
                }
                invalidate()
            }
        }
        return true
    }

    /** Shift is for one letter. */
    private fun afterKey(k: Key) {
        if (shift && k.action == Key.TYPE) {
            shift = false
            build()
        }
    }

    companion object {
        private val WIJE = mapOf(
            '`' to "්‍ර", '~' to "ර්‍", 'q' to "ු", 'Q' to "ූ", 'w' to "අ", 'W' to "උ", 'e' to "ැ", 'E' to "ෑ", 'r' to "ර", 'R' to "ඍ",
            't' to "එ", 'T' to "ඔ", 'y' to "හ", 'Y' to "ශ", 'u' to "ම", 'U' to "ඹ", 'i' to "ස", 'I' to "ෂ", 'o' to "ද", 'O' to "ධ",
            'p' to "ච", 'P' to "ඡ", '[' to "ඤ", '{' to "ඥ", '\\' to "්‍", '|' to "‍්", 'a' to "්", 'A' to "ෟ", 's' to "ි", 'S' to "ී",
            'd' to "ා", 'D' to "ෘ", 'f' to "ෙ", 'F' to "ෆ", 'g' to "ට", 'G' to "ඨ", 'h' to "ය", 'H' to "්‍ය", 'j' to "ව", 'J' to "ළු",
            'k' to "න", 'K' to "ණ", 'l' to "ක", 'L' to "ඛ", ';' to "ත", ':' to "ථ", 'x' to "ං", 'X' to "ඃ", 'c' to "ජ", 'C' to "ඣ",
            'v' to "ඩ", 'V' to "ඪ", 'b' to "ඉ", 'B' to "ඊ", 'n' to "බ", 'N' to "භ", 'm' to "ප", 'M' to "ඵ", ',' to "ල", '<' to "ළ",
            '.' to "ග", '>' to "ඝ",
        )
        private val WIJE_SHIFT = mapOf('[' to '{', ';' to ':', ',' to '<', '.' to '>', '`' to '~', '\\' to '|')
        /** AltGr letters, typed with a long press */
        private val WIJE_ALT = mapOf('o' to "ඳ", '.' to "ඟ", 'v' to "ඬ", 'x' to "ඞ", ',' to "ඏ", 'a' to "ෳ", 'C' to "ඦ",
            'j' to "ද්‍ව", 'O' to "ද්‍ධ", 'p' to "ඤ්ච", 'P' to "ඤ්ඡ", 'c' to "ඤ්ජ", 'G' to "ට්ඨ")
    }
}
