(() => {
  'use strict';
  const modal = document.getElementById('thomsen-bot-dialog');
  if (!modal || typeof modal.showModal !== 'function') return;
  let lastOpener = null;
  const opens = document.querySelectorAll('[data-bot-popup-open]');
  opens.forEach(opener => opener.addEventListener('click', (event) => {
    event.preventDefault();
    if (modal.open) return;
    lastOpener = opener;
    modal.querySelectorAll('details[open]').forEach(details => { details.open = false; });
    modal.showModal();
    document.documentElement.classList.add('tb-dialog-open');
    modal.scrollTop = 0;
  }));
  modal.querySelector('[data-bot-popup-close]').addEventListener('click', () => modal.close());
  modal.addEventListener('click', event => {
    if (event.target === modal) modal.close();
  });
  modal.addEventListener('close', () => {
    document.documentElement.classList.remove('tb-dialog-open');
    if (lastOpener && lastOpener.isConnected) lastOpener.focus();
  });
})();
