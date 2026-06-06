# greenbook presentation

A [reveal.js](https://revealjs.com/) deck explaining the project's thought chain - from the Green Book as it exists today, through the domain model (schedule, series, doses, products, antigens, product class) and the conformance-vs-coverage distinction, to the up-to-date-for-age status model and where the work goes next. It uses the project's [ubiquitous language](../../spec/ubiquitous-language.md) throughout, aimed at a mixed clinical/technical audience.

## View it

Open `presentation.html` in any browser - no build step. The slides load reveal.js, the fonts, and Font Awesome from CDNs, so an internet connection is needed the first time. Arrow keys navigate; press `F` for fullscreen, `S` for speaker notes, `Esc` for the slide overview.

## Files

- `presentation.html` - the slides
- `styles.css` - the theme (paper background, forest-green accents, terracotta for edge-cases)

## Edit the wording in the browser

If the revealjs skill is installed, you can click-to-edit text inline:

```sh
node <skill-path>/scripts/edit-html.js docs/presentation/presentation.html
```

Click any text to edit, `Esc` to deselect, then Save. `Ctrl+C` stops the server.

## Export to PDF

Open `presentation.html?print-pdf` in Chrome and print to PDF, or use [decktape](https://github.com/astefanutti/decktape):

```sh
npx decktape reveal "presentation.html?export" greenbook.pdf
```
