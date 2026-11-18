package lk.arutha.liyamu

import android.content.Context
import android.content.SharedPreferences
import android.graphics.Paint
import android.graphics.Typeface
import android.graphics.drawable.GradientDrawable
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.util.TypedValue
import android.view.Gravity
import android.view.HapticFeedbackConstants
import android.view.MotionEvent
import android.view.View
import android.view.ViewGroup
import android.view.WindowInsets
import android.widget.AbsListView
import android.widget.BaseAdapter
import android.widget.FrameLayout
import android.widget.GridView
import android.widget.ImageView
import android.widget.LinearLayout
import android.widget.TextView

/** The keys and, in their place while it is open, the emoji. Both have the height of the keys. */
class Board(context: Context, val keys: KeyboardView, val emoji: EmojiPanel) : FrameLayout(context) {
    init {
        addView(keys)
        addView(emoji)
        emoji.visibility = View.GONE
    }

    /** Room under the keys and the emoji for the system's gesture bar and keyboard-switch button. */
    override fun onApplyWindowInsets(insets: WindowInsets): WindowInsets {
        val b = if (Build.VERSION.SDK_INT >= 30) insets.getInsets(WindowInsets.Type.navigationBars()).bottom
        else @Suppress("DEPRECATION") insets.systemWindowInsetBottom
        keys.setBottomInset(b)
        emoji.setPadding(0, 0, 0, b + (18 * resources.displayMetrics.density).toInt()) // above the system bar, like the keys
        return super.onApplyWindowInsets(insets)
    }

    val emojiShown get() = emoji.visibility == View.VISIBLE

    fun showEmoji(on: Boolean) {
        if (on == emojiShown) return
        if (on) emoji.open()
        emoji.visibility = if (on) View.VISIBLE else View.GONE
        keys.visibility = if (on) View.INVISIBLE else View.VISIBLE
    }

    override fun onMeasure(w: Int, h: Int) {
        keys.measure(w, h)
        emoji.measure(MeasureSpec.makeMeasureSpec(keys.measuredWidth, MeasureSpec.EXACTLY), MeasureSpec.makeMeasureSpec(keys.measuredHeight, MeasureSpec.EXACTLY))
        setMeasuredDimension(keys.measuredWidth, keys.measuredHeight)
    }
}

/**
 * Emoji, as on Gboard: a grid to scroll through, and under it the way back to the letters, the groups and
 * backspace. Drawn by the phone's own emoji font; those it does not have are left out. The ones used lately
 * come first, kept in the app's private preferences.
 */
