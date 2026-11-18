package lk.arutha.liyamu

import android.content.Context
import android.content.res.Configuration
import android.graphics.Canvas
import android.graphics.Paint
import android.graphics.Path
import android.graphics.RectF
import android.graphics.Typeface
import android.graphics.drawable.Drawable
import android.os.Handler
import android.os.Looper
import android.os.SystemClock
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View

/** A key: what it shows and what it does. */
class Key(
    val label: String,
    /** the character sent to the engine (US-QWERTY key), or 0 for a special key */
    val ch: Char = 0.toChar(),
    val action: Int = TYPE,
    val weight: Float = 1f,
    /** small label in the corner: what a long press types */
    val alt: String = "",
    /** the long press inserts `alt` as it is (a digit); otherwise it is the layout's AltGr letter */
    val altText: Boolean = false,
) {
    val rect = RectF()

    companion object {
        const val TYPE = 0      // ch goes to the engine
        const val TEXT = 1      // label is inserted as it is (digits, punctuation)
        const val SHIFT = 2
        const val BACKSPACE = 3
        const val SYMBOLS = 4
        const val LANGUAGE = 5  // Sinhala / English; a long press opens the phone's list of keyboards
        const val SPACE = 6
        const val ENTER = 7
        const val EMOJI = 8
        const val MORE = 9      // the other page of symbols
    }
}

interface KeyListener {
    fun onKey(key: Key, shifted: Boolean, long: Boolean)
    fun onPick(index: Int)
    fun onVoice()
    fun onSettings()
}

/**
 * The colours of the icon: sepia by day (ivory keys on parchment, brown ink, maroon accents) and warm charcoal
 * by night (gold accents).
 */
class Palette(context: Context) {
    val dark = context.resources.configuration.uiMode and Configuration.UI_MODE_NIGHT_MASK == Configuration.UI_MODE_NIGHT_YES
    val bg = if (dark) 0xFF1B1512.toInt() else 0xFFE7DCC4.toInt()
    val key = if (dark) 0xFF362B25.toInt() else 0xFFFCF7EC.toInt()
    val edge = if (dark) 0xFF120D0B.toInt() else 0xFFC9B892.toInt()
    val special = if (dark) 0xFF2A211C.toInt() else 0xFFD6C7A6.toInt()
    val pressed = if (dark) 0xFF5A4636.toInt() else 0xFFF3CF7A.toInt()
    val accent = if (dark) 0xFFE3B75A.toInt() else 0xFF6E1C22.toInt()
    val onAccent = if (dark) 0xFF2A1A12.toInt() else 0xFFFFF8E8.toInt()
    val ink = if (dark) 0xFFF3E9D6.toInt() else 0xFF3B2A20.toInt()
    val faint = if (dark) 0xFFA8998A.toInt() else 0xFF8A765F.toInt()
}

/**
 * The keyboard: a row of suggestions, then the keys. Drawn by hand, one view.
 * The sizes are those of Gboard, measured on a 411 dp wide phone: ten keys across, 5.3 dp between them, rows
 * 59 dp apart with keys 47 dp tall, letters in Google Sans at 27.5 dp.
 */
class KeyboardView(context: Context) : View(context) {
    var listener: KeyListener? = null
    private var rows: List<List<Key>> = emptyList()
    private var keys: List<Key> = emptyList()
    /** 0 Wijesekara, 1 Easy Singlish, 2 Helakuru keys, 3 English */
    private var layout = 1
    /** password, e-mail and web address fields: letters only, no emoji or language key */
    private var plain = false
    /** 0 letters, 1 and 2 the pages of symbols */
    private var symbols = 0
    /** 0 off, 1 for one letter, 2 locked (a double tap) */
    private var shift = 0
    private var shiftAt = 0L
    private var candidates: List<String> = emptyList()
    private var selected = 0
    private val candidateRects = ArrayList<RectF>()
    private var pressed: Key? = null
    private var pointer = -1
    /** shows the microphone at the end of the suggestion row */
    var voice = false
        set(v) { field = v; invalidate() }
    private val micRect = RectF()
    private val gearRect = RectF()
    private var longDone = false
    private val handler = Handler(Looper.getMainLooper())

    private val dp = resources.displayMetrics.density
    private val sinhala: Typeface = Typeface.createFromAsset(context.assets, "abhaya.ttf")
    private val latin: Typeface = Typeface.createFromAsset(context.assets, "googlesans.ttf")
    private val pal = Palette(context)
    private val paint = Paint(Paint.ANTI_ALIAS_FLAG)
    private val text = Paint(Paint.ANTI_ALIAS_FLAG).apply { textAlign = Paint.Align.CENTER }
    private val preview = RectF()
    private val icons = HashMap<Int, Drawable>()

