package lk.arutha.liyamu

import android.content.Context
import android.content.SharedPreferences
import android.inputmethodservice.InputMethodService
import android.text.InputType
import android.view.KeyEvent
import android.view.View
import android.view.inputmethod.EditorInfo

/** The keyboard service: sends each key to the engine and shows what it answers. */
class LiyamuService : InputMethodService(), KeyListener, SharedPreferences.OnSharedPreferenceChangeListener {
    private lateinit var prefs: Prefs
    private var session = 0L
    private var view: KeyboardView? = null
    private var composing = false
    /** password and number fields get plain keys: nothing goes through the engine */
    private var plain = false

    override fun onCreate() {
        super.onCreate()
        prefs = Prefs(getSharedPreferences(Prefs.NAME, Context.MODE_PRIVATE))
        prefs.p.registerOnSharedPreferenceChangeListener(this)
        session = prefs.create()
        Engine.setShortcuts(prefs.shortcuts.toByteArray())
        Thread { Engine.warmUp() }.start()
    }

    override fun onDestroy() {
        prefs.p.unregisterOnSharedPreferenceChangeListener(this)
        Engine.destroy(session)
        super.onDestroy()
    }

    override fun onSharedPreferenceChanged(p: SharedPreferences?, key: String?) {
        if (key == "shortcuts") Engine.setShortcuts(prefs.shortcuts.toByteArray())
        show(Response(prefs.apply(session)))
        view?.setLayout(prefs.layout)
    }

    override fun onCreateInputView(): View {
        val v = KeyboardView(this)
        v.listener = this
        v.setLayout(prefs.layout)
        view = v
        return v
    }

    override fun onStartInputView(info: EditorInfo, restarting: Boolean) {
        super.onStartInputView(info, restarting)
        Engine.reset(session)
        composing = false
        val cls = info.inputType and InputType.TYPE_MASK_CLASS
        val variation = info.inputType and InputType.TYPE_MASK_VARIATION
        plain = cls != InputType.TYPE_CLASS_TEXT || variation == InputType.TYPE_TEXT_VARIATION_PASSWORD ||
            variation == InputType.TYPE_TEXT_VARIATION_VISIBLE_PASSWORD || variation == InputType.TYPE_TEXT_VARIATION_WEB_PASSWORD ||
            variation == InputType.TYPE_TEXT_VARIATION_EMAIL_ADDRESS || variation == InputType.TYPE_TEXT_VARIATION_URI
        view?.setLayout(if (plain) 3 else prefs.layout)
        view?.setCandidates(emptyList(), 0)
    }

    override fun onFinishInputView(finishing: Boolean) {
        show(Response(Engine.commit(session)))
        super.onFinishInputView(finishing)
    }

    /** The caret was moved by the user: what was being typed stays, and a new word starts. */
    override fun onUpdateSelection(oldStart: Int, oldEnd: Int, newStart: Int, newEnd: Int, cStart: Int, cEnd: Int) {
        super.onUpdateSelection(oldStart, oldEnd, newStart, newEnd, cStart, cEnd)
        if (composing && (newStart != newEnd || (cEnd != -1 && newEnd != cEnd))) {
            currentInputConnection?.finishComposingText()
            Engine.reset(session)
            composing = false
            view?.setCandidates(emptyList(), 0)
        }
    }

    private val english get() = plain || prefs.layout == 3

    override fun onKey(key: Key, shifted: Boolean, long: Boolean) {
        val ic = currentInputConnection ?: return
        when (key.action) {
            Key.TYPE ->
                if (english) ic.commitText(key.label, 1)
                else show(Response(Engine.key(session, Engine.CHAR, key.ch.code, long)))
            Key.TEXT -> {
                // digits and punctuation end the word and are inserted as they are
                show(Response(Engine.commit(session)))
                ic.commitText(key.label, 1)
            }
            Key.SPACE -> engineKey(Engine.SPACE) { ic.commitText(" ", 1) }
            Key.BACKSPACE -> engineKey(Engine.BACKSPACE) {
                if (ic.getSelectedText(0).isNullOrEmpty()) ic.deleteSurroundingTextInCodePoints(1, 0) else ic.commitText("", 1)
            }
            Key.ENTER -> engineKey(Engine.ENTER) { enter() }
            Key.LAYOUT -> if (!plain) {
                show(Response(Engine.commit(session)))
                prefs.layout = (prefs.layout + 1) % Prefs.LAYOUTS.size // the listener applies it
            }
        }
    }

    /** Sends a key to the engine; `otherwise` does what the key normally does when the engine passes it on. */
    private fun engineKey(kind: Int, otherwise: () -> Unit) {
        if (english) return otherwise()
        val r = Response(Engine.key(session, kind, 0, false))
        show(r)
        if (!r.handled) otherwise()
    }

    private fun enter() {
        val ic = currentInputConnection ?: return
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
    override fun onPick(index: Int) {
        show(Response(Engine.pick(session, index)))
        currentInputConnection?.commitText(" ", 1)
    }

    /** Inserts what the engine committed and shows the word being typed underlined, with its suggestions. */
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
        view?.setCandidates(r.candidates, r.selected)
    }
}
