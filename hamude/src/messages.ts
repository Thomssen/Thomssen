// Everything the page says lives here, so it is easy to tweak.

export const dodgeMessages: string[] = [
  'Nice try, {name}. 😏',
  'Whoa, too slow!',
  'The NO button has left the chat.',
  'Error 404: "No" not found.',
  'It is giving... commitment issues.',
  'You really thought that would work?',
  'Cardio day for your mouse hand!',
  'Plot twist: the button is faster than you.',
  'The button is scared of you, {name}.',
  'Almost! (Not even close.)',
  'That button has a restraining order.',
  'Have you tried clicking YES instead?',
  'Skill issue. 💅',
  'The NO button called in sick.',
  'Denial is not just a river in Egypt.',
  'Missed me, missed me, now you have to kiss me!',
  'Bold of you to assume that button works.',
  'Your reflexes called. They want a refund.',
  'Peekaboo! 👀',
  'The button went to get milk. Forever.',
  'Zoom! Zoom! 🏎️',
  'Our lawyers say "NO" is currently unavailable.',
  'This is fine. Everything is fine. 🔥',
  'Even your cursor is rooting for YES.',
  'Maybe the real NO was the friends we made along the way.',
]

export const tapMessages: string[] = [
  'Nice try, {name}. Fat-finger denied! 👆',
  'Your thumb is slow, {name}.',
  'Tapped it? It teleported. 🪄',
]

export const clickMessages: string[] = [
  'Nice try, {name}.',
  'You clicked it?! It escaped anyway. 🏃',
  'Click registered. Denied. 📨',
  'Bold. Unsuccessful. But bold.',
]

export const noLabels: string[] = [
  'No', 'Nope', 'Nah', 'Never', 'Nuh-uh', 'Stop it', 'Hmm no', 'Wrong', 'Nooo', 'Please?', 'Pls no', 'Hard no', 'Eh', 'Hey!',
]

export const ranks: [number, string][] = [
  [0, 'Rookie'],
  [3, 'Wiggler'],
  [7, 'Slippery'],
  [12, 'Professional dodger'],
  [20, 'Cardio champion'],
  [30, 'Denial athlete'],
  [45, 'Legend of the chase'],
  [60, 'Okay, seriously, just click YES'],
]

export const milestones: Record<number, string> = {
  10: '10 attempts! The button is getting nervous. 😅',
  25: '25 attempts. Your mouse needs a vacation. 🏝️',
  42: '42. The answer to life, the universe, and the NO button.',
  50: 'Half a hundred! You are persistent. 🏆',
  69: 'Nice. 😎',
  100: '100 attempts! You unlocked: absolutely nothing. Go touch grass. 🌱',
}

export const fill = (s: string, name: string) => s.replaceAll('{name}', name)
