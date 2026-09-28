// The shanty, "Bound for the Main" (docs/public/bound-for-the-main.mp3): its
// clock and its words. The showreel's score is written to its timeline; this
// song is a recording, so its clock is measured from it instead, and the
// music video (video.ts) places everything on these numbers. Nothing here
// draws or imports, so the page and the tests can read it on their own.
//
// The beats are three beat trackers (madmom's RNN downbeat tracker,
// beat_this, and librosa) averaged beat by beat and smoothed over seven
// beats. The song is in 4/4 at about 122 BPM, drifting between about 120 in
// the a cappella opening and 124. From about 140 s to 165 s (the end of verse
// 5 and all of verse 6) the band drops to held chords and the trackers lose
// the beat, so those beats are spaced evenly between the two bar lines
// either side, which is the tempo around them. The words were force-aligned
// to the Demucs vocal stem with stable-ts (Whisper large-v3-turbo) and the
// starts of lines and answers checked against its spectrogram; five words
// the aligner ran together with the next (four of them a line's first) were
// set by hand from it. Times are seconds from the start of the recording,
// which is also the video's clock.

/** The recording. */
export const SONG = {
  title: "Bound for the Main",
  /** Site-relative URL of the MP3, which the video renderer also muxes in. */
  src: "/bound-for-the-main.mp3",
  /** Its length in seconds, from its decoded samples. */
  duration: 211.573,
} as const;

/**
 * Every beat, seconds. Two pickup beats (boots on the deck) come before the
 * first downbeat, BEATS[FIRST_BAR]; every fourth beat from there is a bar
 * line.
 */
