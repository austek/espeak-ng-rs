# espeak-ng-capi

C ABI for the pure Rust eSpeak NG port: `libespeak-ng.so.1` / `libespeak-ng.a`
with the symbols of upstream's `speak_lib.h` and `espeak_ng.h`.

```sh
cargo build --release -p espeak-ng-capi
capi/scripts/package.sh /usr/local          # lib, headers, espeak-ng.pc
```

Cargo rejects hyphens in library names, so the build produces
`libespeak_ng_capi.{so,a}`; `package.sh` installs them under the C names
(SONAME `libespeak-ng.so.1`).  Headers are hand-written copies of the C
declarations (cbindgen cannot reproduce upstream's typedef and union layout).

## Differences from the C library

- `AUDIO_OUTPUT_PLAYBACK` / `AUDIO_OUTPUT_SYNCH_PLAYBACK` fail:
  `espeak_Initialize` returns -1 and `espeak_ng_InitializeOutput` returns
  `ENS_AUDIO_ERROR`.  Follow-up: an audio backend.
- `AUDIO_OUTPUT_RETRIEVAL` runs synthesis inside `espeak_Synth` on the calling
  thread; the callback fires before the call returns.  `espeak_Synchronize`
  and `espeak_IsPlaying` are therefore trivial.  Unique identifiers and the
  `MSG_TERMINATED` event follow C's retrieval mode.
- Initialization never calls `exit`; a missing data directory returns -1.
- Events: `SAMPLERATE`, `SENTENCE`, `WORD`, `MARK`, `END` and the buffer
  layout (60 ms buffers, one flush per clause) follow C.  Word positions come
  from the engine's scan of the input, so they drift where a token expands to
  several words, and `audio_position` can differ from C by a few ms.  `PLAY`
  and `PHONEME` events are not produced and `espeak_SetUriCallback` /
  `espeak_SetPhonemeCallback` only store the callback.  In SSML mode word and
  sentence events are not produced.
- `espeak_Synth` honours `POS_CHARACTER` start positions only; `end_position`
  is ignored.  `espeak_Synth_Mark` starts after the named `<mark>`.
- `espeak_SetPhonemeTrace` stores the mode but writes nothing.
- `espeak_TextToPhonemes` translates the remaining text on the first call and
  hands out its clauses one by one; the advanced pointer is an offset into
  the input located by scanning for clause punctuation.
- `espeak_SetParameter` stores values unclamped and returns -1 for the
  parameters C rejects (`espeakPUNCTUATION`, `espeakCAPITALS`, ...), as C does;
  the engine clamps when it uses them.
- The `espeak_ng_Compile*` functions other than `CompileDictionary`,
  `SetOutputHooks`, `SetConstF0` and `SetRandSeed` return `ENS_NOT_SUPPORTED`.
- Invalid pointers are not detected; null pointers return error codes.
