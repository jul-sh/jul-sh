/** @typedef {{kind: 'closed'} | {kind: 'dismissed', trigger: HTMLElement}} RestingState */
/** @typedef {{kind: 'hover', trigger: HTMLButtonElement} | {kind: 'activated', trigger: HTMLButtonElement}} ActiveState */
/** @typedef {RestingState | ActiveState | {kind: 'closing', after: RestingState}} PreviewState */
const preview = document.querySelector('#image-preview');
const previewImage = preview.querySelector('.preview-crop');
const fullImage = preview.querySelector('.preview-full');
const hoverPointer = matchMedia('(hover: hover) and (pointer: fine)');
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
const triggers = document.querySelectorAll('button[data-portrait-zoom]');
/** @type {PreviewState} */
let state = {kind: 'closed'};

/** @param {RestingState} next */
function finishClose(next) {
  state = next;
  preview.hidden = true;
  delete preview.dataset.phase;
  delete preview.dataset.fullReady;
}

/** @param {RestingState | ActiveState} next */
function show(next) {
  if (state.kind === 'hover' || state.kind === 'activated') {
    state.trigger.setAttribute('aria-expanded', 'false');
  }
  switch (next.kind) {
    case 'closed':
    case 'dismissed':
      // An opening transition can be dismissed before it paints its first frame.
      // There is no opacity transition to await when the preview is still clear.
      if (preview.hidden || reducedMotion.matches || getComputedStyle(preview).opacity === '0') {
        finishClose(next);
      } else {
        state = {kind: 'closing', after: next};
        preview.dataset.phase = 'closing';
      }
      break;
    case 'hover':
    case 'activated': {
      state = next;
      state.trigger.setAttribute('aria-expanded', 'true');
      const image = state.trigger.querySelector('img');
      const rect = state.trigger.getBoundingClientRect();
      const ratio = rect.width / rect.height;
      const height = Math.min(280, innerHeight - 32, (innerWidth - 32) / ratio);
      const width = height * ratio;
      const left = Math.max(16, Math.min(innerWidth - width - 16, rect.left + rect.width / 2 - width / 2));
      const top = Math.max(16, Math.min(innerHeight - height - 16, rect.top + rect.height / 2 - height / 2));
      const startScale = rect.width / width;
      // With a centered transform origin, these offsets place the small preview
      // directly over the inline portrait before it grows into its final position.
      const startX = rect.left - left - (width - rect.width) / 2;
      const startY = rect.top - top - (height - rect.height) / 2;
      previewImage.src = image.currentSrc || image.src;
      if (!fullImage.getAttribute('src')) fullImage.src = preview.dataset.fullSrc;
      Object.assign(preview.style, {width: `${width}px`, height: `${height}px`, left: `${left}px`, top: `${top}px`});
      preview.style.setProperty('--preview-start-scale', String(startScale));
      preview.style.setProperty('--preview-start-x', `${startX}px`);
      preview.style.setProperty('--preview-start-y', `${startY}px`);
      if (preview.hidden) {
        preview.hidden = false;
        // Establish the closed appearance before starting the opening transition.
        preview.getBoundingClientRect();
      }
      preview.dataset.phase = 'open';
      if (fullImage.complete && fullImage.naturalWidth) {
        requestAnimationFrame(() => {
          if (preview.dataset.phase === 'open') preview.dataset.fullReady = '';
        });
      }
      break;
    }
  }
}

fullImage.addEventListener('load', () => {
  if (preview.dataset.phase === 'open') preview.dataset.fullReady = '';
});

for (const trigger of triggers) {
  trigger.addEventListener('click', () => {
    if (state.kind === 'activated' && state.trigger === trigger) {
      dismiss();
    } else {
      show({kind: 'activated', trigger});
    }
  });
  trigger.addEventListener('pointerenter', event => {
    if (event.pointerType === 'touch' || !hoverPointer.matches) return;
    switch (state.kind) {
      case 'closed':
      case 'hover':
        show({kind: 'hover', trigger});
        break;
      case 'dismissed':
        if (state.trigger !== trigger) show({kind: 'hover', trigger});
        break;
      case 'closing':
        switch (state.after.kind) {
          case 'closed':
            show({kind: 'hover', trigger});
            break;
          case 'dismissed':
            if (state.after.trigger !== trigger) show({kind: 'hover', trigger});
            break;
        }
        break;
      case 'activated':
        break;
    }
  });
  trigger.addEventListener('pointerleave', event => {
    switch (state.kind) {
      case 'hover':
        if (state.trigger !== trigger) break;
        if (event.relatedTarget instanceof Node && preview.contains(event.relatedTarget)) break;
        show({kind: 'closed'});
        break;
      case 'dismissed':
        if (state.trigger === trigger) show({kind: 'closed'});
        break;
      case 'closing':
        switch (state.after.kind) {
          case 'dismissed':
            if (state.after.trigger === trigger) show({kind: 'closed'});
            break;
          case 'closed':
            break;
        }
        break;
      case 'closed':
      case 'activated':
        break;
    }
  });
}

preview.addEventListener('pointerleave', event => {
  switch (state.kind) {
    case 'hover':
    case 'dismissed':
      // Dismissal lasts while the pointer remains in the original thumbnail.
      if (event.relatedTarget instanceof Node && state.trigger.contains(event.relatedTarget)) break;
      show({kind: 'closed'});
      break;
    case 'closing':
      switch (state.after.kind) {
        case 'dismissed':
          if (event.relatedTarget instanceof Node && state.after.trigger.contains(event.relatedTarget)) break;
          show({kind: 'closed'});
          break;
        case 'closed':
          break;
      }
      break;
    case 'closed':
    case 'activated':
      break;
  }
});

preview.addEventListener('transitionend', event => {
  if (event.target !== preview || event.propertyName !== 'opacity') return;
  switch (state.kind) {
    case 'closing':
      finishClose(state.after);
      break;
    case 'closed':
    case 'dismissed':
    case 'hover':
    case 'activated':
      break;
  }
});

reducedMotion.addEventListener('change', () => {
  if (!reducedMotion.matches) return;
  switch (state.kind) {
    case 'closing':
      finishClose(state.after);
      break;
    case 'closed':
    case 'dismissed':
    case 'hover':
    case 'activated':
      break;
  }
});

function dismiss() {
  switch (state.kind) {
    case 'hover':
    case 'activated':
      show({kind: 'dismissed', trigger: state.trigger});
      break;
    case 'closed':
    case 'dismissed':
    case 'closing':
      break;
  }
}

document.addEventListener('keydown', event => {
  if (event.key === 'Escape') dismiss();
});
document.addEventListener('click', event => {
  if (state.kind === 'activated' && !state.trigger.contains(event.target)) dismiss();
});
window.addEventListener('scroll', dismiss, {passive: true});
window.addEventListener('resize', dismiss);