export const BEATS: readonly number[] = [
  0.516, 1.011, 1.503, 1.995, 2.488, 2.981, 3.474, 3.968, 4.463, 4.958, 5.453, 5.950,
  6.446, 6.943, 7.440, 7.940, 8.439, 8.938, 9.437, 9.937, 10.436, 10.937, 11.438, 11.939,
  12.438, 12.935, 13.430, 13.924, 14.419, 14.913, 15.407, 15.903, 16.401, 16.903, 17.405, 17.903,
  18.396, 18.892, 19.385, 19.877, 20.364, 20.851, 21.341, 21.840, 22.334, 22.830, 23.330, 23.832,
  24.331, 24.832, 25.331, 25.833, 26.332, 26.829, 27.326, 27.827, 28.326, 28.824, 29.318, 29.812,
  30.305, 30.798, 31.287, 31.775, 32.261, 32.750, 33.239, 33.729, 34.219, 34.709, 35.198, 35.687,
  36.177, 36.665, 37.154, 37.642, 38.131, 38.620, 39.110, 39.597, 40.086, 40.576, 41.065, 41.554,
  42.042, 42.532, 43.021, 43.508, 43.993, 44.478, 44.960, 45.442, 45.921, 46.402, 46.885, 47.368,
  47.851, 48.335, 48.819, 49.305, 49.790, 50.273, 50.754, 51.235, 51.717, 52.198, 52.679, 53.159,
  53.640, 54.124, 54.605, 55.087, 55.568, 56.048, 56.530, 57.014, 57.496, 57.979, 58.460, 58.942,
  59.425, 59.907, 60.390, 60.876, 61.360, 61.846, 62.336, 62.823, 63.312, 63.800, 64.286, 64.774,
  65.264, 65.751, 66.238, 66.725, 67.211, 67.698, 68.186, 68.670, 69.154, 69.637, 70.121, 70.606,
  71.091, 71.574, 72.057, 72.540, 73.026, 73.512, 73.993, 74.477, 74.963, 75.450, 75.940, 76.426,
  76.913, 77.402, 77.890, 78.376, 78.862, 79.348, 79.834, 80.321, 80.809, 81.297, 81.785, 82.271,
  82.757, 83.245, 83.731, 84.217, 84.703, 85.190, 85.678, 86.165, 86.652, 87.140, 87.628, 88.114,
  88.601, 89.087, 89.573, 90.058, 90.543, 91.028, 91.512, 91.996, 92.482, 92.968, 93.456, 93.942,
  94.430, 94.918, 95.406, 95.892, 96.380, 96.866, 97.352, 97.836, 98.320, 98.805, 99.291, 99.776,
  100.262, 100.748, 101.234, 101.721, 102.205, 102.690, 103.175, 103.660, 104.146, 104.632, 105.116, 105.603,
  106.091, 106.578, 107.065, 107.550, 108.035, 108.524, 109.010, 109.497, 109.982, 110.468, 110.955, 111.442,
  111.929, 112.417, 112.903, 113.389, 113.876, 114.363, 114.849, 115.335, 115.820, 116.307, 116.793, 117.278,
  117.764, 118.250, 118.737, 119.223, 119.708, 120.196, 120.682, 121.168, 121.651, 122.134, 122.617, 123.099,
  123.578, 124.061, 124.543, 125.029, 125.516, 126.003, 126.493, 126.983, 127.471, 127.957, 128.445, 128.932,
  129.420, 129.906, 130.395, 130.882, 131.371, 131.856, 132.345, 132.831, 133.318, 133.803, 134.289, 134.775,
  135.263, 135.748, 136.234, 136.725, 137.216, 137.711, 138.205, 138.698, 139.189, 139.680, 140.162, 140.650,
  141.137, 141.625, 142.113, 142.601, 143.089, 143.577, 144.064, 144.552, 145.040, 145.528, 146.016, 146.504,
  146.991, 147.479, 147.967, 148.455, 148.943, 149.431, 149.918, 150.406, 150.894, 151.382, 151.870, 152.358,
  152.845, 153.333, 153.821, 154.309, 154.797, 155.285, 155.773, 156.260, 156.748, 157.236, 157.724, 158.212,
  158.700, 159.187, 159.675, 160.163, 160.651, 161.139, 161.627, 162.114, 162.602, 163.090, 163.578, 164.066,
  164.554, 165.041, 165.529, 166.021, 166.507, 166.994, 167.478, 167.965, 168.454, 168.942, 169.429, 169.915,
  170.402, 170.892, 171.377, 171.864, 172.349, 172.837, 173.325, 173.812, 174.299, 174.787, 175.272, 175.759,
  176.245, 176.730, 177.217, 177.702, 178.188, 178.676, 179.161, 179.647, 180.131, 180.614, 181.098, 181.582,
  182.068, 182.554, 183.042, 183.530, 184.019, 184.507, 184.993, 185.479, 185.962, 186.444, 186.931, 187.420,
  187.914, 188.408, 188.899, 189.393, 189.889, 190.380, 190.868, 191.351, 191.836, 192.323, 192.809, 193.292,
  193.777, 194.262, 194.748, 195.234, 195.719, 196.205, 196.690, 197.176, 197.661, 198.147, 198.632, 199.118,
  199.603, 200.089, 200.576, 201.062, 201.547, 202.032, 202.518, 203.004, 203.489, 203.973, 204.457, 204.940,
  205.424, 205.908, 206.392, 206.876, 207.361, 207.847, 208.331, 208.812, 209.295, 209.777, 210.259, 210.740,
  211.225,
];

/** BEATS index of the first downbeat, bar 0. */
export const FIRST_BAR = 2;

/**
 * The beat at `t` seconds, as a fractional BEATS index: 0 on the first
 * beat, 2.5 halfway between the third and the fourth. Before the first beat
 * and after the last it runs on at the nearest beat's tempo.
 */
export function beatAt(t: number): number {
  const n = BEATS.length;
  if (t <= BEATS[0]) return (t - BEATS[0]) / (BEATS[1] - BEATS[0]);
  if (t >= BEATS[n - 1]) return n - 1 + (t - BEATS[n - 1]) / (BEATS[n - 1] - BEATS[n - 2]);
  let lo = 0;
  let hi = n - 1;
  while (hi - lo > 1) {
    const mid = (lo + hi) >> 1;
    if (BEATS[mid] <= t) lo = mid;
    else hi = mid;
  }
  return lo + (t - BEATS[lo]) / (BEATS[hi] - BEATS[lo]);
}

/** The time of fractional beat `b` (a BEATS index): beatAt's inverse. */
export function timeOfBeat(b: number): number {
  const n = BEATS.length;
  if (b <= 0) return BEATS[0] + b * (BEATS[1] - BEATS[0]);
  if (b >= n - 1) return BEATS[n - 1] + (b - (n - 1)) * (BEATS[n - 1] - BEATS[n - 2]);
  const i = Math.floor(b);
  return BEATS[i] + (b - i) * (BEATS[i + 1] - BEATS[i]);
}

