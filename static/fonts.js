/** @typedef {{kind: 'idle'} | {kind: 'loading'} | {kind: 'ready'}} FontUpgradeState */
const root = document.documentElement;
const SAFE_FONT_URL = /^[A-Za-z0-9._/?=-]+$/;

/** @param {string | undefined} raw */
function safeFontUrl(raw) {
  return raw && SAFE_FONT_URL.test(raw) && !raw.includes('..') ? raw : null;
}

const romanSrc = safeFontUrl(root.dataset.vfRoman);
const italicSrc = safeFontUrl(root.dataset.vfItalic);

/** @type {FontUpgradeState} */
let upgradeState = {kind: 'idle'};

function upgradeToVariableFonts() {
  switch (upgradeState.kind) {
    case 'loading':
    case 'ready':
      return;
    case 'idle':
      break;
  }
  if (!romanSrc || typeof FontFace !== 'function' || !('fonts' in document)) return;

  upgradeState = {kind: 'loading'};
  const descriptors = {weight: '300 700', display: 'swap'};
  const faces = [
    new FontFace('Kerberos VF', `url("${romanSrc}")`, {...descriptors, style: 'normal'}),
  ];
  if (italicSrc) {
    faces.push(
      new FontFace('Kerberos VF', `url("${italicSrc}")`, {...descriptors, style: 'italic'}),
    );
  }

  Promise.all(faces.map(face => face.load()))
    .then(loaded => {
      for (const face of loaded) document.fonts.add(face);
      upgradeState = {kind: 'ready'};
      root.dataset.vfReady = '';
    })
    .catch(() => {
      // Boot fonts are already active; stay on Tier 1 silently if offline.
      upgradeState = {kind: 'idle'};
    });
}

function scheduleIdleUpgrade() {
  const start = () => {
    if ('requestIdleCallback' in window) {
      requestIdleCallback(() => upgradeToVariableFonts(), {timeout: 1500});
    } else {
      setTimeout(upgradeToVariableFonts, 120);
    }
  };
  if ('fonts' in document && document.fonts.ready) {
    document.fonts.ready.then(start);
  } else {
    start();
  }
}

scheduleIdleUpgrade();

// Begin loading immediately if the user reaches for an interactive link before idle.
document.addEventListener(
  'pointerover',
  event => {
    if (event.target instanceof Element && event.target.closest('a')) upgradeToVariableFonts();
  },
  {passive: true},
);
document.addEventListener('focusin', event => {
  if (event.target instanceof Element && event.target.closest('a')) upgradeToVariableFonts();
});
