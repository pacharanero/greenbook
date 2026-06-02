#!/usr/bin/env bash
# Download canonical Green Book chapter PDFs from gov.uk into ./pdf/.
#
# Filenames are normalised to:
#   green-book-chapter-<N>-<YYYY-MM-DD>.pdf
# where the date is the document's effective/published date taken from the
# gov.uk page (the source URLs themselves use DD_MM_YY which is harder to sort
# and parse, so we rename on the way in).
#
# Usage:
#   s/download-green-book.sh           # download anything missing
#   s/download-green-book.sh --force   # re-download everything
#
# Add new versions by appending to DOWNLOADS below.

set -euo pipefail

OUT_DIR="${OUT_DIR:-pdf}"

# Registry of known-good PDFs.
# Format: <canonical-filename>|<source-url>
DOWNLOADS=(
  "green-book-chapter-11-2026-03-30.pdf|https://assets.publishing.service.gov.uk/media/69cbb5ef8017673ffec0f313/Green-book-chapter-11-immunisation-schedules_30_03_26.pdf"
)

force=0
if [ "${1:-}" = "--force" ]; then
  force=1
fi

mkdir -p "$OUT_DIR"

filesize() {
  # GNU stat (Linux) and BSD stat (macOS) take different flags.
  stat -c %s "$1" 2>/dev/null || stat -f %z "$1"
}

for entry in "${DOWNLOADS[@]}"; do
  filename="${entry%%|*}"
  url="${entry#*|}"
  target="$OUT_DIR/$filename"

  if [ -e "$target" ] && [ "$force" -eq 0 ]; then
    printf 'skip   %s (already present; pass --force to redownload)\n' "$target"
    continue
  fi

  printf 'fetch  %s\n' "$url"
  printf '  ->   %s\n' "$target"
  curl --fail --location --silent --show-error --output "$target" "$url"
  printf 'ok     %s (%s bytes)\n' "$target" "$(filesize "$target")"
done
