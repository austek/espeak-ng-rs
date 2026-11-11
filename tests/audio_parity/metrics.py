"""Audio distance metrics between a reference WAV and a test WAV.

Features are computed on a 10 ms hop: 25 ms Hann frames for the spectrum and
the mel cepstrum, 40 ms frames for the YIN pitch tracker. The two
recordings are aligned with dynamic time warping on the mel cepstrum, and all
frame-wise metrics are averaged along that path.

    mcd       mel-cepstral distortion in dB over cepstral coefficients 1..12.
    lsd       log-spectral distance in dB (RMS over bins, mean over the path).
    f0corr    Pearson correlation of voiced F0 in semitones along the path.
    f0_st_mad median absolute F0 difference in semitones along the path.
    dur_ratio test duration divided by reference duration.
"""
import numpy as np
from scipy.fft import dct, rfft
from scipy.io import wavfile
from scipy.signal import get_window
from scipy.spatial.distance import cdist

HOP_S = 0.010
SPEC_S = 0.025
F0_S = 0.040
NFFT = 1024
N_MEL = 40
N_CEP = 13
FLOOR = 1e-8
F0_MIN, F0_MAX = 60.0, 400.0
YIN_THRESHOLD = 0.15
YIN_VOICED = 0.35
YIN_REFINE = 10
VOICED_RMS = 0.01
MIN_F0_PAIRS = 5


def read_wav(path):
    rate, data = wavfile.read(path)
    if data.ndim > 1:
        data = data[:, 0]
    return rate, data.astype(np.float64) / 32768.0


def _frames(x, size, hop, count):
    padded = np.pad(x, (0, max(0, (count - 1) * hop + size - len(x))))
    index = np.arange(size)[None, :] + hop * np.arange(count)[:, None]
    return padded[index]


def mel_filterbank(rate):
    to_mel = lambda f: 2595.0 * np.log10(1.0 + f / 700.0)
    to_hz = lambda m: 700.0 * (10.0 ** (m / 2595.0) - 1.0)
    edges = to_hz(np.linspace(0.0, to_mel(rate / 2.0), N_MEL + 2))
    bins = np.fft.rfftfreq(NFFT, 1.0 / rate)
    bank = np.zeros((N_MEL, len(bins)))
    for m in range(N_MEL):
        lo, mid, hi = edges[m], edges[m + 1], edges[m + 2]
        bank[m] = np.maximum(0.0, np.minimum((bins - lo) / (mid - lo), (hi - bins) / (hi - mid)))
    return bank


def _yin_lags(frames, lo, hi):
    size = frames.shape[1]
    acf = np.fft.irfft(np.abs(rfft(frames, 2 * size, axis=1)) ** 2, 2 * size, axis=1)
    cum = np.cumsum(frames ** 2, axis=1)
    lags = np.arange(1, hi + 1)
    diff = cum[:, size - 1 - lags] + (cum[:, [size - 1]] - cum[:, lags - 1]) - 2.0 * acf[:, 1:hi + 1]
    norm = diff * lags / np.maximum(np.cumsum(diff, axis=1), 1e-12)
    cand = norm[:, lo - 1:]
    below = cand < YIN_THRESHOLD
    first = np.where(below.any(axis=1), below.argmax(axis=1), cand.argmin(axis=1))
    span = np.minimum(first[:, None] + np.arange(YIN_REFINE)[None, :], cand.shape[1] - 1)
    best = first + np.take_along_axis(cand, span, axis=1).argmin(axis=1)
    depth = np.take_along_axis(cand, best[:, None], axis=1)[:, 0]
    return best + lo, depth


def _f0(x, rate, hop, count):
    size = round(rate * F0_S)
    frames = _frames(x, size, hop, count)
    frames = frames - frames.mean(axis=1, keepdims=True)
    rms = np.sqrt((frames ** 2).mean(axis=1))
    lag, depth = _yin_lags(frames, max(2, int(rate / F0_MAX)), int(rate / F0_MIN))
    voiced = (depth < YIN_VOICED) & (rms > VOICED_RMS)
    return np.where(voiced, rate / lag, 0.0)


def features(path):
    rate, x = read_wav(path)
    hop = round(rate * HOP_S)
    count = len(x) // hop + 1
    size = round(rate * SPEC_S)
    window = get_window("hann", size)
    spectrum = np.abs(rfft(_frames(x, size, hop, count) * window, NFFT, axis=1)) ** 2
    spectrum = spectrum / (window.sum() / 2.0) ** 2
    mel = spectrum @ mel_filterbank(rate).T
    cepstrum = dct(np.log(mel + FLOOR), type=2, norm="ortho", axis=1)[:, 1:N_CEP]
    return {
        "dur": len(x) / rate,
        "mfcc": cepstrum,
        "logspec": 10.0 * np.log10(spectrum + FLOOR),
        "f0": _f0(x, rate, hop, count),
    }


def dtw_path(a, b):
    cost = cdist(a, b)
    n, m = cost.shape
    acc = np.empty((n, m))
    acc[0] = np.cumsum(cost[0])
    for i in range(1, n):
        prev = acc[i - 1]
        above = np.minimum(prev, np.concatenate(([np.inf], prev[:-1])))
        run = np.cumsum(cost[i])
        before = np.concatenate(([0.0], run[:-1]))
        acc[i] = run + np.minimum.accumulate(above - before)
    i, j = n - 1, m - 1
    path = [(i, j)]
    while i > 0 or j > 0:
        if i == 0:
            j -= 1
        elif j == 0:
            i -= 1
        else:
            step = int(np.argmin((acc[i - 1, j - 1], acc[i - 1, j], acc[i, j - 1])))
            i, j = ((i - 1, j - 1), (i - 1, j), (i, j - 1))[step]
        path.append((i, j))
    path.reverse()
    return np.array(path)


def _f0_stats(f_ref, f_test):
    both = (f_ref > 0) & (f_test > 0)
    if both.sum() < MIN_F0_PAIRS:
        return float("nan"), float("nan")
    s_ref, s_test = 12 * np.log2(f_ref[both]), 12 * np.log2(f_test[both])
    mad = float(np.median(np.abs(s_ref - s_test)))
    if np.allclose(s_ref, s_test):
        return 1.0, mad
    if s_ref.std() == 0 or s_test.std() == 0:
        return float("nan"), mad
    return float(np.corrcoef(s_ref, s_test)[0, 1]), mad


def compare(ref, test):
    path = dtw_path(ref["mfcc"], test["mfcc"])
    pi, pj = path[:, 0], path[:, 1]
    diff = ref["mfcc"][pi] - test["mfcc"][pj]
    mcd = (10.0 / np.log(10.0)) * np.sqrt(2.0 * (diff ** 2).sum(axis=1)).mean()
    lsd = np.sqrt(((ref["logspec"][pi] - test["logspec"][pj]) ** 2).mean(axis=1)).mean()
    f0corr, f0_mad = _f0_stats(ref["f0"][pi], test["f0"][pj])
    ratio = test["dur"] / ref["dur"] if ref["dur"] > 0 else float("nan")
    return {"dur_ratio": ratio, "mcd": float(mcd), "lsd": float(lsd), "f0corr": f0corr, "f0_st_mad": f0_mad}