    private val stripH = 48 * dp
    private val rowH = 59 * dp
    private val gapX = 5.33f * dp
    private val gapY = 12 * dp
    private val side = 1.9f * dp
    private val radius = 6 * dp

    init {
        isHapticFeedbackEnabled = true
        build()
    }

    fun setLayout(l: Int, plainField: Boolean) {
        if (l == layout && plainField == plain && symbols == 0) return
        layout = l
        plain = plainField
        symbols = 0
        shift = 0
        build()
    }

    /** English: a capital at the start of a sentence, as the text field asks. A locked shift stays. */
    fun autoShift(on: Boolean) {
        val s = if (on) 1 else 0
        if (shift == 2 || shift == s || symbols != 0) return
        shift = s
        build()
    }

    /** `sel` is the highlighted suggestion; -1 for none (the words that may come next). */
    fun setCandidates(c: List<String>, sel: Int) {
        if (c.isEmpty() && candidates.isEmpty()) return
        candidates = c
        selected = sel
        invalidate()
    }

    // ---- the layouts

    private fun letters(s: String, hints: String = "") =
        s.mapIndexed { i, c -> Key(c.toString(), c, alt = hints.getOrNull(i)?.toString() ?: "", altText = true) }

    private fun texts(s: String) = s.map { Key(it.toString(), action = Key.TEXT) }

    private fun bottom(): List<Key> {
        val english = layout == 3
        val wide = plain
        return listOfNotNull(
            Key(if (symbols == 0) "?123" else if (english) "ABC" else "අආඉ", action = Key.SYMBOLS, weight = 1.5f),
            Key(if (symbols == 2) "<" else ",", action = Key.TEXT),
            if (wide) null else Key("", action = Key.EMOJI),
            if (wide) null else Key("", action = Key.LANGUAGE),
            Key(if (plain) "" else if (english) "English" else "සිංහල", action = Key.SPACE, weight = if (wide) 5f else 3f),
            Key(if (symbols == 2) ">" else ".", action = Key.TEXT),
            Key("", action = Key.ENTER, weight = 1.5f),
        )
    }

    private fun build() {
        val up = shift != 0
        val back = Key("", action = Key.BACKSPACE, weight = 1.5f)
        rows = when {
            symbols == 1 -> listOf(
                texts("1234567890"),
                texts("@#\$_&-+()/"),
                listOf(Key("=\\<", action = Key.MORE, weight = 1.5f)) + texts("*\"':;!?") + back,
                bottom(),
            )
            symbols == 2 -> listOf(
                texts("~`|•√π÷×§∆"),
                texts("£¢€¥^°={}\\"),
                listOf(Key("?123", action = Key.MORE, weight = 1.5f)) + texts("%©®™✓[]") + back,
                bottom(),
            )
            layout == 0 -> listOf(
                wije("qwertyuiop["),
                wije("asdfghjkl;`"),
                listOf(Key("", action = Key.SHIFT)) + wije("xcvbnm,.\\") + Key("", action = Key.BACKSPACE),
                bottom(),
            )
            else -> listOf(
                letters(if (up) "QWERTYUIOP" else "qwertyuiop", "1234567890"),
                letters(if (up) "ASDFGHJKL" else "asdfghjkl"),
                listOf(Key("", action = Key.SHIFT, weight = 1.5f)) + letters(if (up) "ZXCVBNM" else "zxcvbnm") + back,
                bottom(),
            )
        }
        keys = rows.flatten()
        place()
        invalidate()
    }

    /** Wijesekara keys: the label is what the key types, the character is its US-QWERTY key. */
    private fun wije(keys: String) = keys.map { k ->
        val c = if (shift != 0) WIJE_SHIFT[k] ?: k.uppercaseChar() else k
        Key(WIJE[c] ?: c.toString(), c, alt = WIJE_ALT[c] ?: "")
    }

    // ---- measuring and drawing

    /** room under the keys for the system's gesture bar and keyboard-switch button */
    private var bottomInset = 0

    fun setBottomInset(b: Int) {
        if (b != bottomInset) {
            bottomInset = b
            requestLayout()
        }
    }

    override fun onMeasure(w: Int, h: Int) {
        setMeasuredDimension(MeasureSpec.getSize(w), (stripH + rowH * 4 + 30 * dp).toInt() + bottomInset)
    }

    override fun onSizeChanged(w: Int, h: Int, ow: Int, oh: Int) = place()

