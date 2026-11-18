package lk.arutha.liyamu

import android.content.Context
import android.content.Intent
import android.content.SharedPreferences
import android.inputmethodservice.InputMethodService
import android.os.Build
import android.os.Handler
import android.os.Looper
import android.text.InputType
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo
import android.view.inputmethod.InputConnection
import android.view.inputmethod.InputMethodInfo
import android.view.inputmethod.InputMethodManager
import android.view.inputmethod.InputMethodSubtype
import java.util.concurrent.Callable
import java.util.concurrent.ConcurrentLinkedQueue
import java.util.concurrent.Executors

/**
 * The keyboard service: sends each key to the engine and shows what it answers.
 *
 * The engine works on its own thread, so that a key is drawn as released at once and the next touch is taken
 * while the suggestions for the last are still being found. Every key goes through that thread, also those the
 * engine has nothing to do with, and what each leads to is done on the main thread in the order of the keys.
 */
class LiyamuService : InputMethodService(), KeyListener, SharedPreferences.OnSharedPreferenceChangeListener {
    private lateinit var prefs: Prefs
    private var session = 0L
    private var board: Board? = null
    private val view get() = board?.keys
    private var composing = false
    /** password and number fields get plain keys: nothing goes through the engine */
    private var plain = false
    /** the text field asks keyboards not to learn from it (a private or incognito window) */
    private var private = false
    private val worker = Executors.newSingleThreadExecutor()
    private val main = Handler(Looper.getMainLooper())
    private val done = ConcurrentLinkedQueue<() -> Unit>()
    private val drain = Runnable {
        while (true) (done.poll() ?: break)()
    }

    /** `work` on the engine's thread, then `then` with its result on the main thread. */
    private fun <T> engine(work: () -> T, then: (T) -> Unit) {
        worker.execute {
            val r = work()
            done.add { then(r) }
            main.post(drain)
        }
    }

    /** For what cannot wait (the text field is changing): after the keys still on their way. */
    private fun <T> engineNow(work: () -> T): T {
        val r = worker.submit(Callable(work)).get()
        drain.run()
        return r
    }

    override fun onCreate() {
        super.onCreate()
        prefs = Prefs(getSharedPreferences(Prefs.NAME, Context.MODE_PRIVATE))
        prefs.p.registerOnSharedPreferenceChangeListener(this)
        session = prefs.create()
        Engine.setShortcuts(prefs.shortcuts.toByteArray())
        prefs.applyLearning(this)
        Thread { Engine.warmUp() }.start()
    }

    override fun onDestroy() {
        prefs.p.unregisterOnSharedPreferenceChangeListener(this)
        super.onDestroy() // this still ends the word being typed, so the session goes after it
        val s = session
        session = 0
        engineNow { Engine.destroy(s) }
        worker.shutdown()
    }

    override fun onSharedPreferenceChanged(p: SharedPreferences?, key: String?) {
        if (key == "shortcuts") Engine.setShortcuts(prefs.shortcuts.toByteArray())
        if (key == "learn") prefs.applyLearning(this, !private)
        show(engineNow { Response(prefs.apply(session)) })
        applyLayout()
    }

    private val english get() = plain || prefs.english

    private fun applyLayout() {
        view?.setLayout(if (english) 3 else prefs.layout, plain)
        caps()
    }

    override fun onCreateInputView(): View {
        val keys = KeyboardView(this)
        keys.listener = this
        val emoji = EmojiPanel(this, getSharedPreferences("emoji", Context.MODE_PRIVATE), ::onEmoji, { onKey(BACKSPACE, false, false) }) {
            board?.showEmoji(false)
        }
        return Board(this, keys, emoji).also { board = it; applyLayout() }
    }

    override fun onStartInputView(info: EditorInfo, restarting: Boolean) {
        super.onStartInputView(info, restarting)
        engineNow { Engine.reset(session) }
        composing = false
        val cls = info.inputType and InputType.TYPE_MASK_CLASS
        val variation = info.inputType and InputType.TYPE_MASK_VARIATION
        plain = cls != InputType.TYPE_CLASS_TEXT || variation == InputType.TYPE_TEXT_VARIATION_PASSWORD ||
            variation == InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD || variation == InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD ||
            variation == InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS || variation == InputType.TYPE_TEXT_VARIATION_URI
        private = info.imeOptions and EditorInfo.IME_FLAG_NO_PERSONALIZED_LEARNING != 0
        engineNow { prefs.applyLearning(this, !private) }
        board?.showEmoji(false)
        applyLayout()
        view?.setCandidates(emptyList(), 0)
        view?.voice = !plain && voiceKeyboard() != null
    }

    /**
     * Voice typing is done by the phone's own voice keyboard (Google voice typing), not by Liyamu, which has no
     * microphone or internet permission: the key only changes to that keyboard, and it comes back here when done.
     */
    private fun voiceKeyboard(): Pair<InputMethodInfo, InputMethodSubtype>? {
        val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
        for (m in imm.enabledInputMethodList) {
            if (m.packageName == packageName) continue
            for (s in imm.getEnabledInputMethodSubtypeList(m, true)) if (s.mode == "voice") return m to s
        }
        return null
    }

    override fun onVoice() {
        val (m, s) = voiceKeyboard() ?: return
        show(engineNow { Response(Engine.commit(session)) })
        if (Build.VERSION.SDK_INT >= 28) switchInputMethod(m.id, s)
        else {
            val imm = getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager
            @Suppress("DEPRECATION")
            imm.setInputMethodAndSubtype(window?.window?.attributes?.token ?: return, m.id, s)
        }
    }