/** The time of bar `n`'s downbeat, or of a point `n` bars (fractional) in. */
export const bar = (n: number): number => timeOfBeat(FIRST_BAR + 4 * n);

/** The song's sections, in order. The first line of each is sung from its start. */
export const SECTIONS = [
  { id: "intro", label: "Intro", start: 0 },
  { id: "verse-1", label: "Verse 1", start: 12.55 },
  { id: "chorus-1", label: "Chorus", start: 28.55 },
  { id: "verse-2", label: "Verse 2", start: 44.3 },
  { id: "verse-3", label: "Verse 3", start: 59.6 },
  { id: "chorus-2", label: "Chorus", start: 75.3 },
  { id: "verse-4", label: "Verse 4", start: 90.45 },
  { id: "chorus-3", label: "Chorus", start: 106.4 },
  { id: "bridge", label: "Bridge", start: 121.55 },
  { id: "verse-5", label: "Verse 5", start: 129.9 },
  { id: "verse-6", label: "Verse 6", start: 146.8 },
  { id: "final-chorus", label: "Final chorus", start: 168.6 },
  { id: "outro", label: "Outro", start: 184.1 },
] as const satisfies readonly { id: string; label: string; start: number }[];

export type SectionId = (typeof SECTIONS)[number]["id"];

/** `h:mm:ss.fff`, a WebVTT timestamp, to the millisecond. */
function vttTime(s: number): string {
  const ms = Math.round(s * 1000);
  const pad = (n: number, w = 2) => String(n).padStart(w, "0");
  return `${pad(Math.floor(ms / 3_600_000))}:${pad(Math.floor(ms / 60_000) % 60)}:${pad(Math.floor(ms / 1000) % 60)}.${pad(ms % 1000, 3)}`;
}

/**
 * The sections as a WebVTT chapters track, one cue per section, identified
 * by its id: docs/public/bound-for-the-main-chapters.vtt, which a test keeps
 * current.
 */
export function chaptersVtt(): string {
  const cues = SECTIONS.map((s, i) => {
    const end = SECTIONS[i + 1]?.start ?? SONG.duration;
    return `${s.id}\n${vttTime(s.start)} --> ${vttTime(end)}\n${s.label}\n`;
  });
  return ["WEBVTT\n", ...cues].join("\n");
}

/** A sung word and when it starts, seconds. The spelling is as sung ("aitch-kay"). */
export type Sung = readonly [word: string, at: number];

/**
 * One line of the lyrics: the shantyman's call and the crew's answer, as
 * the video shows them (backticks mark code; docs/shanty.md prints the same
 * words, see `printed`, with the answer in parentheses), and every word of
 * both as sung. The answer is the last `answer`-many words of `sung`.
 */
export interface Lyric {
  id: string;
  section: SectionId;
  /** Absent where only the crew shouts. */
  call?: string;
  answer?: string;
  sung: readonly Sung[];
}

/** The chorus's first three lines, the same every time they are sung. */
const CHORUS = [
  { call: "Heave away, haul away, hk!", answer: "Heave ho!" },
  { call: "All hands haul at once!", answer: "Haul away!" },
  { call: "For a lock on each file takes the strain," },
] as const;