    private fun place() {
        if (width == 0) return
        var y = stripH
        for (row in rows) {
            val total = row.sumOf { it.weight.toDouble() }.toFloat()
            // short rows (asdfghjkl) are centred at the width of a key of a row of ten
            val unit = (width - 2 * side) / maxOf(total, 10f)
            var x = (width - unit * total) / 2
            for (k in row) {
                k.rect.set(x + gapX / 2, y + gapY / 2, x + unit * k.weight - gapX / 2, y + rowH - gapY / 2)
                x += unit * k.weight
            }
            y += rowH
        }
    }

    private fun isSinhala(s: String) = s.any { it in '඀'..'෿' }

    /** A vowel sign or other mark with no letter under it. */
    private fun isSign(s: String) = s.isNotEmpty() && (s == REPAYA || s[0] == '\u200D' || s[0] in '\u0DCA'..'\u0DDF' || s[0] in '\u0D81'..'\u0D83' || s[0] in '\u0DF2'..'\u0DF3')

    private val signPath = Path()
    private val signBounds = RectF()

    /**
     * A sign alone is put on a dotted circle by the font. After a no-break space it is shaped with
     * no circle; its outline is then centred on the key by its own ink, a little larger. The small
     * signs (ං ් ු) are enlarged further, towards [fit], so that they are not lost on the key.
     */
    private fun drawSign(c: Canvas, s: String, cx: Float, cy: Float, scale: Float, fit: Float) {
        val t = when {
            s == REPAYA -> "$s\u00A0" // the repaya sits over the letter after it
            s[0] == '\u200D' -> "\u00A0" + s.substring(1)
            else -> "\u00A0$s"
        }
        text.textSize *= scale
        text.getTextPath(t, 0, t.length, 0f, 0f, signPath)
        signPath.computeBounds(signBounds, true)
        c.save()
        val more = (fit / maxOf(signBounds.width(), signBounds.height(), 1f)).coerceIn(1f, 1.2f)
        c.translate(cx, cy)
        c.scale(more, more)
        c.translate(-signBounds.centerX(), -signBounds.centerY())
        c.drawPath(signPath, text)
        c.restore()
    }

    /** Latin letters stand on one line, centred as capitals; Sinhala is centred on its own ink. */
    private fun baseline(sinhalaLabel: Boolean, centre: Float): Float {
        if (!sinhalaLabel) return centre + text.textSize * 0.358f
        val fm = text.fontMetrics
        return centre - (fm.ascent + fm.descent) / 2
    }

    override fun onDraw(c: Canvas) {
        c.drawColor(pal.bg)
        drawStrip(c)
        for (k in keys) {
            val shifted = k.action == Key.SHIFT && shift != 0
            val face = when {
                k === pressed -> pal.pressed
                shifted || k.action == Key.ENTER -> pal.accent
                k.action == Key.TYPE || k.action == Key.SPACE || k.action == Key.EMOJI || k.action == Key.LANGUAGE -> pal.key
                k.action == Key.TEXT && symbols != 0 && k.rect.top < stripH + 3 * rowH -> pal.key
                else -> pal.special
            }
            // "?123" and Enter are round, as on Gboard
            val r = if (k.action == Key.SYMBOLS || k.action == Key.ENTER) k.rect.height() / 2 else radius
            // a darker edge under the key gives it a little height
            paint.color = pal.edge
            c.drawRoundRect(k.rect.left, k.rect.top + dp, k.rect.right, k.rect.bottom + dp, r, r, paint)
            paint.color = face
            c.drawRoundRect(k.rect, r, r, paint)
            val colour = if (k.action == Key.ENTER || shifted) pal.onAccent else pal.ink
            val icon = when (k.action) {
                Key.SHIFT -> if (shift == 2) R.drawable.ic_shift_lock else if (shift == 1) R.drawable.ic_shift_on else R.drawable.ic_shift
                Key.BACKSPACE -> R.drawable.ic_backspace
                Key.ENTER -> R.drawable.ic_enter
                Key.EMOJI -> R.drawable.ic_emoji
                Key.LANGUAGE -> R.drawable.ic_globe
                else -> 0
            }
            if (icon != 0) {
                drawIcon(c, icon, k.rect.centerX(), k.rect.centerY(), 24 * dp, colour)
                continue
            }
            val si = isSinhala(k.label)
            text.typeface = if (si) sinhala else latin
            text.color = if (k.action == Key.SPACE) pal.faint else colour
            text.textSize = when {
                k.action == Key.SPACE -> (if (si) 16 else 14) * dp
                k.action == Key.SYMBOLS || k.action == Key.MORE -> 17 * dp
                isSign(k.label) -> 27 * dp
                si -> (if (k.label.length > 2) 18 else 27) * dp
                k.action == Key.TEXT -> 24 * dp
                else -> 27.5f * dp
            }
            if (si && !isSign(k.label)) { // the widest letters (ඤ ඥ) must stay inside a Wijesekara key
                val room = k.rect.width() - 5 * dp
                val w = text.measureText(k.label)
                if (w > room) text.textSize *= room / w
            }
            if (isSign(k.label)) {
                // the joining and the touching hal kirima look like the plain one: a mark under each tells them apart
                val mark = when (k.label) { JOINING -> "><"; TOUCHING -> ")("; else -> "" }
                drawSign(c, k.label, k.rect.centerX(), k.rect.centerY() - (if (mark.isEmpty()) 0f else 8 * dp), 1f, 19 * dp)
                if (mark.isNotEmpty()) {
                    text.typeface = latin
                    text.textSize = 15 * dp
                    c.drawText(mark, k.rect.centerX(), k.rect.bottom - 8 * dp, text)
                }
            }
            else c.drawText(k.label, k.rect.centerX(), baseline(si, k.rect.centerY()), text)
            if (k.alt.length == 1) { // the joined letters (ද්‍ව ඤ්ච …) are too wide for a corner; a long press still types them
                text.typeface = if (k.altText) latin else sinhala
                text.textSize = 11 * dp
                text.color = pal.faint
                if (isSign(k.alt)) drawSign(c, k.alt, k.rect.right - 8 * dp, k.rect.top + 9 * dp, 1.2f, 0f)
                else c.drawText(k.alt, k.rect.right - 6.5f * dp, k.rect.top + 12 * dp, text)
            }
        }
        drawPreview(c)
    }

