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
