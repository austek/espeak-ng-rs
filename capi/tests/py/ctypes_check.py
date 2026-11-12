"""Drive libespeak-ng through ctypes the way Python users do.

usage: ctypes_check.py <libespeak-ng.so> <fixtures dir>
Expectations come from the fixtures recorded from the reference C library.
"""
import ctypes
import re
import sys
from ctypes import (CFUNCTYPE, POINTER, Structure, Union, c_char, c_char_p, c_int,
                    c_short, c_uint, c_void_p)

lib_path, fixtures = sys.argv[1], sys.argv[2]


class EventId(Union):
    _fields_ = [("number", c_int), ("name", c_char_p), ("string", c_char * 8)]


class Event(Structure):
    _fields_ = [("type", c_int), ("unique_identifier", c_uint), ("text_position", c_int),
                ("length", c_int), ("audio_position", c_int), ("sample", c_int),
                ("user_data", c_void_p), ("id", EventId)]


class Voice(Structure):
    _fields_ = [("name", c_char_p), ("languages", c_char_p), ("identifier", c_char_p),
                ("gender", ctypes.c_ubyte), ("age", ctypes.c_ubyte),
                ("variant", ctypes.c_ubyte), ("xx1", ctypes.c_ubyte),
                ("score", c_int), ("spare", c_void_p)]


CALLBACK = CFUNCTYPE(c_int, POINTER(c_short), c_int, POINTER(Event))
lib = ctypes.CDLL(lib_path)
lib.espeak_Initialize.argtypes = [c_int, c_int, c_char_p, c_int]
lib.espeak_SetVoiceByName.argtypes = [c_char_p]
lib.espeak_TextToPhonemes.restype = c_char_p
lib.espeak_TextToPhonemes.argtypes = [POINTER(c_void_p), c_int, c_int]
lib.espeak_ListVoices.restype = POINTER(POINTER(Voice))
lib.espeak_ListVoices.argtypes = [c_void_p]
lib.espeak_Synth.argtypes = [c_char_p, ctypes.c_size_t, c_uint, c_int, c_uint, c_uint,
                             POINTER(c_uint), c_void_p]
lib.espeak_SetSynthCallback.argtypes = [CALLBACK]
lib.espeak_SetParameter.argtypes = [c_int, c_int, c_int]
lib.espeak_GetParameter.argtypes = [c_int, c_int]


def check(cond, what):
    if not cond:
        print("FAIL:", what)
        sys.exit(1)
    print("ok:", what)


check(lib.espeak_Initialize(2, 0, None, 0) == 22050, "Initialize returns the sample rate")
check(lib.espeak_SetVoiceByName(b"en") == 0, "SetVoiceByName(en)")

text = ctypes.create_string_buffer("Hello world, this is a test.".encode())
ptr = c_void_p(ctypes.addressof(text))
clauses = []
while ptr.value:
    clauses.append(lib.espeak_TextToPhonemes(ctypes.byref(ptr), 1, 2).decode())
check(clauses == ["həlˈəʊ wˈɜːld", "ðɪs ɪz ɐ tˈɛst"], "TextToPhonemes IPA clauses %r" % clauses)

samples = []


@CALLBACK
def on_synth(wav, n, events):
    if wav:
        samples.extend(wav[:n])
    return 0


lib.espeak_SetSynthCallback(on_synth)
body = "Hello world, this is a test.".encode()
uid = c_uint(0)
rc = lib.espeak_Synth(body, len(body) + 1, 0, 1, 0, 0x1001, ctypes.byref(uid), None)
fixture = open(fixtures + "/expected/wav_0.txt").read()
want = int(re.search(r"samples=(\d+)", fixture).group(1))
check(rc == 0 and len(samples) == want, "Synth delivers %d samples (got %d)" % (want, len(samples)))
fnv = 1469598103934665603
for s in samples:
    for b in (s & 0xFF, (s >> 8) & 0xFF):
        fnv = ((fnv ^ b) * 1099511628211) & 0xFFFFFFFFFFFFFFFF
check("%016x" % fnv == re.search(r"fnv=(\w+)", fixture).group(1), "audio matches the C library")

voices = lib.espeak_ListVoices(None)
count = 0
while voices[count]:
    count += 1
listed = open(fixtures + "/expected/voices.txt").read().split("de-filter")[0].count("\n")
check(count == listed, "ListVoices count %d" % count)

check(lib.espeak_GetParameter(1, 0) == 175, "default rate")
check(lib.espeak_SetParameter(1, 200, 0) == 0 and lib.espeak_GetParameter(1, 1) == 200, "rate round trip")
check(lib.espeak_SetParameter(3, 10, 0) == 0 and lib.espeak_GetParameter(3, 1) == 10, "pitch round trip")
check(lib.espeak_Terminate() == 0, "Terminate")
print("PASS")
