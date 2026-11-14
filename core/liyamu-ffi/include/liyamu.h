/* Liyamu Sinhala input engine — C API. Strings are UTF-8, NUL-terminated.
 * Free every response with liyamu_response_free and every session with liyamu_session_free. */
#ifndef LIYAMU_H
#define LIYAMU_H

#include <stdbool.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

enum {
    LIYAMU_KEY_CHAR = 0,      /* ch = the US-QWERTY character, shift applied */
    LIYAMU_KEY_SPACE = 1,
    LIYAMU_KEY_BACKSPACE = 2,
    LIYAMU_KEY_ENTER = 3,
    LIYAMU_KEY_TAB = 4,
    LIYAMU_KEY_ESCAPE = 5,
    LIYAMU_KEY_OTHER = 6,     /* Home, F-keys ...: commits and passes the key on */
    LIYAMU_KEY_LEFT = 7,      /* arrows move through the candidates while there are some, */
    LIYAMU_KEY_RIGHT = 8,     /* otherwise they are like LIYAMU_KEY_OTHER */
    LIYAMU_KEY_UP = 9,
    LIYAMU_KEY_DOWN = 10,
};

typedef struct LiyamuOptions {
    uint32_t layout;  /* 0 Wijesekara, 1 Easy Singlish, 2 Helakuru phonetic */
    uint32_t output;  /* 0 Unicode, 1 FM, 2 Isi */
    uint32_t order;   /* 0 traditional (kombuva first), 1 logical */
    bool pali;        /* touching letters: Wijesekara hal touches and X types ඞ; clusters are written touching */
    bool suggest;     /* offer word suggestions (candidates) */
    bool repaya;      /* ර් before a consonant as repaya */
    bool conjuncts;   /* Pali pairs (ක්‍ව ත්‍ථ න්‍ද ...) as joined letters */
    bool pali_words;  /* suggest Pali-only words */
} LiyamuOptions;

typedef struct LiyamuResponse {
    bool handled;           /* false: pass the key on to the app, after inserting commit */
    bool lossy;             /* part of the text could not be written in the legacy font */
    char *commit;           /* text to insert, encoded for the output */
    char *preedit;          /* word being composed (encoded); replaces the previous preedit */
    char *preedit_unicode;  /* the same word in Unicode, for the candidate window */
    uint32_t candidate_count;
    char **candidates;      /* Unicode word suggestions */
    uint32_t selected;      /* the highlighted candidate; Space commits it */
} LiyamuResponse;

typedef struct LiyamuSession LiyamuSession;

LiyamuSession *liyamu_session_new(const LiyamuOptions *opts); /* NULL = defaults */
void liyamu_session_free(LiyamuSession *s);
LiyamuResponse *liyamu_session_set_options(LiyamuSession *s, const LiyamuOptions *opts);
LiyamuResponse *liyamu_session_key(LiyamuSession *s, uint32_t kind, uint32_t ch, bool altgr, bool ctrl);
LiyamuResponse *liyamu_session_commit(LiyamuSession *s);
LiyamuResponse *liyamu_session_pick(LiyamuSession *s, uint32_t index); /* commit a candidate */
void liyamu_session_reset(LiyamuSession *s);
/* loads the word lists now instead of at the first key; call on a background thread at start */
void liyamu_warm_up(void);
/* the user's own shortcuts, for all sessions: lines of keys=words; NULL for none */
void liyamu_set_shortcuts(const char *text);
void liyamu_response_free(LiyamuResponse *r);

#ifdef __cplusplus
}
#endif
#endif
