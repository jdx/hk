import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const configDir = dirname(fileURLToPath(import.meta.url));
const videoPath = resolve(configDir, "../public/bound-for-the-main.mp4");
const posterPath = resolve(configDir, "../public/bound-for-the-main-poster.jpg");
const songPath = resolve(configDir, "../public/bound-for-the-main.mp3");

export interface ShantyFiles {
  /** Site-relative URL of the song's MP3, versioned by its bytes. */
  song: string;
  /**
   * The music video rendered by `mise run docs:shanty`, which the docs
   * deploy runs before building, or null when this build has none.
   */
  video: { src: string; poster: string } | null;
}

// Versioned so browsers and link previews that cached an earlier file fetch
// the new one.
const version = (file: Buffer) => createHash("sha256").update(file).digest("hex").slice(0, 12);

/** The shanty's files, as showreel.data.ts finds the showreel's. */
export function shantyFiles(): ShantyFiles {
  const video =
    existsSync(videoPath) && existsSync(posterPath)
      ? {
          src: `/bound-for-the-main.mp4?v=${version(readFileSync(videoPath))}`,
          poster: `/bound-for-the-main-poster.jpg?v=${version(readFileSync(posterPath))}`,
        }
      : null;
  return { song: `/bound-for-the-main.mp3?v=${version(readFileSync(songPath))}`, video };
}

export declare const data: ShantyFiles;

export default {
  // Pick up a render made while the dev server is running.
  watch: [videoPath, posterPath, songPath],
  load: shantyFiles,
};
