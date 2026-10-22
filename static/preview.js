/** @typedef {{kind: 'closed'} | {kind: 'dismissed', trigger: HTMLElement}} RestingState */
/** @typedef {{kind: 'hover', trigger: HTMLElement} | {kind: 'focus', trigger: HTMLElement} | {kind: 'tap', trigger: HTMLElement}} ActiveState */
/** @typedef {RestingState | ActiveState | {kind: 'closing', after: RestingState}} PreviewState */
const preview = document.querySelector('#image-preview');
const previewImage = preview.querySelector('.preview-crop');
const fullImage = preview.querySelector('.preview-full');
const hoverPointer = matchMedia('(hover: hover) and (pointer: fine)');
const reducedMotion = matchMedia('(prefers-reduced-motion: reduce)');
const triggers = document.querySelectorAll('[data-portrait-zoom]');
/** @type {PreviewState} */
let state = {kind: 'closed'};

/** @param {PreviewState} next */
function setState(next) {
  state = next;
  for (const trigger of triggers) trigger.setAttribute('aria-expanded', 'false');
  switch (state.kind) {
    case 'hover':
    case 'focus':
    case 'tap':
      state.trigger.setAttribute('aria-expanded', 'true');
      break;
    case 'closed':
    case 'dismissed':
    case 'closing':
      break;
  }
}

/** @param {RestingState} next */
function finishClose(next) {
  setState(next);
  preview.hidden = true;
  delete preview.dataset.phase;
  delete preview.dataset.input;
  delete preview.dataset.fullReady;
}

/** @param {RestingState | ActiveState} next */
function show(next) {
  switch (next.kind) {
    case 'closed':
    case 'dismissed':
      // An opening transition can be dismissed before it paints its first frame.
      // There is no opacity transition to await when the preview is still clear.
      if (preview.hidden || reducedMotion.matches || getComputedStyle(preview).opacity === '0') {
        finishClose(next);
      } else {
        setState({kind: 'closing', after: next});
        preview.dataset.phase = 'closing';
      }
      break;
    case 'hover':
    case 'focus':
    case 'tap': {
      setState(next);
      const image = state.trigger.querySelector('img');
      const triggerRect = state.trigger.getBoundingClientRect();
      // The inline oval reserves width only, keeping the paragraph's leading even.
      const rect = triggerRect.height === 0 ? image.getBoundingClientRect() : triggerRect;
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
      preview.dataset.input = state.kind;
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
  trigger.addEventListener('pointerdown', event => {
    // Touch activation is handled by click. Avoid opening on incidental focus
    // before the touch finishes, which could move the target under the finger.
    if (event.pointerType === 'touch' || !hoverPointer.matches) event.preventDefault();
  });
  trigger.addEventListener('click', event => {
    // One click path handles touch and keyboard activation without also toggling
    // on emulated mouse events or pinning a desktop hover preview.
    if (event.detail !== 0 && (event.pointerType === 'mouse' || (event.pointerType !== 'touch' && hoverPointer.matches))) return;
    switch (state.kind) {
      case 'tap':
        if (state.trigger === trigger) show({kind: 'dismissed', trigger});
        else show({kind: 'tap', trigger});
        break;
      case 'hover':
      case 'focus':
        if (event.detail === 0 && event.pointerType !== 'touch' && state.trigger === trigger) {
          show({kind: 'dismissed', trigger});
        } else {
          show({kind: 'tap', trigger});
        }
        break;
      case 'closed':
      case 'dismissed':
      case 'closing':
        show({kind: 'tap', trigger});
        break;
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
      case 'focus':
      case 'tap':
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
      case 'focus':
      case 'tap':
        break;
    }
  });
  trigger.addEventListener('focus', () => {
    if (!trigger.matches(':focus-visible')) return;
    switch (state.kind) {
      case 'tap':
        break;
      case 'closed':
      case 'dismissed':
      case 'closing':
      case 'hover':
      case 'focus':
        show({kind: 'focus', trigger});
        break;
    }
  });
  trigger.addEventListener('blur', () => {
    switch (state.kind) {
      case 'focus':
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
      case 'hover':
      case 'tap':
        break;
    }
  });
}

// Capture before the trigger's handler: the opening click passes through while
// a later click anywhere dismisses without reopening or following a link.
document.addEventListener('click', event => {
  switch (state.kind) {
    case 'hover':
    case 'focus':
    case 'tap':
      event.preventDefault();
      event.stopPropagation();
      show({kind: 'dismissed', trigger: state.trigger});
      break;
    case 'closing':
      // Let the closing animation finish before another click activates a link.
      event.preventDefault();
      event.stopPropagation();
      break;
    case 'closed':
    case 'dismissed':
      break;
  }
}, {capture: true});

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
    case 'focus':
    case 'tap':
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
    case 'focus':
    case 'tap':
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
    case 'focus':
    case 'tap':
      break;
  }
});

function dismiss() {
  switch (state.kind) {
    case 'hover':
    case 'focus':
    case 'tap':
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
window.addEventListener('scroll', dismiss, {passive: true});
window.addEventListener('resize', dismiss);
