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
  var DISCORD_INVITE = "";
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
