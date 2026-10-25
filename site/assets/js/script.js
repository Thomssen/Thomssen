// Thomsen Software front page behavior.
(function () {
  var reduce = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  var root = document.documentElement;
  var vh = function () { return window.innerHeight || 800; };

  // Scroll reveal
  document.querySelectorAll(".hero2__top > *, .stack__col, .stack h2, .stack .eyebrow, .why .eyebrow, .why h2, .bcard, .faq__intro, .qa, .discord, .story__left > .eyebrow, .story__left > h2, .foot__cols > *").forEach(function (el) { el.classList.add("reveal"); });
  if (!reduce && "IntersectionObserver" in window) {
    root.classList.add("js");
    var io = new IntersectionObserver(function (es) {
      es.forEach(function (e) { if (e.isIntersecting) { e.target.classList.add("in"); io.unobserve(e.target); } });
    }, { rootMargin: "0px 0px -8% 0px", threshold: 0.05 });
    document.querySelectorAll(".reveal").forEach(function (el) { io.observe(el); });
  }

  var hwin = document.getElementById("hwin");
  var story = document.querySelector("[data-story]");
  var items = [].slice.call(document.querySelectorAll(".story__item"));
  var slides = [].slice.call(document.querySelectorAll(".sslide"));
  var navLinks = [].slice.call(document.querySelectorAll(".nav2__links a"));
  var spy = ["home", "products", "why", "faq"].map(function (id) { return document.getElementById(id); });
  var m = document.querySelector("[data-fill]");
  var words = [];
  if (m) {
    m.innerHTML = m.textContent.trim().split(/\s+/).map(function (w) { return '<span class="w">' + w + "</span>"; }).join(" ");
    words = [].slice.call(m.querySelectorAll(".w"));
  }
  var wide = window.matchMedia("(min-width: 861px)");
  var current = -1;

  function setActive(i) {
    if (i === current) return;
    current = i;
    items.forEach(function (el, k) { el.classList.toggle("active", k === i); });
    slides.forEach(function (el, k) { el.classList.toggle("active", k === i); });
  }

  function update() {
    var y = window.scrollY || 0;
    if (hwin && !reduce) hwin.style.setProperty("--rx", Math.max(0, 18 - (y / (vh() * 0.55)) * 18).toFixed(2) + "deg");
    if (m) {
      var r = m.getBoundingClientRect();
      var p = Math.max(0, Math.min(1, (vh() * 0.85 - r.top) / (vh() * 0.55 + r.height * 0.5)));
      var n = p * words.length;
      words.forEach(function (w, i) { w.style.opacity = reduce ? 1 : Math.max(0.18, Math.min(1, n - i + 0.3)).toFixed(2); });
    }
    if (story && wide.matches) {
      var sr = story.getBoundingClientRect();
      var span = Math.max(1, sr.height - vh());
      var prog = Math.max(0, Math.min(0.999, -sr.top / span));
      setActive(Math.floor(prog * items.length));
    }
    var cur = 0;
    spy.forEach(function (s, i) { if (s && s.getBoundingClientRect().top <= vh() * 0.4) cur = i; });
    navLinks.forEach(function (a, i) { a.classList.toggle("active", i === cur); });
  }
  var queued = false;
  function onScroll() { if (queued) return; queued = true; requestAnimationFrame(function () { queued = false; update(); }); }
  window.addEventListener("scroll", onScroll, { passive: true });
  window.addEventListener("resize", onScroll);
  update();

  items.forEach(function (el, i) {
    el.addEventListener("click", function () {
      if (!story || !wide.matches) return;
      var top = story.offsetTop + ((i + 0.5) / items.length) * (story.offsetHeight - vh());
      window.scrollTo({ top: top, behavior: reduce ? "auto" : "smooth" });
    });
  });

  // Cursor light on cards
  if (!reduce) {
    document.querySelectorAll(".bcard, .qa, .discord").forEach(function (el) {
      el.addEventListener("pointermove", function (e) {
        var r = el.getBoundingClientRect();
        el.style.setProperty("--mx", (e.clientX - r.left) + "px");
        el.style.setProperty("--my", (e.clientY - r.top) + "px");
      });
    });
  }

  // Live Discord counts. Set DISCORD_INVITE to the code in your invite link (discord.gg/CODE).
  var DISCORD_INVITE = "thomsen";
  var box = document.getElementById("discord-stats");
  var join = document.getElementById("discord-link");
  if (box && DISCORD_INVITE) {
    join.href = "https://discord.gg/" + DISCORD_INVITE;
    var fmt = function (n) { return Number(n).toLocaleString("en-US"); };
    var refresh = function () {
      fetch("https://discord.com/api/v10/invites/" + DISCORD_INVITE + "?with_counts=true")
        .then(function (r) { return r.ok ? r.json() : Promise.reject(); })
        .then(function (d) {
          document.getElementById("d-members").textContent = fmt(d.approximate_member_count);
          document.getElementById("d-online").textContent = fmt(d.approximate_presence_count);
          box.hidden = false;
        })
        .catch(function () {});
    };
    refresh();
    setInterval(refresh, 60000);
  }
})();

// Easter egg: edit the theme string in the code block on the home page.
// Type "light" (or "dark") and the whole site switches theme. The choice is saved in the browser.
(function () {
  var inp = document.getElementById("theme-in");
  if (!inp) return;
  var root = document.documentElement;
  function current() { return root.getAttribute("data-theme") === "light" ? "light" : "dark"; }
  function fit() { inp.size = Math.max(4, inp.value.length || 1); }
  function setTheme(t) {
    if (t === "light") root.setAttribute("data-theme", "light"); else root.removeAttribute("data-theme");
    try { localStorage.setItem("thomsen_theme", t); } catch (e) {}
  }
  inp.value = current(); fit();
  inp.addEventListener("focus", function () { inp.select(); });
  inp.addEventListener("input", function () {
    var v = inp.value.trim().toLowerCase();
    if ((v === "light" || v === "dark") && v !== current()) { setTheme(v); inp.value = v; inp.blur(); }
    fit();
  });
  inp.addEventListener("keydown", function (e) { if (e.key === "Enter") inp.blur(); });
  inp.addEventListener("blur", function () { inp.value = current(); fit(); });
})();
