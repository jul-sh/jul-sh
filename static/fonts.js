/** @typedef {{kind: 'idle'} | {kind: 'loading'} | {kind: 'ready'}} FontUpgradeState */
/** @typedef {{src:string,style:'normal'|'italic'}} FaceSpec */
/** @typedef {{kind:'unavailable'}|{kind:'available',faces:[FaceSpec,...FaceSpec[]]}} PageFonts */
const root = document.documentElement;
const SAFE_FONT_URL = /^[A-Za-z0-9._/?=-]+$/;

/** @param {string | undefined} raw */
function safeFontUrl(raw) {
  return raw && SAFE_FONT_URL.test(raw) && !raw.includes('..') ? raw : null;
}

/** @returns {PageFonts} */
function readPageFonts() {
  const romanSrc = safeFontUrl(root.dataset.vfRoman);
  if (!romanSrc) return {kind:'unavailable'};
  /** @type {[FaceSpec,...FaceSpec[]]} */
  const faces = [{src:romanSrc,style:'normal'}];
  const italicSrc = safeFontUrl(root.dataset.vfItalic);
  if (italicSrc) {
    faces.push({src:italicSrc,style:'italic'});
  }
  return {kind:'available',faces};
}
const pageFonts = readPageFonts();

/** @typedef {'kerberos' | 'caleuche'} ProfiledFamily */
function syncOpticalSizes() {
  /** @type {Array<{element:HTMLElement,family:ProfiledFamily,opticalSize:number}>} */
  const runs = [];
  for (const element of document.querySelectorAll('body,body *')) {
    if (!(element instanceof HTMLElement)) continue;
    const style = getComputedStyle(element);
    /** @type {ProfiledFamily} */
    let family;
    if (style.fontFamily.includes('Kerberos')) family = 'kerberos';
    else if (style.fontFamily.includes('Caleuche')) family = 'caleuche';
    else continue;
    const opticalSize = Math.min(72, Math.max(6, Number.parseFloat(style.fontSize) * .75));
    runs.push({element, family, opticalSize});
  }
  for (const {element, family, opticalSize} of runs) {
    element.style.fontOpticalSizing = 'none';
    switch (family) {
      case 'kerberos':
        element.style.fontVariationSettings = `'opsz' ${opticalSize}, 'GRAD' var(--font-grade), 'YTLC' var(--font-lowercase-height)`;
        break;
      case 'caleuche':
        element.style.fontVariationSettings = `'opsz' ${opticalSize}, 'YTLC' var(--font-lowercase-height)`;
        break;
    }
  }
}
syncOpticalSizes();
window.addEventListener('resize', syncOpticalSizes, {passive:true});
window.addEventListener('beforeprint', syncOpticalSizes);
window.addEventListener('afterprint', syncOpticalSizes);

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
  switch (pageFonts.kind) {
    case 'unavailable': return;
    case 'available': break;
  }
  if (typeof FontFace !== 'function' || !('fonts' in document)) return;

  upgradeState = {kind: 'loading'};
  const faces = pageFonts.faces.map(({src,style}) => new FontFace('Kerberos VF', `url("${src}")`, {
    weight:'300 700', display:'swap', style,
    ascentOverride:'96%', descentOverride:'24.5%', lineGapOverride:'0%',
  }));

  Promise.all(faces.map(face => face.load()))
    .then(loaded => {
      for (const face of loaded) document.fonts.add(face);
      upgradeState = {kind: 'ready'};
      root.dataset.vfReady = '';
      syncOpticalSizes();
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
    document.fonts.ready.then(() => {
      syncOpticalSizes();
      start();
    });
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
