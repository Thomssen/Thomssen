// Cookie settings: stores only the visitor's own choice in local storage.
(function () {
  var KEY = "thomsen_cookie_choice";
  function read() { try { return JSON.parse(localStorage.getItem(KEY)); } catch (e) { return null; } }
  function write(v) { try { localStorage.setItem(KEY, JSON.stringify(v)); } catch (e) {} }
  var saved = read();

  var bar = document.createElement("div");
  bar.className = "ckbar"; bar.hidden = true; bar.setAttribute("role", "region"); bar.setAttribute("aria-label", "Cookie notice");
  bar.innerHTML = '<p>This site only stores your theme and cookie choice in your browser. No analytics or advertising.</p><div><button type="button" class="btn btn--ghost btn--sm" data-ck="open">Settings</button><button type="button" class="btn btn--light btn--sm" data-ck="ok">Got it</button></div>';

  var dlg = document.createElement("div");
  dlg.className = "ck"; dlg.hidden = true;
  dlg.innerHTML = '<div class="ck__box" role="dialog" aria-modal="true" aria-labelledby="ck-title" tabindex="-1">' +
    '<h2 id="ck-title">Cookie settings</h2>' +
    '<p class="ck__intro">We keep browser storage to a minimum. Here is exactly what this site uses.</p>' +
    '<div class="ck__row"><div><strong>Strictly necessary</strong><p>Stores your theme and cookie choice in your browser\'s local storage. It is never sent to us and is not used to track you.</p></div><span class="ck__tag">Always on</span></div>' +
    '<div class="ck__row"><div><strong>Analytics and advertising</strong><p>Not used. This site runs no analytics or advertising scripts. If that ever changes, it will only happen with your consent.</p></div><label class="ck__sw"><input type="checkbox" id="ck-opt" aria-label="Allow analytics and advertising"><span></span></label></div>' +
    '<div class="ck__act"><button type="button" class="btn btn--light btn--sm" data-ck="save">Save choice</button><button type="button" class="btn btn--ghost btn--sm" data-ck="close">Close</button><a href="privacy.html#s3">Privacy Policy</a></div></div>';
  document.body.appendChild(bar); document.body.appendChild(dlg);

  var box = dlg.querySelector(".ck__box"), opt = dlg.querySelector("#ck-opt"), last = null;
  function openDlg() { last = document.activeElement; var c = read(); opt.checked = !!(c && c.optional); dlg.hidden = false; bar.hidden = true; box.focus(); }
  function closeDlg() { dlg.hidden = true; if (last && last.focus) last.focus(); if (!read()) bar.hidden = false; }
  function save() { write({ necessary: true, optional: opt.checked, date: new Date().toISOString() }); dlg.hidden = true; bar.hidden = true; if (last && last.focus) last.focus(); }

  bar.addEventListener("click", function (e) {
    var a = e.target.getAttribute && e.target.getAttribute("data-ck");
    if (a === "open") openDlg();
    if (a === "ok") { write({ necessary: true, optional: false, date: new Date().toISOString() }); bar.hidden = true; }
  });
  dlg.addEventListener("click", function (e) {
    var a = e.target.getAttribute && e.target.getAttribute("data-ck");
    if (a === "save") save(); else if (a === "close" || e.target === dlg) closeDlg();
  });
  document.addEventListener("keydown", function (e) {
    if (e.key === "Escape" && !dlg.hidden) closeDlg();
    if (e.key === "Tab" && !dlg.hidden) {
      var f = [].slice.call(dlg.querySelectorAll("button, input, a")); if (!f.length) return;
      var first = f[0], lastEl = f[f.length - 1];
      if (e.shiftKey && (document.activeElement === first || document.activeElement === box)) { e.preventDefault(); lastEl.focus(); }
      else if (!e.shiftKey && document.activeElement === lastEl) { e.preventDefault(); first.focus(); }
    }
  });
  document.addEventListener("click", function (e) {
    var t = e.target.closest && e.target.closest("[data-cookie-settings]");
    if (t) { e.preventDefault(); openDlg(); }
  });
  if (!saved) bar.hidden = false;
})();
