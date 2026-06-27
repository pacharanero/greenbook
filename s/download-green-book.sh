#!/usr/bin/env bash
# Download canonical Green Book source PDFs into ./pdf/.
#
# Filenames are normalised to:
#   green-book-<YYYY-MM-DD>.pdf
#   green-book-chapter-<N>-<YYYY-MM-DD>.pdf
# where the date is the document's publication/document date. The source URLs
# use several naming schemes, so we rename on the way in.
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
  "green-book-2006-10-30.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20080817104105mp_/http://www.dh.gov.uk/en/Publicationsandstatistics/Publications/PublicationsPolicyAndGuidance/DH_079917?IdcService=GET_FILE&dID=115974&Rendition=Web"
  "green-book-2006-10-30-letter.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20080817104105mp_/http://www.dh.gov.uk/en/Publicationsandstatistics/Publications/PublicationsPolicyAndGuidance/DH_079917?IdcService=GET_FILE&dID=115812&Rendition=Web"
  "green-book-chapter-11-2013-03-20.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20130504233315mp_/https://www.gov.uk/government/uploads/system/uploads/attachment_data/file/147874/Green-Book-Chapter-11.pdf"
  "green-book-chapter-11-2013-05-07.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20130627114320mp_/https://www.gov.uk/government/uploads/system/uploads/attachment_data/file/198714/Chapter_11_The_UK_immunisation_schedule.pdf"
  "green-book-chapter-11-2014-04-30.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20140607041311mp_/https://www.gov.uk/government/uploads/system/uploads/attachment_data/file/307609/2902222_Green_Book_Chapter_11_v2_2_final.pdf"
  "green-book-chapter-11-2016-09-20.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20161124043448mp_/https://www.gov.uk/government/uploads/system/uploads/attachment_data/file/554298/Green_Book_Chapter_11.pdf"
  "green-book-chapter-11-2019-04-15.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20190509202436mp_/https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/795467/Greenbook_chapter_11.pdf"
  "green-book-chapter-11-2019-09-16.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20191003053742mp_/https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/831680/Greenbook_chapter_11_UK_Immunisation_schedule.pdf"
  "green-book-chapter-11-2020-01-02.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20200102213743mp_/https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/855727/Greenbook_chapter_11_UK_Immunisation_schedule.pdf"
  "green-book-chapter-11-2022-03-17.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20220317170925mp_/https://assets.publishing.service.gov.uk/government/uploads/system/uploads/attachment_data/file/1060682/Greenbook-chapter-11-11Mar22.pdf"
  "green-book-chapter-11-2025-06-03.pdf|https://webarchive.nationalarchives.gov.uk/ukgwa/20250603152059mp_/https://assets.publishing.service.gov.uk/media/6839d882e0f10eed80aafb7e/Green_Book_Chapter_11_Routine_Immunisation_05.pdf"
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
