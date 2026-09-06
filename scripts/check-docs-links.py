#!/usr/bin/env python3
"""Check local page, asset, and fragment links in the built VitePress site."""

from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import unquote, urljoin, urlsplit

DIST = Path(__file__).resolve().parents[1] / "docs/.vitepress/dist"


class Page(HTMLParser):
    def __init__(self, path):
        super().__init__(convert_charrefs=True)
        self.ids = set()
        self.links = []
        self.feed(path.read_text())

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.ids.add(attrs["id"])
        if tag in {"a", "link"} and attrs.get("href"):
            self.links.append(attrs["href"])
        if tag in {"img", "script", "source"} and attrs.get("src"):
            self.links.append(attrs["src"])


def main():
    pages = {p.relative_to(DIST).as_posix(): Page(p) for p in DIST.rglob("*.html")}
    if not pages:
        raise SystemExit("No built pages found. Build the documentation first.")

    errors = set()
    checked = 0
    for name, page in pages.items():
        for href in page.links:
            parsed = urlsplit(href)
            if (parsed.scheme or href.startswith("//")) and parsed.hostname != "hk.jdx.dev":
                continue
            url = urlsplit(urljoin("/" + name, href))
            relative = unquote(url.path).lstrip("/")
            candidates = [relative]
            if relative.endswith("/") or not relative:
                candidates = [relative + "index.html"]
            elif not Path(relative).suffix:
                candidates.extend([relative + ".html", relative + "/index.html"])
            target = next((c for c in candidates if (DIST / c).is_file()), None)
            checked += 1
            if target is None:
                errors.add(f"{name}: missing target {href}")
            elif url.fragment and target in pages:
                fragment = unquote(url.fragment)
                if fragment not in pages[target].ids:
                    errors.add(f"{name}: missing fragment {href}")

    if errors:
        raise SystemExit("\n".join(sorted(errors)))
    print(f"Checked {checked} local links and assets across {len(pages)} pages")


if __name__ == "__main__":
    main()
