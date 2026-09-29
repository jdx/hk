// Publishing a finished render: its video, then its poster, each renamed
// over the file the site deploys. The poster goes up last. If it cannot,
// the video this render put up comes down again, so a page never pairs it
// with an older poster; the shanty's page, with no video, plays the song
// alone. A video that is no longer this render's, because another render
// has published over it since, is left for that render.

import { renameSync, rmSync, statSync } from "node:fs";

/** Rename `video.from` to `video.to`, then `poster.from` to `poster.to`. */
export function publishRender({ video, poster }) {
  renameSync(video.from, video.to);
  const ours = statSync(video.to);
  try {
    renameSync(poster.from, poster.to);
  } catch (err) {
    const now = statSync(video.to, { throwIfNoEntry: false });
    if (now && now.dev === ours.dev && now.ino === ours.ino) rmSync(video.to, { force: true });
    throw err;
  }
}
