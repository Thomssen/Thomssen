/* THOMSEN SHIELD — cookie consent banner + settings modal.
   Builds the .tp-cc banner and .tp-modal settings dialog (styled in
   styles.css section 13) and stores the visitor's choice in
   localStorage under "thomsen-consent". No third-party consent library. */
(function () {
  var STORAGE_KEY = 'thomsen-consent';
  var banner = null;
  var modal = null;

  function getConsent() {
    try {
      var raw = localStorage.getItem(STORAGE_KEY);
      return raw ? JSON.parse(raw) : null;
    } catch (e) {
      return null;
    }
  }

  function setConsent(value) {
    try { localStorage.setItem(STORAGE_KEY, JSON.stringify(value)); } catch (e) { /* ignore */ }
    // Hook analytics here once you add a provider, e.g.:
    // if (value.analytics) loadAnalytics();
  }

  function buildBanner() {
    banner = document.createElement('div');
    banner.className = 'tp-cc';
    banner.setAttribute('role', 'dialog');
    banner.setAttribute('aria-label', 'Cookie preferences');
    banner.innerHTML =
      '<p class="tp-cc__title">Cookies</p>' +
      '<p class="tp-cc__text">We use essential local storage to run this site. The optional analytics choice below is reserved for future use - we don’t currently run any analytics or tracking scripts.</p>' +
      '<div class="tp-cc__actions">' +
        '<button type="button" class="btn btn--secondary" data-cc-decline>Essential only</button>' +
        '<button type="button" class="btn btn--primary" data-cc-accept>Accept all</button>' +
      '</div>' +
      '<button type="button" class="tp-cc__link" data-cc-settings>Cookie Settings</button>';
    document.body.appendChild(banner);
    requestAnimationFrame(function () { banner.classList.add('is-in'); });

    banner.querySelector('[data-cc-accept]').addEventListener('click', function () {
      setConsent({ essential: true, analytics: true });
      hideBanner();
    });
    banner.querySelector('[data-cc-decline]').addEventListener('click', function () {
      setConsent({ essential: true, analytics: false });
      hideBanner();
    });
    banner.querySelector('[data-cc-settings]').addEventListener('click', openModal);
  }

  function hideBanner() {
    if (!banner) return;
    banner.classList.remove('is-in');
    var toRemove = banner;
    banner = null;
    window.setTimeout(function () { toRemove.remove(); }, 320);
  }

  function buildModal() {
    modal = document.createElement('div');
    modal.className = 'tp-modal';
    modal.innerHTML =
      '<div class="tp-modal__backdrop" data-cc-close></div>' +
      '<div class="tp-modal__card" role="dialog" aria-label="Cookie settings">' +
        '<div class="tp-modal__head"><h2>Cookie settings</h2>' +
        '<button type="button" class="tp-modal__x" data-cc-close aria-label="Close">×</button></div>' +
        '<div class="tp-modal__row">' +
          '<div>' +
            '<p class="tp-modal__name">Essential <span>(required)</span></p>' +
            '<p class="tp-modal__desc">Needed for the site to work — can’t be turned off.</p>' +
          '</div>' +
          '<label class="tp-switch tp-switch--locked"><input type="checkbox" checked disabled /><span></span></label>' +
        '</div>' +
        '<div class="tp-modal__row">' +
          '<div>' +
            '<p class="tp-modal__name">Analytics</p>' +
            '<p class="tp-modal__desc">Reserved for future use - no analytics currently runs on this site either way. See the <a href="privacy.html">Privacy Policy</a>.</p>' +
          '</div>' +
          '<label class="tp-switch"><input type="checkbox" id="cc-analytics" /><span></span></label>' +
        '</div>' +
        '<button type="button" class="btn btn--primary" data-cc-save>Save preferences</button>' +
      '</div>';
    document.body.appendChild(modal);

    modal.querySelectorAll('[data-cc-close]').forEach(function (el) {
      el.addEventListener('click', closeModal);
    });
    modal.querySelector('[data-cc-save]').addEventListener('click', function () {
      var analytics = modal.querySelector('#cc-analytics').checked;
      setConsent({ essential: true, analytics: analytics });
      closeModal();
      hideBanner();
    });

    var existing = getConsent();
    if (existing) {
      modal.querySelector('#cc-analytics').checked = !!existing.analytics;
    }
  }

  function openModal() {
    if (!modal) buildModal();
    requestAnimationFrame(function () { modal.classList.add('is-in'); });
  }

  function closeModal() {
    if (!modal) return;
    modal.classList.remove('is-in');
  }

  document.addEventListener('DOMContentLoaded', function () {
    document.querySelectorAll('[data-cc-open]').forEach(function (el) {
      el.addEventListener('click', function (e) {
        e.preventDefault();
        openModal();
      });
    });

    if (!getConsent()) {
      buildBanner();
    }
  });
})();
