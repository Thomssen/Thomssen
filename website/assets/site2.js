/* THOMSEN SHIELD — shared site behavior: theme toggle, mobile nav, footer year. */
(function () {
  var root = document.documentElement;

  function syncThemeIcons() {
    var toggle = document.querySelector('.theme-toggle');
    if (!toggle) return;
    var icons = toggle.querySelector('.theme-toggle__icons');
    if (!icons) return;
    var isDark = root.getAttribute('data-theme') !== 'light';
    icons.setAttribute('data-dark', isDark ? 'true' : 'false');
  }
  syncThemeIcons();

  document.querySelectorAll('.theme-toggle').forEach(function (toggle) {
    toggle.addEventListener('click', function () {
      var next = root.getAttribute('data-theme') === 'light' ? 'dark' : 'light';
      root.classList.add('theme-transition');
      root.setAttribute('data-theme', next);
      try { localStorage.setItem('thomsen-theme', next); } catch (e) { /* ignore */ }
      syncThemeIcons();
      window.setTimeout(function () { root.classList.remove('theme-transition'); }, 320);
    });
  });

  // Mobile nav burger
  var navWrap = document.querySelector('.nav-wrap');
  var burger = document.querySelector('.nav__burger');
  var scrim = document.querySelector('.nav-scrim');

  function closeMenu() {
    if (!navWrap) return;
    navWrap.classList.remove('is-menu-open');
    if (burger) burger.setAttribute('aria-expanded', 'false');
  }
  function toggleMenu() {
    if (!navWrap) return;
    var open = navWrap.classList.toggle('is-menu-open');
    if (burger) burger.setAttribute('aria-expanded', open ? 'true' : 'false');
  }

  if (burger) burger.addEventListener('click', toggleMenu);
  if (scrim) scrim.addEventListener('click', closeMenu);
  document.querySelectorAll('.nav-panel__link, .nav-panel__cta').forEach(function (link) {
    link.addEventListener('click', closeMenu);
  });
  document.addEventListener('keydown', function (e) {
    if (e.key === 'Escape') closeMenu();
  });

  // Footer year
  document.querySelectorAll('[data-year]').forEach(function (el) {
    el.textContent = new Date().getFullYear();
  });

  // Footer "Email" link - copies the support address instead of navigating
  var copyEmailLink = document.getElementById('copyEmail');
  if (copyEmailLink) {
    copyEmailLink.addEventListener('click', function (e) {
      e.preventDefault();
      var email = 'support@thomssen.dev';
      var oldText = copyEmailLink.textContent;
      function showCopied() {
        copyEmailLink.textContent = 'Copied!';
        window.setTimeout(function () { copyEmailLink.textContent = oldText; }, 1500);
      }
      if (navigator.clipboard && navigator.clipboard.writeText) {
        navigator.clipboard.writeText(email).then(showCopied, showCopied);
      } else {
        showCopied();
      }
    });
  }

  // FAQ accordion (Thomsen Shield product page)
  document.querySelectorAll('.faq-item').forEach(function (item) {
    var button = item.querySelector('.faq-item__q');
    var answer = item.querySelector('.faq-item__a');
    if (!button || !answer) return;
    button.addEventListener('click', function () {
      var isOpen = item.getAttribute('data-open') === 'true';
      if (isOpen) {
        item.setAttribute('data-open', 'false');
        button.setAttribute('aria-expanded', 'false');
        answer.style.maxHeight = '';
      } else {
        item.setAttribute('data-open', 'true');
        button.setAttribute('aria-expanded', 'true');
        answer.style.maxHeight = answer.scrollHeight + 'px';
      }
    });
  });

  // Feature modal (pricing card "See more") - clones a hidden <template>
  // per product into the shared modal shell. See main2.css section 13b.
  var featureModal = document.querySelector('.feature-modal');
  if (featureModal) {
    var featureModalBody = featureModal.querySelector('.feature-modal__body');

    function openFeatureModal(key) {
      var tpl = document.getElementById('feature-modal-' + key);
      if (!tpl || !featureModalBody) return;
      featureModalBody.innerHTML = '';
      featureModalBody.appendChild(tpl.content.cloneNode(true));
      featureModal.classList.add('is-in');
      featureModal.setAttribute('aria-hidden', 'false');
    }
    function closeFeatureModal() {
      featureModal.classList.remove('is-in');
      featureModal.setAttribute('aria-hidden', 'true');
    }

    document.querySelectorAll('[data-feature-modal]').forEach(function (btn) {
      btn.addEventListener('click', function () {
        openFeatureModal(btn.getAttribute('data-feature-modal'));
      });
    });
    featureModal.querySelectorAll('[data-feature-modal-close]').forEach(function (el) {
      el.addEventListener('click', closeFeatureModal);
    });
    document.addEventListener('keydown', function (e) {
      if (e.key === 'Escape') closeFeatureModal();
    });
  }
})();