export const LYRICS: readonly Lyric[] = [
  {
    id: "verse-1.1",
    section: "verse-1",
    call: 'I christened her `"feat: hoist the sails"`,',
    answer: "Heave ho!",
    sung: [["I", 12.8], ["christened", 13.3], ["her", 13.96], ["feat,", 14.16], ["hoist", 14.86], ["the", 15.1], ["sails,", 15.24], ["Heave", 15.88], ["ho!", 16.38]],
  },
  {
    id: "verse-1.2",
    section: "verse-1",
    call: "And I typed `git commit` and hit return,",
    sung: [["And", 16.9], ["I", 17.02], ["typed", 17.22], ["git", 17.62], ["commit", 18.16], ["and", 18.5], ["hit", 18.76], ["return,", 19.02]],
  },
  {
    id: "verse-1.3",
    section: "verse-1",
    call: "Then the hook piped all hands on deck,",
    answer: "Haul away!",
    sung: [["Then", 20.82], ["the", 20.98], ["hook", 21.24], ["piped", 21.48], ["all", 21.98], ["hands", 22.26], ["on", 22.74], ["deck,", 23.14], ["Haul", 23.78], ["away!", 24.1]],
  },
  {
    id: "verse-1.4",
    section: "verse-1",
    call: "And they tumbled up from stem to stern!",
    sung: [["And", 24.8], ["they", 24.92], ["tumbled", 25.1], ["up", 25.64], ["from", 26], ["stem", 26.14], ["to", 26.52], ["stern!", 27.14]],
  },
  {
    id: "chorus-1.1",
    section: "chorus-1",
    ...CHORUS[0],
    sung: [["Heave", 28.78], ["away,", 29.16], ["haul", 29.8], ["away,", 30.12], ["aitch-kay!", 30.8], ["Heave", 31.82], ["ho!", 32.22]],
  },
  {
    id: "chorus-1.2",
    section: "chorus-1",
    ...CHORUS[1],
    sung: [["All", 32.8], ["hands", 33.06], ["haul", 33.6], ["at", 33.92], ["once!", 34.16], ["Haul", 34.72], ["away!", 35]],
  },
  {
    id: "chorus-1.3",
    section: "chorus-1",
    ...CHORUS[2],
    sung: [["For", 36.6], ["a", 36.82], ["lock", 37.08], ["on", 37.36], ["each", 37.62], ["file", 37.94], ["takes", 38.44], ["the", 38.78], ["strain,", 38.96]],
  },
  {
    id: "chorus-1.4",
    section: "chorus-1",
    call: "And we're bound away for the main!",
    sung: [["And", 40.32], ["we're", 40.7], ["bound", 40.98], ["away", 41.38], ["for", 42.36], ["the", 42.68], ["main!", 42.92]],
  },
  {
    id: "verse-2.1",
    section: "verse-2",
    call: "I'd a TODO line I hadn't staged,",
    answer: "Heave ho!",
    sung: [["I'd", 44.56], ["a", 44.76], ["to-do", 44.88], ["line", 45.18], ["I", 45.6], ["hadn't", 45.82], ["staged,", 46.32], ["Heave", 47.34], ["ho!", 47.8]],
  },
  {
    id: "verse-2.2",
    section: "verse-2",
    call: "So hk stashed it safe from the cold,",
    sung: [["So", 48.52], ["aitch-kay", 48.62], ["stashed", 49.4], ["it", 49.5], ["safe", 49.64], ["from", 49.98], ["the", 50.4], ["cold,", 50.58]],
  },
  {
    id: "verse-2.3",
    section: "verse-2",
    call: "For the crew see just the cargo I ship,",
    answer: "Haul away!",
    sung: [["For", 52.12], ["the", 52.3], ["crew", 52.5], ["see", 52.76], ["just", 53], ["the", 53.3], ["cargo", 53.5], ["I", 53.94], ["ship,", 54.32], ["Haul", 55], ["away!", 55.34]],
  },
  {
    id: "verse-2.4",
    section: "verse-2",
    call: "And my line lay snug in the hold.",
    sung: [["And", 56.02], ["my", 56.16], ["line", 56.36], ["lay", 56.8], ["snug", 57.16], ["in", 57.58], ["the", 58.06], ["hold.", 58.36]],
  },
  {
    id: "verse-3.1",
    section: "verse-3",
    call: "Then `shfmt`, `prettier` and `ruff`,",
    answer: "Heave ho!",
    sung: [["Then", 59.85], ["shell-format,", 60.1], ["prettier", 61.26], ["and", 61.64], ["ruff,", 62.18], ["Heave", 62.76], ["ho!", 63.3]],
  },
  {
    id: "verse-3.2",
    section: "verse-3",
    call: "All tailed on at once on the rising tide,",
    sung: [["All", 63.88], ["tailed", 63.88], ["on", 64.32], ["at", 64.5], ["once", 64.64], ["on", 65.04], ["the", 65.36], ["rising", 65.6], ["tide,", 66]],
  },
  {
    id: "verse-3.3",
    section: "verse-3",
    call: "Each hand on a file of their own, made fast,",
    answer: "Haul away!",
    sung: [["Each", 67.62], ["hand", 68.02], ["on", 68.48], ["a", 68.76], ["file", 69.1], ["of", 69.46], ["their", 69.74], ["own,", 69.9], ["made", 70.38], ["fast,", 70.78], ["Haul", 71.5], ["away!", 71.82]],
  },
  {
    id: "verse-3.4",
    section: "verse-3",
    call: "So the three of them hauled side by side!",
    sung: [["So", 72.1], ["the", 72.1], ["three", 72.1], ["of", 72.24], ["them", 72.52], ["hauled", 72.8], ["side", 73.4], ["by", 73.62], ["side!", 73.78]],
  },
  {
    id: "chorus-2.1",
    section: "chorus-2",
    ...CHORUS[0],
    sung: [["Heave", 75.56], ["away,", 75.76], ["haul", 76.4], ["away,", 76.7], ["aitch-kay!", 77.42], ["Heave", 78.36], ["ho!", 78.8]],
  },
  {
    id: "chorus-2.2",
    section: "chorus-2",
    ...CHORUS[1],
    sung: [["All", 79.68], ["hands", 79.68], ["haul", 80.16], ["at", 80.52], ["once!", 80.74], ["Haul", 81.34], ["away!", 81.6]],
  },
  {
    id: "chorus-2.3",
    section: "chorus-2",
    ...CHORUS[2],
    sung: [["For", 83.2], ["a", 83.44], ["lock", 83.64], ["on", 83.94], ["each", 84.2], ["file", 84.52], ["takes", 84.98], ["the", 85.36], ["strain,", 85.58]],
  },
  {
    id: "chorus-2.4",
    section: "chorus-2",
    call: "And we're bound away for the main!",
    sung: [["And", 87.14], ["we're", 87.28], ["bound", 87.54], ["away", 87.98], ["for", 88.74], ["the", 89.26], ["main!", 89.5]],
  },
  {
    id: "verse-4.1",
    section: "verse-4",
    call: "All the fixes were staged, so I skipped the dance,",
    answer: "Heave ho!",
    sung: [["All", 90.72], ["the", 91.14], ["fixes", 91.4], ["were", 91.8], ["staged,", 92.2], ["so", 92.92], ["I", 93.02], ["skipped", 93.2], ["the", 93.62], ["dance,", 93.72], ["Heave", 94.12], ["ho!", 94.4]],
  },
  {
    id: "verse-4.2",
    section: "verse-4",
    call: "No “fail, `git add`, and commit again”,",
    sung: [["No", 94.88], ["fail,", 95.06], ["git", 95.7], ["add,", 95.8], ["and", 96.24], ["commit", 96.54], ["a-gain,", 96.94]],
  },
  {
    id: "verse-4.3",
    section: "verse-4",
    call: "Then my stash came up like an anchor weighed,",
    answer: "Haul away!",
    sung: [["Then", 98.48], ["my", 98.72], ["stash", 99], ["came", 99.6], ["up", 99.98], ["like", 100.68], ["an", 100.88], ["anchor", 101.1], ["weighed,", 101.62], ["Haul", 102.4], ["away!", 102.82]],
  },
  {
    id: "verse-4.4",
    section: "verse-4",
    call: 'And `"feat: hoist the sails"` made the main!',
    sung: [["And", 102.96], ["feat,", 103.06], ["hoist", 103.6], ["the", 103.84], ["sails,", 104.02], ["made", 104.64], ["the", 104.82], ["main!", 105]],
  },
  {
    id: "chorus-3.1",
    section: "chorus-3",
    ...CHORUS[0],
    sung: [["Heave", 106.68], ["away,", 106.88], ["haul", 107.52], ["away,", 107.8], ["aitch-kay!", 108.56], ["Heave", 109.52], ["ho!", 109.94]],
  },
  {
    id: "chorus-3.2",
    section: "chorus-3",
    ...CHORUS[1],
    sung: [["All", 110.5], ["hands", 110.82], ["haul", 111.32], ["at", 111.64], ["once!", 111.86], ["Haul", 112.44], ["away!", 112.7]],
  },
  {
    id: "chorus-3.3",
    section: "chorus-3",
    ...CHORUS[2],
    sung: [["For", 114.32], ["a", 114.54], ["lock", 114.78], ["on", 115.08], ["each", 115.32], ["file", 115.64], ["takes", 116.08], ["the", 116.44], ["strain,", 116.64]],
  },
  {
    id: "chorus-3.4",
    section: "chorus-3",
    call: "And we're bound away for the main!",
    sung: [["And", 118.18], ["we're", 118.36], ["bound", 118.6], ["away", 119.02], ["for", 119.88], ["the", 120.34], ["main!", 120.58]],
  },
  {
    id: "bridge.1",
    section: "bridge",
    call: "Who'll check the cargo?",
    answer: "`hk check`!",
    sung: [["Who'll", 121.82], ["check", 122.06], ["the", 122.24], ["cargo?", 122.44], ["Aitch-kay", 123.08], ["check!", 123.48]],
  },
  {
    id: "bridge.2",
    section: "bridge",
    call: "Who'll mend the canvas?",
    answer: "`hk fix`!",
    sung: [["Who'll", 123.8], ["mend", 124.04], ["the", 124.22], ["canvas?", 124.42], ["Aitch-kay", 125], ["fix!", 125.36]],
  },
  {
    id: "bridge.3",
    section: "bridge",
    call: "Who'll haul the halyard?",
    answer: "All hands at once!",
    sung: [["Who'll", 125.88], ["haul", 126], ["the", 126.18], ["halyard?", 126.38], ["All", 126.84], ["hands", 126.96], ["at", 127.22], ["once!", 127.4]],
  },
  {
    id: "bridge.4",
    section: "bridge",
    call: "And where are we bound?",
    answer: "Bound for the main!",
    sung: [["And", 127.8], ["where", 127.92], ["are", 128.12], ["we", 128.24], ["bound?", 128.36], ["Bound", 128.66], ["for", 128.94], ["the", 129.16], ["main!", 129.32]],
  },
  {
    id: "verse-5.1",
    section: "verse-5",
    call: "The hook, the helm and the harbor-master,",
    answer: "Heave ho!",
    sung: [["The", 130.14], ["hook,", 130.26], ["the", 130.7], ["helm", 130.78], ["and", 131.14], ["the", 131.3], ["harbor-master,", 131.5], ["Heave", 132.98], ["ho!", 133.32]],
  },
  {
    id: "verse-5.2",
    section: "verse-5",
    call: "All muster the self-same crew,",
    sung: [["All", 133.98], ["muster", 134.18], ["the", 134.8], ["self-same", 135.06], ["crew,", 136]],
  },
  {
    id: "verse-5.3",
    section: "verse-5",
    call: "For they steer by the one set of charts,",
    answer: "Haul away!",
    sung: [["For", 137.66], ["they", 137.76], ["steer", 137.96], ["by", 138.52], ["the", 138.82], ["one", 139.04], ["set", 139.34], ["of", 139.76], ["charts,", 139.94], ["Haul", 140.6], ["away!", 140.96]],
  },
  {
    id: "verse-5.4",
    section: "verse-5",
    call: "In `hk.pkl`, typed and true!",
    sung: [["In", 141.96], ["aitch-kay", 142.16], ["dot", 142.66], ["pickle,", 142.88], ["typed", 143.78], ["and", 144.16], ["true!", 144.38]],
  },
  {
    id: "verse-6.1",
    section: "verse-6",
    call: "Her sister sailed with `deploy.sh`,",
    answer: "Bound for the main?",
    sung: [["Her", 147.04], ["sister", 147.2], ["sailed", 147.68], ["with", 148.32], ["deploy", 148.58], ["dot", 149.08], ["ess-aitch,", 149.6], ["Bound", 150.24], ["for", 150.62], ["the", 150.9], ["main?", 151.04]],
  },
  {
    id: "verse-6.2",
    section: "verse-6",
    call: "And a loose end no fixer could splice,",
    sung: [["And", 152.24], ["a", 152.32], ["loose", 152.5], ["end", 152.82], ["no", 153.1], ["fixer", 153.4], ["could", 154.08], ["splice,", 154.44]],
  },
  {
    id: "verse-6.3",
    section: "verse-6",
    call: "Then `shellcheck` sang out from aloft, “Bug, ho!”",
    answer: "Where away?",
    sung: [["Then", 156.62], ["shell-check", 156.82], ["sang", 157.66], ["out", 158.08], ["from", 158.68], ["aloft,", 158.8], ["Bug,", 159.72], ["ho!", 160.06], ["Where", 160.78], ["away?", 161.12]],
  },
  {
    id: "verse-6.4",
    section: "verse-6",
    call: "“Dead ahead!” And the hook tugged her twice!",
    sung: [["Dead", 162.58], ["ahead!", 162.74], ["And", 163.76], ["the", 163.82], ["hook", 163.98], ["tugged", 164.5], ["her", 164.92], ["twice!", 165.14]],
  },
  {
    id: "verse-6.5",
    section: "verse-6",
    answer: "Heave! Ho!",
    sung: [["Heave!", 167.62], ["Ho!", 168.48]],
  },
  {
    id: "final-chorus.1",
    section: "final-chorus",
    ...CHORUS[0],
    sung: [["Heave", 168.86], ["away,", 169.26], ["haul", 170.04], ["away,", 170.14], ["aitch-kay!", 170.94], ["Heave", 171.82], ["ho!", 172.32]],
  },
  {
    id: "final-chorus.2",
    section: "final-chorus",
    ...CHORUS[1],
    sung: [["All", 172.78], ["hands", 173.16], ["haul", 173.66], ["at", 174.04], ["once!", 174.22], ["Haul", 174.76], ["away!", 175.08]],
  },
  {
    id: "final-chorus.3",
    section: "final-chorus",
    ...CHORUS[2],
    sung: [["For", 176.68], ["a", 176.86], ["lock", 177.16], ["on", 177.42], ["each", 177.68], ["file", 178], ["takes", 178.46], ["the", 178.84], ["strain,", 179.54]],
  },
  {
    id: "final-chorus.4",
    section: "final-chorus",
    call: "But she never made the main!",
    sung: [["But", 180.68], ["she", 180.7], ["never", 180.98], ["made", 181.58], ["the", 182.58], ["main!", 182.94]],
  },
  {
    id: "outro.1",
    section: "outro",
    call: "So all you hands who would sign aboard,",
    answer: "Heave ho!",
    sung: [["So", 184.34], ["all", 184.36], ["you", 184.68], ["hands", 184.86], ["who", 185.28], ["would", 185.6], ["sign", 185.78], ["aboard,", 186.36], ["Heave", 187.38], ["ho!", 187.9]],
  },
  {
    id: "outro.2",
    section: "outro",
    call: "There's but three words left to say:",
    sung: [["There's", 188.5], ["but", 188.64], ["three", 188.82], ["words", 189.28], ["left", 189.8], ["to", 190.42], ["say,", 190.7]],
  },
  {
    id: "outro.3",
    section: "outro",
    call: "`mise use hk`!",
    answer: "`mise use hk`!",
    sung: [["Meez", 193.97], ["use", 194.4], ["aitch-kay!", 194.68], ["Meez", 195.96], ["use", 196.4], ["aitch-kay!", 196.62]],
  },
  {
    id: "outro.4",
    section: "outro",
    call: "And we're bound away for the main!",
    sung: [["And", 198.06], ["we're", 198.3], ["bound", 198.56], ["away", 199], ["for", 200.82], ["the", 201.9], ["main!", 202.72]],
  },
];

