/* One program, two link targets: the C libespeak-ng and this crate's
 * libespeak-ng.so.  Each scenario prints a transcript; the transcripts must
 * match byte for byte. */
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>
#include <wchar.h>
#include <espeak-ng/speak_lib.h>
#include <espeak-ng/espeak_ng.h>

static uint64_t fnv = 1469598103934665603ULL;
static long total_samples = 0;
static int dump_events = 0;
static int call_no = 0;

static void feed(const short *wav, int n) {
	const unsigned char *p = (const unsigned char *)wav;
	for (int i = 0; i < n * 2; i++) {
		fnv ^= p[i];
		fnv *= 1099511628211ULL;
	}
	total_samples += n;
}

static void print_event(const espeak_EVENT *e) {
	printf("  ev type=%d uid=%u pos=%d len=%d apos=%d", (int)e->type,
	       e->unique_identifier, e->text_position, e->length, e->audio_position);
	switch (e->type) {
	case espeakEVENT_WORD:
	case espeakEVENT_SENTENCE:
		printf(" num=%d", e->id.number);
		break;
	case espeakEVENT_MARK:
	case espeakEVENT_PLAY:
		printf(" name=%s", e->id.name ? e->id.name : "(null)");
		break;
	case espeakEVENT_PHONEME:
		printf(" ph=%.8s", e->id.string);
		break;
	default:
		break;
	}
	printf(" ud=%s\n", e->user_data ? (const char *)e->user_data : "(null)");
}

static int on_synth(short *wav, int n, espeak_EVENT *events) {
	if (dump_events) {
		printf("cb %d samples=%d wav=%s\n", call_no, wav ? n : -1, wav ? "data" : "null");
		for (espeak_EVENT *e = events; e && e->type != espeakEVENT_LIST_TERMINATED; e++)
			print_event(e);
	}
	call_no++;
	if (wav)
		feed(wav, n);
	return 0;
}

static int on_uri(int type, const char *uri, const char *base) {
	printf("uri type=%d uri=%s base=%s\n", type, uri, base ? base : "(null)");
	return 0;
}

static int init(int mode, int options) {
	int rate = espeak_Initialize((espeak_AUDIO_OUTPUT)mode, 0, NULL, options);
	if (rate < 0) {
		printf("init failed %d\n", rate);
		exit(2);
	}
	espeak_SetSynthCallback(on_synth);
	return rate;
}

static int scenario_phon(int argc, char **argv) {
	/* phon <voice> <phonememode> <text> */
	init(AUDIO_OUTPUT_SYNCHRONOUS, 0);
	printf("setvoice=%d\n", (int)espeak_SetVoiceByName(argv[0]));
	int mode = atoi(argv[1]);
	const void *p = argv[2];
	int guard = 0;
	while (p && guard++ < 200) {
		int term = -1;
		const char *s = espeak_TextToPhonemesWithTerminator(&p, espeakCHARS_UTF8, mode, &term);
		printf("[%s] term=%x\n", s ? s : "(null)", term);
	}
	espeak_Terminate();
	return 0;
}

static int scenario_batch(int argc, char **argv) {
	/* batch <cases.tsv>: lines "voice<TAB>text"; prints mnemonics then IPA per clause */
	FILE *f = fopen(argv[0], "r");
	char line[2000];
	if (!f)
		return 3;
	init(AUDIO_OUTPUT_SYNCHRONOUS, 0);
	while (fgets(line, sizeof line, f)) {
		line[strcspn(line, "\r\n")] = 0;
		char *tab = strchr(line, '\t');
		if (!tab)
			continue;
		*tab = 0;
		printf("## %s | %s\n", line, tab + 1);
		printf("setvoice=%d\n", (int)espeak_SetVoiceByName(line));
		static const int modes[] = {0, 2, 2 | 0x80 | (0x35f << 8), 0 | ('_' << 8)};
		for (size_t m = 0; m < sizeof modes / sizeof modes[0]; m++) {
			const void *p = tab + 1;
			int guard = 0;
			while (p && guard++ < 100) {
				int term = -1;
				const char *s = espeak_TextToPhonemesWithTerminator(&p, espeakCHARS_UTF8, modes[m], &term);
				printf("mode=%x [%s] term=%x\n", modes[m], s ? s : "(null)", term);
			}
		}
	}
	fclose(f);
	espeak_Terminate();
	return 0;
}

