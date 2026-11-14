package lk.arutha.liyamu

import android.content.Context
import android.content.res.Configuration
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
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
    fun onVoice()
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
    /** shows the microphone at the end of the suggestion row */
    var voice = false
        set(v) { field = v; invalidate() }
    private val micRect = RectF()
    private var longDone = false
    private val handler = Handler(Looper.getMainLooper())

    private val dp = resources.displayMetrics.density
    private val dark = resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
    private val sinhala: Typeface = Typeface.createFromAsset(context.assets, "abhaya.ttf")
    // Sepia by day (ivory keys on parchment, brown ink, maroon accents) and warm charcoal by night (gold accents):
    // the colours of the icon.
    private val bg = if (dark) 0xFF1B1512.toInt() else 0xFFE7DCC4.toInt()
    private val strip = if (dark) 0xFF231B17.toInt() else 0xFFDFD2B6.toInt()
    private val keyBg = if (dark) 0xFF362B25.toInt() else 0xFFFCF7EC.toInt()
    private val keyEdge = if (dark) 0xFF120D0B.toInt() else 0xFFC9B892.toInt()
    private val specialBg = if (dark) 0xFF2A211C.toInt() else 0xFFD6C7A6.toInt()
    private val pressedBg = if (dark) 0xFF5A4636.toInt() else 0xFFF3CF7A.toInt()
    private val accent = if (dark) 0xFFE3B75A.toInt() else 0xFF6E1C22.toInt()
    private val onAccent = if (dark) 0xFF2A1A12.toInt() else 0xFFFFF8E8.toInt()
    private val ink = if (dark) 0xFFF3E9D6.toInt() else 0xFF3B2A20.toInt()
    private val faint = if (dark) 0xFFA8998A.toInt() else 0xFF8A765F.toInt()
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
        paint.color = strip
        c.drawRect(0f, 0f, width.toFloat(), stripH, paint)
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
            // a darker edge under the key gives it a little height
            val face = paint.color
            paint.color = keyEdge
            c.drawRoundRect(k.rect.left, k.rect.top + 1.5f * dp, k.rect.right, k.rect.bottom + 1.5f * dp, 8 * dp, 8 * dp, paint)
            paint.color = face
            c.drawRoundRect(k.rect, 8 * dp, 8 * dp, paint)
            val sinhalaLabel = k.label.any { it in '඀'..'෿' }
            text.typeface = if (sinhalaLabel) sinhala else Typeface.DEFAULT
            text.color = if (k.action == Key.ENTER || (k.action == Key.SHIFT && shift)) onAccent else if (k.action == Key.SPACE) faint else ink
            text.textSize = when {
                k.action == Key.SPACE -> 14 * dp
                k.action == Key.SYMBOLS -> 15 * dp
                sinhalaLabel -> (if (k.label.length > 2) 17 else 23) * dp
                else -> 21 * dp
            }
            if (drawIcon(c, k, text.color, face)) continue
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
        drawPreview(c)
    }

    private val shape = Path()

    /** Shift, backspace, Enter and the layout key are drawn as solid shapes: the font's outline arrows are too faint. */
    private fun drawIcon(c: Canvas, k: Key, colour: Int, face: Int): Boolean {
        val x = k.rect.centerX()
        val y = k.rect.centerY()
        val u = dp
        paint.color = colour
        paint.strokeCap = Paint.Cap.ROUND
        paint.strokeJoin = Paint.Join.ROUND
        shape.rewind()
        when (k.action) {
            Key.SHIFT -> {
                shape.moveTo(x, y - 10 * u); shape.lineTo(x + 10 * u, y); shape.lineTo(x + 4.5f * u, y)
                shape.lineTo(x + 4.5f * u, y + 8.5f * u); shape.lineTo(x - 4.5f * u, y + 8.5f * u)
                shape.lineTo(x - 4.5f * u, y); shape.lineTo(x - 10 * u, y); shape.close()
                paint.style = Paint.Style.FILL_AND_STROKE
                paint.strokeWidth = 1.6f * u
                c.drawPath(shape, paint)
            }
            Key.ENTER -> {
                paint.style = Paint.Style.STROKE
                paint.strokeWidth = 3 * u
                shape.moveTo(x + 8.5f * u, y - 7.5f * u); shape.lineTo(x + 8.5f * u, y + 3 * u); shape.lineTo(x - 3 * u, y + 3 * u)
                c.drawPath(shape, paint)
                shape.rewind()
                shape.moveTo(x - 10 * u, y + 3 * u); shape.lineTo(x - 3 * u, y - 3.5f * u); shape.lineTo(x - 3 * u, y + 9.5f * u); shape.close()
                paint.style = Paint.Style.FILL_AND_STROKE
                paint.strokeWidth = 1.6f * u
                c.drawPath(shape, paint)
            }
            Key.BACKSPACE -> {
                shape.moveTo(x - 12 * u, y); shape.lineTo(x - 5 * u, y - 8 * u); shape.lineTo(x + 11 * u, y - 8 * u)
                shape.lineTo(x + 11 * u, y + 8 * u); shape.lineTo(x - 5 * u, y + 8 * u); shape.close()
                paint.style = Paint.Style.FILL_AND_STROKE
                paint.strokeWidth = 1.6f * u
                c.drawPath(shape, paint)
                paint.style = Paint.Style.STROKE
                paint.strokeWidth = 2 * u
                paint.color = face
                c.drawLine(x - 0.5f * u, y - 3.5f * u, x + 6.5f * u, y + 3.5f * u, paint)
                c.drawLine(x - 0.5f * u, y + 3.5f * u, x + 6.5f * u, y - 3.5f * u, paint)
            }
            Key.LAYOUT -> {
                // a globe in the ink of the keys
                val r = 9.5f * u
                paint.style = Paint.Style.STROKE
                paint.strokeWidth = 1.7f * u
                c.drawCircle(x, y, r, paint)
                c.drawOval(x - 4.2f * u, y - r, x + 4.2f * u, y + r, paint)
                c.drawLine(x - r, y, x + r, y, paint)
                c.drawLine(x - 8.2f * u, y - 4.8f * u, x + 8.2f * u, y - 4.8f * u, paint)
                c.drawLine(x - 8.2f * u, y + 4.8f * u, x + 8.2f * u, y + 4.8f * u, paint)
            }
            else -> { paint.style = Paint.Style.FILL; return false }
        }
        paint.style = Paint.Style.FILL
        return true
    }

    /** The pressed letter, larger, above the finger that hides it. */
    private fun drawPreview(c: Canvas) {
        val k = pressed ?: return
        if (k.action != Key.TYPE && k.action != Key.TEXT) return
        val w = maxOf(k.rect.width() * 1.35f, 46 * dp)
        val cx = k.rect.centerX().coerceIn(w / 2 + 2 * dp, width - w / 2 - 2 * dp)
        val r = RectF(cx - w / 2, k.rect.top - 58 * dp, cx + w / 2, k.rect.top - 4 * dp)
        paint.color = keyEdge
        c.drawRoundRect(r.left, r.top + 2 * dp, r.right, r.bottom + 2 * dp, 12 * dp, 12 * dp, paint)
        paint.color = accent
        c.drawRoundRect(r, 12 * dp, 12 * dp, paint)
        text.typeface = if (k.label.any { it in '\u0D80'..'\u0DFF' }) sinhala else Typeface.DEFAULT
        text.color = onAccent
        text.textSize = 30 * dp
        val fm = text.fontMetrics
        c.drawText(k.label, r.centerX(), r.centerY() - (fm.ascent + fm.descent) / 2, text)
    }

    private fun drawCandidates(c: Canvas) {
        candidateRects.clear()
        val end = if (voice) width - stripH - 4 * dp else width.toFloat()
        if (voice) drawMic(c)
        if (candidates.isEmpty()) {
            text.typeface = Typeface.DEFAULT
            text.textSize = 13 * dp
            text.color = faint
            text.letterSpacing = 0.04f
            c.drawText("ලියමු  ·  " + Prefs.LAYOUTS[layout], width / 2f, stripH / 2 + 5 * dp, text)
            text.letterSpacing = 0f
            return
        }
        var x = 6 * dp
        text.textSize = 19 * dp
        for ((i, w) in candidates.withIndex()) {
            text.typeface = if (w.any { it in '඀'..'෿' }) sinhala else Typeface.DEFAULT
            val tw = text.measureText(w) + 22 * dp
            val r = RectF(x, 6 * dp, x + tw, stripH - 4 * dp)
            if (i > 0 && r.right > end) break
            if (r.right > end) r.right = end
            candidateRects.add(r)
            if (i == selected) {
                paint.color = accent
                c.drawRoundRect(r, 8 * dp, 8 * dp, paint)
            }
            text.color = if (i == selected) onAccent else ink
            val fm = text.fontMetrics
            c.drawText(w, r.centerX(), r.centerY() - (fm.ascent + fm.descent) / 2, text)
            // a hairline between the words that are not highlighted
            if (i != selected && i + 1 != selected && i + 1 < candidates.size) {
                paint.color = keyEdge
                c.drawRect(r.right + 1.5f * dp, r.top + 9 * dp, r.right + 2.5f * dp, r.bottom - 9 * dp, paint)
            }
            x += tw + 4 * dp
        }
    }

    /** A small microphone, drawn in lines so that it takes the colours of the theme. */
    private fun drawMic(c: Canvas) {
        micRect.set(width - stripH - 2 * dp, 0f, width - 2 * dp, stripH)
        val cx = micRect.centerX()
        val cy = micRect.centerY()
        paint.color = specialBg
        c.drawCircle(cx, cy, 17 * dp, paint)
        paint.color = accent
        c.drawRoundRect(cx - 3.2f * dp, cy - 10 * dp, cx + 3.2f * dp, cy + 2 * dp, 3.2f * dp, 3.2f * dp, paint)
        paint.style = Paint.Style.STROKE
        paint.strokeWidth = 1.8f * dp
        paint.strokeCap = Paint.Cap.ROUND
        c.drawArc(cx - 6.5f * dp, cy - 8 * dp, cx + 6.5f * dp, cy + 5.5f * dp, 0f, 180f, false, paint)
        c.drawLine(cx, cy + 5.5f * dp, cx, cy + 9.5f * dp, paint)
        c.drawLine(cx - 3.5f * dp, cy + 9.5f * dp, cx + 3.5f * dp, cy + 9.5f * dp, paint)
        paint.style = Paint.Style.FILL
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
                    if (voice && micRect.contains(e.x, e.y)) {
                        performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                        listener?.onVoice()
                        return true
                    }
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