/** The copy as displayed, without the backticks. */
export const plain = (text: string): string => text.replaceAll("`", "");

/**
 * The copy as the lyrics page prints it: no backticks, and straight quotes
 * where the video sets curly ones (Space Grotesk draws `"` as a closing
 * quote on both sides).
 */
export const printed = (text: string): string => plain(text).replace(/[“”]/g, '"');

/** The line with id `id`. */
export function lyric(id: string): Lyric {
  const l = LYRICS.find((x) => x.id === id);
  if (!l) throw new Error(`no lyric "${id}"`);
  return l;
}

/** How many of a line's sung words are the crew's answer. */
export const answerLength = (l: Lyric): number => (l.answer ? plain(l.answer).split(" ").length : 0);

/** When the call's first word is sung. */
export const callAt = (l: Lyric): number => l.sung[0][1];

/** When each of the answer's words is sung, in order. */
export const answerAt = (l: Lyric): number[] => l.sung.slice(l.sung.length - answerLength(l)).map(([, t]) => t);

/**
 * When `word` (as sung, punctuation ignored) is sung in line `id`: its
 * `nth` time there (0, the first, by default).
 */
export function sungAt(id: string, word: string, nth = 0): number {
  const bare = (w: string) => w.replace(/[^\p{L}\p{N}'-]/gu, "").toLowerCase();
  const hits = lyric(id).sung.filter(([w]) => bare(w) === bare(word));
  if (hits.length <= nth) throw new Error(`"${word}" is not sung ${nth + 1} time(s) in ${id}`);
  return hits[nth][1];
}