static int scenario_wav(int argc, char **argv) {
	/* wav <voice> <flags> <text> [outfile] */
	int rate = init(AUDIO_OUTPUT_SYNCHRONOUS, 0);
	espeak_SetVoiceByName(argv[0]);
	unsigned flags = (unsigned)strtoul(argv[1], NULL, 0) | espeakCHARS_UTF8;
	unsigned uid = 0;
	dump_events = 0;
	espeak_ERROR r = espeak_Synth(argv[2], strlen(argv[2]) + 1, 0, POS_CHARACTER, 0, flags, &uid, NULL);
	espeak_Synchronize();
	printf("rate=%d synth=%d samples=%ld fnv=%016llx\n", rate, (int)r, total_samples,
	       (unsigned long long)fnv);
	espeak_Terminate();
	return 0;
}

static int scenario_events(int argc, char **argv) {
	/* events <mode> <voice> <flags> <text> */
	int mode = atoi(argv[0]);
	init(mode, 0);
	espeak_SetVoiceByName(argv[1]);
	unsigned flags = (unsigned)strtoul(argv[2], NULL, 0) | espeakCHARS_UTF8;
	unsigned uid = 0;
	dump_events = 1;
	espeak_SetUriCallback(on_uri);
	espeak_ERROR r = espeak_Synth(argv[3], strlen(argv[3]) + 1, 0, POS_CHARACTER, 0, flags, &uid, "UD");
	espeak_Synchronize();
	printf("synth=%d uid=%u samples=%ld fnv=%016llx\n", (int)r, uid, total_samples,
	       (unsigned long long)fnv);
	espeak_Terminate();
	return 0;
}

static int scenario_voices(int argc, char **argv) {
	init(AUDIO_OUTPUT_SYNCHRONOUS, 0);
	const espeak_VOICE **v = espeak_ListVoices(NULL);
	for (int i = 0; v && v[i]; i++) {
		printf("%s|%s|", v[i]->name, v[i]->identifier);
		const char *l = v[i]->languages;
		while (*l) {
			printf("%d:%s,", (unsigned char)l[0], l + 1);
			l += strlen(l + 1) + 2;
		}
		printf("|%d|%d|%d\n", v[i]->gender, v[i]->age, v[i]->variant);
	}
	espeak_VOICE spec;
	memset(&spec, 0, sizeof spec);
	spec.languages = "de";
	const espeak_VOICE **d = espeak_ListVoices(&spec);
	for (int i = 0; d && d[i] && i < 5; i++)
		if (strncmp(d[i]->identifier, "mb/", 3) != 0) /* MBROLA files are optional data */
			printf("de-filter %d %s\n", i, d[i]->name);
	printf("setprops=%d\n", (int)espeak_SetVoiceByProperties(&spec));
	espeak_VOICE *cur = espeak_GetCurrentVoice();
	printf("current=%s|%s\n", cur ? cur->name : "(null)", cur ? cur->identifier : "(null)");
	printf("byname-missing=%d\n", (int)espeak_SetVoiceByName("no-such-voice-xyz"));
	printf("byname-fr=%d\n", (int)espeak_SetVoiceByName("fr"));
	cur = espeak_GetCurrentVoice();
	printf("current=%s\n", cur ? cur->name : "(null)");
	espeak_Terminate();
	return 0;
}

static int scenario_params(int argc, char **argv) {
	init(AUDIO_OUTPUT_SYNCHRONOUS, 0);
	static const int ids[] = {espeakRATE, espeakVOLUME, espeakPITCH, espeakRANGE,
	                          espeakPUNCTUATION, espeakCAPITALS, espeakWORDGAP,
	                          espeakSSML_BREAK_MUL};
	for (size_t i = 0; i < sizeof ids / sizeof ids[0]; i++) {
		int id = ids[i];
		int def = espeak_GetParameter((espeak_PARAMETER)id, 0);
		int cur = espeak_GetParameter((espeak_PARAMETER)id, 1);
		printf("param %d default=%d current=%d", id, def, cur);
		static const int values[] = {0, 1, 60, 175, 450, 1000, -5};
		for (size_t j = 0; j < sizeof values / sizeof values[0]; j++) {
			int e = espeak_SetParameter((espeak_PARAMETER)id, values[j], 0);
			printf(" set(%d)=%d->%d", values[j], e, espeak_GetParameter((espeak_PARAMETER)id, 1));
		}
		int e = espeak_SetParameter((espeak_PARAMETER)id, 7, 1);
		printf(" rel(7)=%d->%d", e, espeak_GetParameter((espeak_PARAMETER)id, 1));
		printf(" default-after=%d\n", espeak_GetParameter((espeak_PARAMETER)id, 0));
	}
	printf("samplerate=%d\n", espeak_ng_GetSampleRate());
	const char *path = NULL;
	const char *ver = espeak_Info(&path);
	printf("info-nonnull=%d path-nonnull=%d\n", ver != NULL, path != NULL);
	printf("cancel=%d sync=%d playing=%d\n", (int)espeak_Cancel(), (int)espeak_Synchronize(),
	       espeak_IsPlaying());
	espeak_Terminate();
	return 0;
}

