#ifndef ESPEAK_NG_SPEAK_LIB_H
#define ESPEAK_NG_SPEAK_LIB_H

/* Declarations of the eSpeak NG C API (speak_lib.h) implemented by the
 * espeak-ng-capi library.  Names, types and values match upstream's header. */

#include <stdio.h>
#include <stddef.h>

#if defined(_WIN32) || defined(_WIN64)
#ifdef LIBESPEAK_NG_EXPORT
#define ESPEAK_API __declspec(dllexport)
#else
#define ESPEAK_API __declspec(dllimport)
#endif
#else
#define ESPEAK_API
#endif

#define ESPEAK_API_REVISION  13

#define espeakRATE_MINIMUM  80
#define espeakRATE_MAXIMUM  450
#define espeakRATE_NORMAL   175

typedef enum {
  espeakEVENT_LIST_TERMINATED = 0,
  espeakEVENT_WORD = 1,
  espeakEVENT_SENTENCE = 2,
  espeakEVENT_MARK = 3,
  espeakEVENT_PLAY = 4,
  espeakEVENT_END = 5,
  espeakEVENT_MSG_TERMINATED = 6,
  espeakEVENT_PHONEME = 7,
  espeakEVENT_SAMPLERATE = 8
} espeak_EVENT_TYPE;

typedef struct {
	espeak_EVENT_TYPE type;
	unsigned int unique_identifier;
	int text_position;
	int length;
	int audio_position;
	int sample;
	void* user_data;
	union {
		int number;
		const char *name;
		char string[8];
	} id;
} espeak_EVENT;

typedef enum {
	POS_CHARACTER = 1,
	POS_WORD,
	POS_SENTENCE
} espeak_POSITION_TYPE;

typedef enum {
	AUDIO_OUTPUT_PLAYBACK,
	AUDIO_OUTPUT_RETRIEVAL,
	AUDIO_OUTPUT_SYNCHRONOUS,
	AUDIO_OUTPUT_SYNCH_PLAYBACK
} espeak_AUDIO_OUTPUT;

typedef enum {
	EE_OK=0,
	EE_INTERNAL_ERROR=-1,
	EE_BUFFER_FULL=1,
	EE_NOT_FOUND=2
} espeak_ERROR;

#define espeakINITIALIZE_PHONEME_EVENTS 0x0001
#define espeakINITIALIZE_PHONEME_IPA   0x0002
#define espeakINITIALIZE_DONT_EXIT     0x8000

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API int espeak_Initialize(espeak_AUDIO_OUTPUT output, int buflength, const char *path, int options);

typedef int (t_espeak_callback)(short*, int, espeak_EVENT*);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API void espeak_SetSynthCallback(t_espeak_callback* SynthCallback);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API void espeak_SetUriCallback(int (*UriCallback)(int, const char*, const char*));

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API void espeak_SetPhonemeCallback(int (*PhonemeCallback)(const char *));

#define espeakCHARS_AUTO   0
#define espeakCHARS_UTF8   1
#define espeakCHARS_8BIT   2
#define espeakCHARS_WCHAR  3
#define espeakCHARS_16BIT  4

#define espeakSSML        0x10
#define espeakPHONEMES    0x100
#define espeakENDPAUSE    0x1000
#define espeakKEEP_NAMEDATA 0x2000

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_Synth(const void *text,
	size_t size,
	unsigned int position,
	espeak_POSITION_TYPE position_type,
	unsigned int end_position,
	unsigned int flags,
	unsigned int* unique_identifier,
	void* user_data);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_Synth_Mark(const void *text,
	size_t size,
	const char *index_mark,
	unsigned int end_position,
	unsigned int flags,
	unsigned int* unique_identifier,
	void* user_data);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_Key(const char *key_name);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_Char(wchar_t character);

typedef enum {
  espeakSILENCE=0,
  espeakRATE=1,
  espeakVOLUME=2,
  espeakPITCH=3,
  espeakRANGE=4,
  espeakPUNCTUATION=5,
  espeakCAPITALS=6,
  espeakWORDGAP=7,
  espeakOPTIONS=8,
  espeakINTONATION=9,
  espeakSSML_BREAK_MUL=10,

  espeakRESERVED2=11,
  espeakEMPHASIS,
  espeakLINELENGTH,
  espeakVOICETYPE,
  N_SPEECH_PARAM
} espeak_PARAMETER;

typedef enum {
  espeakPUNCT_NONE=0,
  espeakPUNCT_ALL=1,
  espeakPUNCT_SOME=2
} espeak_PUNCT_TYPE;

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_SetParameter(espeak_PARAMETER parameter, int value, int relative);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API int espeak_GetParameter(espeak_PARAMETER parameter, int current);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_SetPunctuationList(const wchar_t *punctlist);

#define espeakPHONEMES_SHOW    0x01
#define espeakPHONEMES_IPA     0x02
#define espeakPHONEMES_TRACE   0x08
#define espeakPHONEMES_MBROLA  0x10
#define espeakPHONEMES_TIE     0x80

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API void espeak_SetPhonemeTrace(int phonememode, FILE *stream);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API const char *espeak_TextToPhonemes(const void **textptr, int textmode, int phonememode);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API const char *espeak_TextToPhonemesWithTerminator(const void **textptr, int textmode, int phonememode, int *terminator);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API void espeak_CompileDictionary(const char *path, FILE *log, int flags);

typedef struct {
	const char *name;
	const char *languages;
	const char *identifier;
	unsigned char gender;
	unsigned char age;
	unsigned char variant;
	unsigned char xx1;
	int score;
	void *spare;
} espeak_VOICE;

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API const espeak_VOICE **espeak_ListVoices(espeak_VOICE *voice_spec);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_SetVoiceByFile(const char *filename);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_SetVoiceByName(const char *name);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_SetVoiceByProperties(espeak_VOICE *voice_spec);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_VOICE *espeak_GetCurrentVoice(void);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_Cancel(void);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API int espeak_IsPlaying(void);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_Synchronize(void);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API espeak_ERROR espeak_Terminate(void);

#ifdef __cplusplus
extern "C"
#endif
ESPEAK_API const char *espeak_Info(const char **path_data);

#endif