    /** The gear opens the Liyamu app: the settings, the shortcuts and the help. */
    override fun onSettings() {
        show(engineNow { Response(Engine.commit(session)) })
        requestHideSelf(0)
        startActivity(Intent(this, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
    }

    override fun onFinishInputView(finishing: Boolean) {
        show(engineNow { Response(Engine.commit(session)) })
        super.onFinishInputView(finishing)
    }

    /** The caret was moved by the user: what was being typed stays, and a new word starts. */
    override fun onUpdateSelection(oldStart: Int, oldEnd: Int, newStart: Int, newEnd: Int, cStart: Int, cEnd: Int) {
        super.onUpdateSelection(oldStart, oldEnd, newStart, newEnd, cStart, cEnd)
        if (composing && (newStart != newEnd || (cEnd != -1 && newEnd != cEnd))) {
            engineNow { Engine.reset(session) }
            currentInputConnection?.finishComposingText()
            composing = false
            view?.setCandidates(emptyList(), 0)
        }
        if (!composing) caps()
    }

    /** English: Shift comes on by itself where the text field wants a capital (the start of a sentence). */
    private fun caps() {
        val info = currentInputEditorInfo
        val on = english && !plain && info != null && info.inputType != InputType.TYPE_NULL &&
            (currentInputConnection?.getCursorCapsMode(info.inputType) ?: 0) != 0
        view?.autoShift(on)
    }

    override fun onKey(key: Key, shifted: Boolean, long: Boolean) {
        val english = english
        when (key.action) {
            Key.TYPE -> when {
                long && key.altText -> afterWord { it.commitText(key.alt, 1) } // the number in the corner of the key
                english -> plainly { it.commitText(key.label, 1) }
                else -> {
                    val ch = key.ch.code
                    engine({ Response(Engine.key(session, Engine.CHAR, ch, long)) }) { show(it) }
                }
            }
            // digits and punctuation end the word and are inserted as they are
            Key.TEXT -> afterWord { it.commitText(key.label, 1) }
            Key.SPACE -> engineKey(english, Engine.SPACE) { it.commitText(" ", 1) }
            Key.BACKSPACE -> engineKey(english, Engine.BACKSPACE) {
                if (it.getSelectedText(0).isNullOrEmpty()) it.deleteSurroundingTextInCodePoints(1, 0) else it.commitText("", 1)
            }
            Key.ENTER -> engineKey(english, Engine.ENTER) { enter(it) }
            Key.LANGUAGE ->
                if (long) (getSystemService(Context.INPUT_METHOD_SERVICE) as InputMethodManager).showInputMethodPicker()
                else prefs.english = !prefs.english // the listener ends the word and changes the keys
            Key.EMOJI -> board?.showEmoji(true)
        }
    }

    private fun onEmoji(e: String) = afterWord { it.commitText(e, 1) }

    /** A key the engine has nothing to do with, in its turn after the keys before it. */
    private fun plainly(then: (InputConnection) -> Unit) = engine({}) {
        currentInputConnection?.let(then)
        caps()
    }

    /** Ends the word being typed, then `then`. */
    private fun afterWord(then: (InputConnection) -> Unit) = engine({ Response(Engine.commit(session)) }) {
        show(it)
        currentInputConnection?.let(then)
        caps()
    }

    /** Sends a key to the engine; `otherwise` does what the key normally does when the engine passes it on. */
    private fun engineKey(english: Boolean, kind: Int, otherwise: (InputConnection) -> Unit) {
        if (english) return plainly(otherwise)
        engine({ Response(Engine.key(session, kind, 0, false)) }) { r ->
            show(r)
            if (!r.handled) currentInputConnection?.let(otherwise)
        }
    }

    private fun enter(ic: InputConnection) {
        val info = currentInputEditorInfo
        val action = info.imeOptions and EditorInfo.IME_MASK_ACTION
        if (info.imeOptions and EditorInfo.IME_FLAG_NO_ENTER_ACTION == 0 && action != EditorInfo.IME_ACTION_NONE &&
            action != EditorInfo.IME_ACTION_UNSPECIFIED) {
            ic.performEditorAction(action)
        } else {
            ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_DOWN, KeyEvent.KEYCODE_ENTER))
            ic.sendKeyEvent(KeyEvent(KeyEvent.ACTION_UP, KeyEvent.KEYCODE_ENTER))
        }
    }

    /** A tapped suggestion is followed by a space, as on other phone keyboards. */
    override fun onPick(index: Int) = engine({ Response(Engine.pick(session, index)) }) { r ->
        show(r)
        if (r.commit.isNotEmpty()) currentInputConnection?.commitText(" ", 1)
    }

    /**
     * Inserts what the engine committed and shows the word being typed underlined, with its suggestions. After
     * a word and a space there is no word being typed: the suggestions are then the words that may come next,
     * none of them highlighted.
     */
    private fun show(r: Response) {
        val ic = currentInputConnection ?: return
        ic.beginBatchEdit()
        if (r.commit.isNotEmpty()) {
            ic.commitText(r.commit, 1) // replaces the underlined word
            composing = false
        }
        if (r.preedit.isNotEmpty()) {
            ic.setComposingText(r.preedit, 1)
            composing = true
        } else if (composing) {
            ic.setComposingText("", 1)
            ic.finishComposingText()
            composing = false
        }
        ic.endBatchEdit()
        view?.setCandidates(r.candidates, if (r.preedit.isEmpty()) -1 else r.selected)
    }

    companion object {
        private val BACKSPACE = Key("", action = Key.BACKSPACE)
    }
}