    private fun drawIcon(c: Canvas, id: Int, x: Float, y: Float, size: Float, colour: Int) {
        val d = icons.getOrPut(id) { context.getDrawable(id)!!.mutate() }
        val h = (size / 2).toInt()
        d.setBounds(x.toInt() - h, y.toInt() - h, x.toInt() + h, y.toInt() + h)
        d.setTint(colour)
        d.draw(c)
    }

    private fun previewed(k: Key) = (k.action == Key.TYPE || k.action == Key.TEXT) && k.label.isNotEmpty()

    /** The pressed letter, larger, above the finger that hides it. */
    private fun drawPreview(c: Canvas) {
        val k = pressed ?: return
        if (!previewed(k)) return
        val w = maxOf(k.rect.width() * 1.35f, 46 * dp)
        val cx = k.rect.centerX().coerceIn(w / 2 + 2 * dp, width - w / 2 - 2 * dp)
        preview.set(cx - w / 2, k.rect.top - 60 * dp, cx + w / 2, k.rect.top - 5 * dp)
        paint.color = pal.edge
        c.drawRoundRect(preview.left, preview.top + 2 * dp, preview.right, preview.bottom + 2 * dp, 12 * dp, 12 * dp, paint)
        paint.color = pal.accent
        c.drawRoundRect(preview, 12 * dp, 12 * dp, paint)
        val si = isSinhala(k.label)
        text.typeface = if (si) sinhala else latin
        text.color = pal.onAccent
        text.textSize = 32 * dp
        if (isSign(k.label)) drawSign(c, k.label, preview.centerX(), preview.centerY(), 1f, 24 * dp)
        else c.drawText(k.label, preview.centerX(), baseline(si, preview.centerY()), text)
    }

    /** The suggestions, then the settings gear and the microphone. */
    private fun drawStrip(c: Canvas) {
        candidateRects.clear()
        val slot = 44 * dp
        var end = width - 4 * dp
        if (voice) {
            micRect.set(end - slot, 0f, end, stripH)
            paint.color = pal.key
            c.drawCircle(micRect.centerX(), micRect.centerY(), 17.5f * dp, paint)
            drawIcon(c, R.drawable.ic_mic, micRect.centerX(), micRect.centerY(), 23 * dp, pal.ink)
            end -= slot
        } else micRect.setEmpty()
        gearRect.set(end - slot, 0f, end, stripH)
        drawIcon(c, R.drawable.ic_settings, gearRect.centerX(), gearRect.centerY(), 23 * dp, pal.faint)
        end -= slot
        if (candidates.isEmpty()) {
            text.typeface = sinhala
            text.textSize = 16 * dp
            text.color = pal.faint
            val name = if (plain) "" else "  ·  " + NAMES[layout]
            c.drawText("ලියමු$name", end / 2 + 2 * dp, baseline(true, stripH / 2), text)
            return
        }
        var x = 6 * dp
        for ((i, w) in candidates.withIndex()) {
            val si = isSinhala(w)
            text.typeface = if (si) sinhala else latin
            text.textSize = (if (si) 20 else 18) * dp
            val tw = text.measureText(w) + 20 * dp
            val r = RectF(x, 6 * dp, x + tw, stripH - 6 * dp)
            if (i > 0 && r.right > end) break
            if (r.right > end) r.right = end
            candidateRects.add(r)
            if (i == selected) {
                paint.color = pal.accent
                c.drawRoundRect(r, 8 * dp, 8 * dp, paint)
            }
            text.color = if (i == selected) pal.onAccent else pal.ink
            c.drawText(w, r.centerX(), baseline(si, r.centerY()), text)
            // a hairline between the words that are not highlighted
            if (i > 0 && i != selected && i - 1 != selected) {
                paint.color = pal.edge
                c.drawRect(r.left - 2.5f * dp, r.top + 8 * dp, r.left - 1.5f * dp, r.bottom - 8 * dp, paint)
            }
            x += tw + 4 * dp
        }
    }