class EmojiPanel(
    context: Context,
    private val recent: SharedPreferences,
    private val onEmoji: (String) -> Unit,
    private val onBackspace: () -> Unit,
    private val onLetters: () -> Unit,
) : LinearLayout(context) {
    private val dp = resources.displayMetrics.density
    private fun px(v: Float) = (v * dp).toInt()
    private val pal = Palette(context)
    private val glyphs = Paint()
    private val groups: List<Pair<String, List<String>>> =
        GROUPS.map { (tab, all) -> tab to all.split(' ').filter { it.isNotEmpty() && glyphs.hasGlyph(it) } }
    private var shown: List<String> = emptyList()
    private var group = -1
    private val tabs = ArrayList<TextView>()
    private val handler = Handler(Looper.getMainLooper())

    private val adapter = object : BaseAdapter() {
        override fun getCount() = shown.size
        override fun getItem(i: Int) = shown[i]
        override fun getItemId(i: Int) = i.toLong()
        override fun getView(i: Int, old: View?, parent: ViewGroup): View {
            val t = old as? TextView ?: TextView(context).apply {
                gravity = Gravity.CENTER
                setTextSize(TypedValue.COMPLEX_UNIT_DIP, 27f)
                setTextColor(pal.ink)
                includeFontPadding = false
                layoutParams = AbsListView.LayoutParams(AbsListView.LayoutParams.MATCH_PARENT, px(47f))
            }
            t.text = shown[i]
            return t
        }
    }

    private val grid = GridView(context).apply {
        numColumns = 8
        stretchMode = GridView.STRETCH_COLUMN_WIDTH
        isVerticalScrollBarEnabled = false
        selector = GradientDrawable().apply { setColor(pal.pressed); cornerRadius = 10 * dp }
        setPadding(px(4f), px(6f), px(4f), 0)
        clipToPadding = false
        adapter = this@EmojiPanel.adapter
        setOnItemClickListener { v, _, i, _ ->
            v.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
            val e = shown[i]
            onEmoji(e)
            remember(e)
        }
    }

    init {
        orientation = VERTICAL
        setBackgroundColor(pal.bg)
        addView(grid, LayoutParams(LayoutParams.MATCH_PARENT, 0, 1f))
        val bar = LinearLayout(context).apply {
            gravity = Gravity.CENTER_VERTICAL
            setPadding(px(4f), px(4f), px(4f), px(6f))
        }
        bar.addView(TextView(context).apply {
            text = "ABC"
            typeface = Typeface.createFromAsset(context.assets, "googlesans.ttf")
            setTextSize(TypedValue.COMPLEX_UNIT_DIP, 16f)
            setTextColor(pal.ink)
            gravity = Gravity.CENTER
            background = pill(pal.special)
            setOnClickListener { it.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP); onLetters() }
        }, LayoutParams(px(58f), px(42f)))
        val names = listOf("🕘") + groups.map { it.first }
        names.forEachIndexed { i, tab ->
            val t = TextView(context).apply {
                text = tab
                setTextSize(TypedValue.COMPLEX_UNIT_DIP, 17f)
                setTextColor(pal.ink)
                gravity = Gravity.CENTER
                setOnClickListener { show(i - 1) }
            }
            tabs.add(t)
            bar.addView(t, LayoutParams(0, px(38f), 1f))
        }
        val back = ImageView(context).apply {
            setImageResource(R.drawable.ic_backspace)
            setColorFilter(pal.ink)
            scaleType = ImageView.ScaleType.CENTER
            background = pill(pal.special)
        }
        // held down, backspace goes on deleting
        val again = object : Runnable {
            override fun run() { onBackspace(); handler.postDelayed(this, 55) }
        }
        back.setOnTouchListener { v, e ->
            when (e.actionMasked) {
                MotionEvent.ACTION_DOWN -> {
                    v.performHapticFeedback(HapticFeedbackConstants.KEYBOARD_TAP)
                    onBackspace()
                    handler.postDelayed(again, 350)
                }
                MotionEvent.ACTION_UP, MotionEvent.ACTION_CANCEL -> handler.removeCallbacks(again)
            }
            true
        }
        bar.addView(back, LayoutParams(px(58f), px(42f)))
        addView(bar, LayoutParams(LayoutParams.MATCH_PARENT, LayoutParams.WRAP_CONTENT))
    }

    private fun pill(colour: Int) = GradientDrawable().apply { setColor(colour); cornerRadius = 21 * dp }

    private fun recents() = (recent.getString("recent", "") ?: "").split(' ').filter { it.isNotEmpty() }

    private fun remember(e: String) {
        val list = (listOf(e) + recents().filter { it != e }).take(40)
        recent.edit().putString("recent", list.joinToString(" ")).apply()
    }

    /** Opens on the emoji used lately, or on the faces when there are none yet. */
    fun open() = show(if (recents().isEmpty()) 0 else -1)

    /** `g`: -1 for the ones used lately. */
    private fun show(g: Int) {
        group = g
        shown = if (g < 0) recents() else groups[g].second
        adapter.notifyDataSetChanged()
        grid.setSelection(0)
        tabs.forEachIndexed { i, t -> t.background = if (i == g + 1) pill(pal.key) else null }
    }

    companion object {
        /** The tab of each group, and its emoji. */
        private val GROUPS = listOf(
            "😀" to "😀 😃 😄 😁 😆 😅 🤣 😂 🙂 🙃 😉 😊 😇 🥰 😍 🤩 😘 😗 😚 😙 😋 😛 😜 🤪 😝 🤑 🤗 🤭 🤫 🤔 🤐 🤨 😐 😑 😶 😏 😒 🙄 😬 🤥 " +
                "😌 😔 😪 🤤 😴 😷 🤒 🤕 🤢 🤮 🤧 🥵 🥶 🥴 😵 🤯 🤠 🥳 😎 🤓 🧐 😕 😟 🙁 ☹️ 😮 😯 😲 😳 🥺 😦 😧 😨 😰 😥 😢 😭 😱 😖 😣 😞 " +
                "😓 😩 😫 🥱 😤 😡 😠 🤬 😈 👿 💀 ☠️ 💩 🤡 👻 👽 🤖 😺 😸 😹 😻 😼 😽 🙀 😿 😾 🙈 🙉 🙊",
            "👍" to "👍 👎 🙏 👏 🙌 👌 ✌️ 🤞 🤟 🤘 🤙 👋 🤚 🖐️ ✋ 👈 👉 👆 👇 ☝️ ✊ 👊 🤛 🤜 🤝 💪 ✍️ 🤲 👐 🤏 🖖 💅 🤳 👂 👃 👀 👁️ 🧠 👄 👅 🦷 " +
                "👶 🧒 👦 👧 🧑 👨 👩 🧓 👴 👵 👮 👷 💂 🕵️ 🧑‍⚕️ 🧑‍🎓 🧑‍🏫 🧑‍🌾 🧑‍🍳 🧑‍💻 👰 🤵 👸 🤴 🤰 🤱 👼 🎅 🙇 💁 🙅 🙆 🙋 🤦 🤷 🙎 🙍 💇 💆 🧘 " +
                "🚶 🏃 💃 🕺 👫 👭 👬 💏 💑 👪 👣",
            "❤️" to "❤️ 🧡 💛 💚 💙 💜 🖤 🤍 🤎 💔 ❣️ 💕 💞 💓 💗 💖 💘 💝 💟 ♥️ 💌 💋 💯 💢 💥 💫 💦 💨 💤 🔥 ✨ ⭐ 🌟 🎉 🎊 🎈 🎁 🎂 🏆 🥇 " +
                "✅ ✔️ ❌ ❎ ❓ ❔ ❗ ❕ ‼️ ⁉️ ⚠️ 🚫 ⛔ 🔴 🟠 🟡 🟢 🔵 🟣 ⚫ ⚪ 🟤 ☸️ 🕉️ ☮️ ☯️ ♻️ ➕ ➖ ➗ ✖️ ♾️ ©️ ®️ ™️ 🔔 🔕 🎵 🎶 💬 💭 🗯️ " +
                "⬆️ ⬇️ ⬅️ ➡️ ↗️ ↘️ ↙️ ↖️ ↩️ ↪️ 🔄 🔃 🆗 🆕 🆓 🔝 🔚 🔙 🔛 🔜",
            "🐘" to "🐘 🐶 🐱 🐭 🐹 🐰 🦊 🐻 🐼 🐨 🐯 🦁 🐮 🐷 🐸 🐵 🐒 🦍 🐔 🐧 🐦 🐤 🦆 🦅 🦉 🦚 🦜 🐺 🐗 🐴 🦄 🐝 🐛 🦋 🐌 🐞 🐜 🦟 🕷️ 🐢 🐍 " +
                "🦎 🐙 🦑 🦐 🦀 🐠 🐟 🐬 🐳 🐋 🦈 🐊 🐅 🐆 🦓 🦌 🐂 🐃 🐄 🐎 🐖 🐏 🐑 🐐 🐪 🐫 🦒 🦏 🦛 🐁 🐀 🐇 🐿️ 🦔 🦇 🐕 🐩 🐈 🐓 🦃 🕊️ " +
                "🌵 🎄 🌲 🌳 🌴 🌱 🌿 ☘️ 🍀 🎍 🎋 🍃 🍂 🍁 🍄 🌾 💐 🌷 🌹 🥀 🌺 🌸 🌼 🌻 🪷 🌞 🌝 🌛 🌜 🌚 🌕 🌙 🌎 🌍 🌏 💫 ⚡ ☄️ 🌈 ☀️ 🌤️ ⛅ " +
                "☁️ 🌧️ ⛈️ 🌩️ ❄️ ☃️ ⛄ 🌬️ 💧 ☔ 🌊 🌫️",
            "🍛" to "🍛 🍚 🍲 🍜 🍝 🍞 🥖 🥐 🥞 🧇 🧀 🍗 🍖 🥩 🥓 🍔 🍟 🍕 🌭 🥪 🌮 🌯 🥙 🥚 🍳 🥘 🥣 🥗 🍿 🧈 🍱 🍘 🍙 🍠 🍢 🍣 🍤 🍥 🥮 🍡 🥟 " +
                "🦪 🍦 🍧 🍨 🍩 🍪 🎂 🍰 🧁 🥧 🍫 🍬 🍭 🍮 🍯 🍏 🍎 🍐 🍊 🍋 🍌 🍉 🍇 🍓 🍈 🍒 🍑 🥭 🍍 🥥 🥝 🍅 🍆 🥑 🥦 🥬 🥒 🌶️ 🌽 🥕 🧄 🧅 " +
                "🥔 🥜 🌰 🍼 🥛 ☕ 🍵 🧃 🥤 🧋 🍶 🍺 🍻 🥂 🍷 🥃 🍸 🍹 🧉 🧊 🥢 🍽️ 🍴 🥄 🔪",
            "⚽" to "⚽ 🏏 🏀 🏈 ⚾ 🥎 🎾 🏐 🏉 🥏 🎱 🏓 🏸 🏒 🏑 🥍 ⛳ 🏹 🎣 🥊 🥋 🎽 🛹 ⛸️ 🎿 🏂 🏋️ 🤸 ⛹️ 🤾 🏌️ 🏇 🧘 🏄 🏊 🤽 🚣 🧗 🚵 🚴 🏆 " +
                "🥇 🥈 🥉 🏅 🎖️ 🎫 🎟️ 🎪 🎭 🎨 🎬 🎤 🎧 🎼 🎹 🥁 🎷 🎺 🎸 🪕 🎻 🎲 ♟️ 🎯 🎳 🎮 🎰 🧩 🪁",
            "🚗" to "🚗 🚕 🚙 🚌 🚎 🏎️ 🚓 🚑 🚒 🚐 🚚 🚛 🚜 🛺 🛵 🏍️ 🚲 🛴 🚨 🚔 🚍 🚘 🚖 🚡 🚠 🚟 🚃 🚋 🚞 🚝 🚄 🚅 🚈 🚂 🚆 🚇 🚊 🚉 ✈️ 🛫 🛬 " +
                "💺 🚀 🛸 🚁 🛶 ⛵ 🚤 🛥️ 🛳️ ⛴️ 🚢 ⚓ ⛽ 🚧 🚦 🚥 🚏 🗺️ 🗿 🗽 🗼 🏰 🏯 🏟️ 🎡 🎢 🎠 ⛲ ⛱️ 🏖️ 🏝️ 🏜️ 🌋 ⛰️ 🏔️ 🗻 🏕️ ⛺ 🏠 🏡 " +
                "🏘️ 🏚️ 🏗️ 🏭 🏢 🏬 🏣 🏤 🏥 🏦 🏨 🏪 🏫 🏩 💒 🏛️ ⛪ 🕌 🕍 🛕 🕋 ⛩️ 🛤️ 🛣️ 🌅 🌄 🌠 🎇 🎆 🌇 🌆 🏙️ 🌃 🌌 🌉 🌁",
            "💡" to "💡 📱 💻 ⌨️ 🖥️ 🖨️ 🖱️ 💽 💾 💿 📀 📷 📸 📹 🎥 📞 ☎️ 📟 📠 📺 📻 🎙️ ⏱️ ⏲️ ⏰ 🕰️ ⌛ ⏳ 📡 🔋 🔌 🔦 🕯️ 🪔 🧯 💸 💵 💴 💶 💷 💰 " +
                "💳 💎 ⚖️ 🧰 🔧 🔨 ⚒️ 🛠️ ⛏️ 🔩 ⚙️ 🧱 ⛓️ 🧲 🔫 💣 🧨 🪓 🔪 🗡️ ⚔️ 🛡️ 🚬 ⚰️ ⚱️ 🏺 🔮 📿 🧿 💈 ⚗️ 🔭 🔬 🕳️ 💊 💉 🩸 🩹 🩺 🌡️ " +
                "🧹 🧺 🧻 🚽 🚰 🚿 🛁 🧼 🪒 🧽 🧴 🛎️ 🔑 🗝️ 🚪 🪑 🛋️ 🛏️ 🧸 🖼️ 🛍️ 🛒 🎁 🎈 🎏 🎀 🎊 🎉 🎎 🏮 🎐 🧧 ✉️ 📩 📨 📧 💌 📥 📤 📦 " +
                "🏷️ 📪 📫 📬 📭 📮 📯 📜 📃 📄 📑 🧾 📊 📈 📉 🗒️ 🗓️ 📆 📅 🗑️ 📇 🗃️ 🗳️ 🗄️ 📋 📁 📂 🗂️ 🗞️ 📰 📓 📔 📒 📕 📗 📘 📙 📚 📖 🔖 " +
                "🧷 🔗 📎 🖇️ 📐 📏 🧮 📌 📍 ✂️ 🖊️ 🖋️ ✒️ 🖌️ 🖍️ 📝 ✏️ 🔍 🔎 🔏 🔐 🔒 🔓 👓 🕶️ 👔 👕 👖 🧣 🧤 🧥 🧦 👗 👘 🥻 👙 👚 👛 👜 👝 " +
                "🎒 👞 👟 🥾 🥿 👠 👡 👢 👑 👒 🎩 🎓 🧢 ⛑️ 💄 💍 🌂 ☂️",
            "🇱🇰" to "🇱🇰 🏳️ 🏴 🏁 🚩 🏳️‍🌈 🇮🇳 🇲🇻 🇧🇩 🇵🇰 🇳🇵 🇧🇹 🇲🇲 🇹🇭 🇰🇭 🇱🇦 🇻🇳 🇲🇾 🇸🇬 🇮🇩 🇵🇭 🇨🇳 🇯🇵 🇰🇷 🇦🇺 🇳🇿 🇦🇪 🇸🇦 🇶🇦 🇰🇼 🇴🇲 🇧🇭 🇮🇱 🇹🇷 " +
                "🇬🇧 🇮🇪 🇫🇷 🇩🇪 🇮🇹 🇪🇸 🇵🇹 🇳🇱 🇧🇪 🇨🇭 🇦🇹 🇸🇪 🇳🇴 🇩🇰 🇫🇮 🇵🇱 🇬🇷 🇷🇺 🇺🇦 🇺🇸 🇨🇦 🇲🇽 🇧🇷 🇦🇷 🇿🇦 🇪🇬 🇰🇪 🇳🇬",
        )
    }
}