static int scenario_ngapi(int argc, char **argv) {
	espeak_ng_InitializePath(NULL);
	printf("init=%x\n", espeak_ng_Initialize(NULL));
	printf("out=%x\n", espeak_ng_InitializeOutput(ENOUTPUT_MODE_SYNCHRONOUS, 0, NULL));
	espeak_SetSynthCallback(on_synth);
	printf("voice=%x\n", espeak_ng_SetVoiceByName("de"));
	printf("missing=%x\n", espeak_ng_SetVoiceByName("nonexistent-voice"));
	printf("rate=%d\n", espeak_ng_GetSampleRate());
	printf("param=%x\n", espeak_ng_SetParameter(espeakRATE, 200, 0));
	printf("rate=%d\n", espeak_GetParameter(espeakRATE, 1));
	unsigned uid = 7;
	printf("synth=%x\n", espeak_ng_Synthesize("Guten Tag.", 11, 0, POS_CHARACTER, 0,
	                                          espeakCHARS_UTF8 | espeakENDPAUSE, &uid, NULL));
	printf("uid=%u samples=%ld fnv=%016llx\n", uid, total_samples, (unsigned long long)fnv);
	static const espeak_ng_STATUS codes[] = {ENS_OK, ENS_VOICE_NOT_FOUND, ENS_NOT_INITIALIZED,
	                                         ENS_COMPILE_ERROR, ENS_AUDIO_ERROR};
	for (size_t i = 0; i < sizeof codes / sizeof codes[0]; i++) {
		char buf[200];
		espeak_ng_GetStatusCodeMessage(codes[i], buf, sizeof buf);
		printf("msg %x=%s\n", codes[i], buf);
	}
	printf("cancel=%x sync=%x\n", espeak_ng_Cancel(), espeak_ng_Synchronize());
	printf("char=%d\n", (int)espeak_Char(L'A'));
	printf("key=%d\n", (int)espeak_Key("a"));
	printf("terminate=%x\n", espeak_ng_Terminate());
	return 0;
}

static int scenario_errors(int argc, char **argv) {
	/* behaviours around bad input; no crash, same codes */
	printf("synth-before-init=%d\n",
	       (int)espeak_Synth("x", 2, 0, POS_CHARACTER, 0, espeakCHARS_UTF8, NULL, NULL));
	init(AUDIO_OUTPUT_SYNCHRONOUS, 0);
	printf("synth-null=%d\n",
	       (int)espeak_Synth(NULL, 0, 0, POS_CHARACTER, 0, espeakCHARS_UTF8, NULL, NULL));
	printf("synth-empty=%d\n",
	       (int)espeak_Synth("", 1, 0, POS_CHARACTER, 0, espeakCHARS_UTF8, NULL, NULL));
	printf("byname-null=%d\n", (int)espeak_SetVoiceByName(NULL));
	printf("props-null=%d\n", (int)espeak_SetVoiceByProperties(NULL));
	printf("punct-null=%d\n", (int)espeak_SetPunctuationList(NULL));
	printf("bad-param=%d\n", (int)espeak_SetParameter((espeak_PARAMETER)99, 1, 0));
	printf("bad-get=%d\n", espeak_GetParameter((espeak_PARAMETER)-3, 1));
	printf("terminate=%d\n", (int)espeak_Terminate());
	printf("after-terminate=%d\n",
	       (int)espeak_Synth("x", 2, 0, POS_CHARACTER, 0, espeakCHARS_UTF8, NULL, NULL));
	return 0;
	espeak_Terminate();
	return 0;
}

int main(int argc, char **argv) {
	if (argc < 2)
		return 1;
	const char *s = argv[1];
	if (!strcmp(s, "phon")) return scenario_phon(argc - 2, argv + 2);
	if (!strcmp(s, "batch")) return scenario_batch(argc - 2, argv + 2);
	if (!strcmp(s, "ngapi")) return scenario_ngapi(argc - 2, argv + 2);
	if (!strcmp(s, "wav")) return scenario_wav(argc - 2, argv + 2);
	if (!strcmp(s, "events")) return scenario_events(argc - 2, argv + 2);
	if (!strcmp(s, "voices")) return scenario_voices(argc - 2, argv + 2);
	if (!strcmp(s, "params")) return scenario_params(argc - 2, argv + 2);
	if (!strcmp(s, "errors")) return scenario_errors(argc - 2, argv + 2);
	return 1;
}