    // ---- touch

    private val repeat = object : Runnable {
        override fun run() {
            val k = pressed ?: return
            if (k.action == Key.BACKSPACE) {
                listener?.onKey(k, false, false)
                handler.postDelayed(this, 55)
            } else if ((k.alt.isNotEmpty() || k.action == Key.LANGUAGE) && !longDone) {
                longDone = true
                performHapticFeedback(HapticFeedbackConstants.LONG_PRESS)
                listener?.onKey(k, shift != 0, true)
                afterKey(k)
            }
        }
    }

    /**
     * One key is held at a time. A second finger coming down types the key under the first at once, so fast
     * typing with two thumbs loses no letter.
     */
    override fun onTouchEvent(e: MotionEvent): Boolean {
        val i = e.actionIndex
        when (e.actionMasked) {
            MotionEvent.ACTION_DOWN, MotionEvent.ACTION_POINTER_DOWN -> {
                release(true)
                down(e.getX(i), e.getY(i), e.getPointerId(i))
            }
            MotionEvent.ACTION_UP, MotionEvent.ACTION_POINTER_UP -> if (e.getPointerId(i) == pointer) release(true)
            MotionEvent.ACTION_CANCEL -> release(false)
        }
        return true
    }

    private fun down(x: Float, y: Float, id: Int) {
        if (y < stripH) {
            when {
                micRect.contains(x, y) -> { tick(); listener?.onVoice() }
                gearRect.contains(x, y) -> { tick(); listener?.onSettings() }
                else -> {
                    val i = candidateRects.indexOfFirst { it.contains(x, y) }
                    if (i >= 0) { tick(); listener?.onPick(i) }
                }
            }
            return
        }
        val k = keys.minByOrNull { k ->
            val dx = maxOf(k.rect.left - x, 0f, x - k.rect.right)
            val dy = maxOf(k.rect.top - y, 0f, y - k.rect.bottom)
            dx * dx + dy * dy
        } ?: return
        pressed = k
        pointer = id
        longDone = false
        tick()
        if (k.action == Key.BACKSPACE) listener?.onKey(k, false, false)
        handler.postDelayed(repeat, 350)
        invalidate()
    }

    private fun tick() = performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)

    /** The finger left the key: it shows as released first, and then does its work. */
    private fun release(type: Boolean) {
        handler.removeCallbacks(repeat)
        val k = pressed ?: return
        pressed = null
        pointer = -1
        invalidate()
        if (!type || longDone || k.action == Key.BACKSPACE) return
        when (k.action) {
            Key.SHIFT -> {
                val now = SystemClock.uptimeMillis()
                shift = if (shift == 2) 0 else if (now - shiftAt < 350) 2 else if (shift == 1) 0 else 1
                shiftAt = now
                build()
            }
            Key.SYMBOLS -> { symbols = if (symbols == 0) 1 else 0; shift = 0; build() }
            Key.MORE -> { symbols = 3 - symbols; build() }
            else -> { listener?.onKey(k, shift != 0, false); afterKey(k) }
        }
    }

    /** Shift is for one letter, unless it is locked. */
    private fun afterKey(k: Key) {
        if (shift == 1 && k.action == Key.TYPE) {
            shift = 0
            build()
        }
    }

    companion object {
        /** what the suggestion row shows while there is nothing to suggest */
        val NAMES = Prefs.LAYOUTS + "English"
        private const val REPAYA = "ර්\u200D"
        private const val JOINING = "්\u200D"
        private const val TOUCHING = "\u200D්"
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
