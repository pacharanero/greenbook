# greenbook presentation

A [reveal.js](https://revealjs.com/) deck explaining the project's thought chain - from the Green Book as it exists today, through the domain model (schedule, series, doses, products, antigens, product class) and the conformance-vs-coverage distinction, to the up-to-date-for-age status model and where the work goes next. It uses the project's [ubiquitous language](../../spec/ubiquitous-language.md) throughout, aimed at a mixed clinical/technical audience.

## View it

Run `s/present` from the repo root to serve the slides and open them in your browser, or just open `presentation.html` directly - no build step. The slides load reveal.js, the fonts, and Font Awesome from CDNs, so an internet connection is needed the first time. Arrow keys navigate; press `F` for fullscreen, `S` for speaker notes, `Esc` for the slide overview.

## Files

- `presentation.html` - the slides
- `styles.css` - the theme (paper background, forest-green accents, terracotta for edge-cases)
- `edit.js` - a small reveal-aware in-browser text editor (see below)

## Edit the wording in the browser

```sh
node docs/presentation/edit.js
```

This serves the deck at `http://localhost:3456` with every text element made editable. Click any text to edit it, `Esc` to deselect, then **Save**; `Ctrl+C` stops the server. Navigate slides first (arrow keys) to reach the text you want - only the visible slide is editable at a time.

It is a project-local editor rather than the generic revealjs-skill one because reveal.js needs two special accommodations, both handled here:

- **Clean save.** Reveal.js mutates the DOM heavily at runtime (injected backgrounds/controls, `present`/`past`/`future` classes, inline transforms, aria attributes). Serialising that live DOM writes all of it back to the file. `edit.js` instead reads the pristine file, copies in only your edited text, and saves that - so the git diff is just your wording changes. (HTML entities like `&rarr;` are normalised to their characters on the first save; this is idempotent thereafter.)
- **Editing under a scaled transform.** Reveal scales the slides to fit the window with a CSS `transform`, and `contenteditable` misbehaves under that (mis-placed caret, clicks not landing) - especially in Firefox. `edit.js` pins reveal to scale 1 while editing so the caret behaves.

## Export to PDF

Open `presentation.html?print-pdf` in Chrome and print to PDF, or use [decktape](https://github.com/astefanutti/decktape):

```sh
npx decktape reveal "presentation.html?export" greenbook.pdf
```
