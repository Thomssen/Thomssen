// Fade the hero content in on load.
document.querySelectorAll('.hero__copy > *, .stage').forEach((el, i) => {
  el.classList.add('reveal');
  setTimeout(() => el.classList.add('in'), 80 + i * 90);
});

// Mark the nav link for the section in view.
const links = [...document.querySelectorAll('.nav__links a')];
links.forEach(a => a.addEventListener('click', () => {
  links.forEach(l => l.classList.toggle('active', l === a));
}));

(function(){var t=document.getElementById("track");if(!t)return;function by(d){t.scrollBy({left:d*312,behavior:"smooth"})}document.getElementById("prev").addEventListener("click",function(){by(-1)});document.getElementById("next").addEventListener("click",function(){by(1)})})();

;// Live Discord counts. Set DISCORD_INVITE to the code in your invite link (discord.gg/CODE).
(function () {
  var DISCORD_INVITE = "thomsen";
  var box = document.getElementById("discord-stats");
  var join = document.getElementById("discord-link");
  if (!box || !DISCORD_INVITE) return;
  join.href = "https://discord.gg/" + DISCORD_INVITE;
  function fmt(n) { return Number(n).toLocaleString("en-US"); }
  function update() {
    fetch("https://discord.com/api/v10/invites/" + DISCORD_INVITE + "?with_counts=true")
      .then(function (r) { return r.ok ? r.json() : Promise.reject(); })
      .then(function (d) {
        document.getElementById("d-members").textContent = fmt(d.approximate_member_count);
        document.getElementById("d-online").textContent = fmt(d.approximate_presence_count);
        box.hidden = false;
      })
      .catch(function () {});
  }
  update();
  setInterval(update, 60000);
})();

;(function () {
  var root = document.documentElement;
  var reduce = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  document.querySelectorAll(".why h2, .why .eyebrow, .wcard, .faq h2, .faq .eyebrow, .faq__intro p, .qa, .discord").forEach(function (el) { el.classList.add("reveal"); });
  if (reduce || !("IntersectionObserver" in window)) return;
  root.classList.add("js");
  var io = new IntersectionObserver(function (es) {
    es.forEach(function (e) { if (e.isIntersecting) { e.target.classList.add("in"); io.unobserve(e.target); } });
  }, { rootMargin: "0px 0px -8% 0px", threshold: 0.05 });
  document.querySelectorAll(".reveal").forEach(function (el) { io.observe(el); });

  var arts = [].slice.call(document.querySelectorAll("[data-art]"));
  function tick() {
    var vh = window.innerHeight || 800;
    arts.forEach(function (el) {
      var r = el.getBoundingClientRect();
      var p = Math.max(0, Math.min(1, (vh - r.top) / (vh * 0.75)));
      el.style.transform = "scale(" + (0.9 + 0.1 * p).toFixed(3) + ") translateY(" + ((1 - p) * 24).toFixed(1) + "px)";
      el.style.opacity = (0.35 + 0.65 * p).toFixed(3);
    });
  }
  tick();
  window.addEventListener("scroll", function () { requestAnimationFrame(tick); }, { passive: true });
  window.addEventListener("resize", tick);
})();

;(function () {
  var reduce = window.matchMedia && window.matchMedia("(prefers-reduced-motion: reduce)").matches;
  // Word-by-word fill for the manifesto.
  var m = document.querySelector("[data-fill]");
  var words = [];
  if (m) {
    m.innerHTML = m.textContent.trim().split(/\s+/).map(function (w) { return '<span class="w">' + w + "</span>"; }).join(" ");
    words = [].slice.call(m.querySelectorAll(".w"));
  }
  function fill() {
    if (!m) return;
    var vh = window.innerHeight || 800, r = m.getBoundingClientRect();
    var p = Math.max(0, Math.min(1, (vh * 0.85 - r.top) / (vh * 0.55 + r.height * 0.5)));
    var n = p * words.length;
    words.forEach(function (w, i) { w.style.opacity = reduce ? 1 : Math.max(0.18, Math.min(1, n - i + 0.3)).toFixed(2); });
  }
  fill();
  window.addEventListener("scroll", function () { requestAnimationFrame(fill); }, { passive: true });
  window.addEventListener("resize", fill);
  if (reduce) return;
  // Cursor light on hero, cards and product panels. Tilt on product panels.
  function track(sel, tilt) {
    document.querySelectorAll(sel).forEach(function (el) {
      el.addEventListener("pointermove", function (e) {
        var r = el.getBoundingClientRect(), x = e.clientX - r.left, y = e.clientY - r.top;
        el.style.setProperty("--mx", x + "px"); el.style.setProperty("--my", y + "px");
        if (tilt) { var s = el.querySelector("svg"); if (s) s.style.transform = "rotateY(" + ((x / r.width - 0.5) * 7).toFixed(2) + "deg) rotateX(" + ((0.5 - y / r.height) * 7).toFixed(2) + "deg)"; }
      });
      if (tilt) el.addEventListener("pointerleave", function () { var s = el.querySelector("svg"); if (s) s.style.transform = ""; });
    });
  }
  track(".hero", false); track(".wcard, .qa, .discord", false); track("[data-art]", true);
})();
