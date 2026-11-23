# Website for saltClient
To get back to the source code of the launcher, press the little
'website' drop down, and then press 'main'.

## Files
- `index.html` and `404.html`: the pages, in the same design as the Mouse 33 site (light + dark,
  floating header, scroll reveals, pinned in-game stage). `styles.css` styles both.
- `site.js`: theme toggle, header, nav highlight, the pinned stage, the FAQ slide and the
  first-visit preloader.
- `accent.js`: the accent picker. It re-tints the page and repaints the accent-coloured
  parts of the in-game screenshots (`img[data-recolor]`) to the chosen colour.
- `favicon.svg`, `apple-touch-icon.png`, `og.png`, `logo.svg`: the "salt" wordmark and icons.
- `brand/`: the logo pipeline. `salt_logo.py` sweeps a flat pen nib along Bezier centrelines
  (helpers in `logo_gen.py`, from Mouse 33) and shears it 44 degrees. Regenerate with
  `python -I brand/make_assets.py` then `python -I brand/inline_logo.py` (copies the path into
  the pages).
