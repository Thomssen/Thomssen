import { useState } from "react";
import { AtSign, Phone, User, Home, Globe, CreditCard, KeyRound, AtSign as At2, CalendarDays, Sparkles, Plus, Pencil, Trash2, Check, X, FlaskConical, type LucideIcon } from "lucide-react";
import { Button, Card, Input, Notice, PageHeader, Row, Secret, Select, Toggle } from "../components/ui";
import { api, CATEGORY_LABEL, type EntryKind, type Settings } from "../lib/api";
import { useApp } from "../lib/app";

type FilterKey = keyof Settings["filters"];
const FILTERS: { key: FilterKey; title: string; desc: string; icon: LucideIcon }[] = [
  { key: "emails", title: "Email addresses", desc: "Any address like name@domain.tld, plus the ones you add below.", icon: AtSign },
  { key: "phones", title: "Phone numbers", desc: "International and local formats, labelled numbers, and your own numbers.", icon: Phone },
  { key: "names", title: "Names", desc: "Your saved real names (all common spellings) and labelled or greeted names such as “Name: …”, “Dear …”, “Mr …”.", icon: User },
  { key: "addresses", title: "Home addresses", desc: "Street + number patterns (EN/DK/DE/NL/SE), US ZIP lines, UK postcodes, PO boxes and your saved addresses.", icon: Home },
  { key: "ips", title: "IP addresses", desc: "IPv4 and IPv6. Loopback and unspecified addresses are ignored.", icon: Globe },
  { key: "financial", title: "Financial information", desc: "Credit cards (Luhn-checked), IBANs (mod-97 checked) and labelled bank account numbers.", icon: CreditCard },
  { key: "passwords", title: "Passwords & tokens", desc: "“password: …” values, bearer tokens, JWTs, API keys and common token formats.", icon: KeyRound },
  { key: "usernames", title: "Usernames", desc: "Labelled usernames (“user: …”, “login: …”) and the usernames you save.", icon: At2 },
  { key: "dob", title: "Dates of birth", desc: "Dates that follow a birth label such as “DOB” or “Date of birth”.", icon: CalendarDays },
  { key: "custom", title: "Custom information", desc: "Your own keywords and text patterns (regular expressions).", icon: Sparkles },
];

const KINDS: { value: EntryKind; label: string; ph: string }[] = [
  { value: "name", label: "Real name", ph: "Jane Marie Doe" },
  { value: "email", label: "Email address", ph: "jane.doe@example.com" },
  { value: "phone", label: "Phone number", ph: "+1 555 012 3456" },
  { value: "address", label: "Home address", ph: "12 Example Street, Springfield 12345" },
  { value: "username", label: "Username", ph: "janedoe42" },
  { value: "keyword", label: "Keyword / phrase", ph: "Project Falcon" },
  { value: "regex", label: "Text pattern (regex)", ph: "TICKET-\\d{4}" },
];

function EntryList() {
  const { entries, reloadEntries, toast } = useApp();
  const [kind, setKind] = useState<EntryKind>("name");
  const [value, setValue] = useState("");
  const [editing, setEditing] = useState<string | null>(null);
  const [draft, setDraft] = useState("");
  const [draftKind, setDraftKind] = useState<EntryKind>("name");
  const ph = KINDS.find((k) => k.value === kind)!.ph;

  const add = async () => {
    try { await api.addEntry(kind, value); setValue(""); await reloadEntries(); toast("Saved (encrypted with Windows DPAPI)"); }
    catch (e) { toast(String(e), "error"); }
  };
  const save = async (id: string) => {
    try { await api.updateEntry(id, draftKind, draft); setEditing(null); await reloadEntries(); }
    catch (e) { toast(String(e), "error"); }
  };
  const del = async (id: string) => { try { await api.deleteEntry(id); await reloadEntries(); } catch (e) { toast(String(e), "error"); } };

  return (
    <Card title="My personal information" subtitle="Teach Privacy Blur what to hide. Stored encrypted with Windows DPAPI, readable only by your Windows account on this PC, and never sent anywhere.">
      <div className="flex gap-2">
        <Select value={kind} options={KINDS} onChange={setKind} />
        <Input value={value} placeholder={ph} onChange={(e) => setValue(e.target.value)} onKeyDown={(e) => e.key === "Enter" && value.trim() && add()} />
        <Button kind="primary" icon={Plus} disabled={!value.trim()} onClick={add}>Add</Button>
      </div>
      <p className="mt-2 text-[12px] text-mute-500">Entries are matched case-insensitively and tolerate different spacing. Phone numbers also match without the country code.</p>
      <ul className="mt-4 space-y-1.5">
        {entries.length === 0 && <li className="rounded-lg bg-ink-900 px-3 py-4 text-center text-mute-500">Nothing saved yet.</li>}
        {entries.map((en) => (
          <li key={en.id} className="flex items-center gap-3 rounded-lg bg-ink-900 px-3 py-2">
            {editing === en.id ? (
              <>
                <Select value={draftKind} options={KINDS} onChange={setDraftKind} />
                <Input value={draft} autoFocus onChange={(e) => setDraft(e.target.value)} onKeyDown={(e) => e.key === "Enter" && save(en.id)} />
                <Button kind="primary" icon={Check} onClick={() => save(en.id)} aria-label="Save" />
                <Button kind="ghost" icon={X} onClick={() => setEditing(null)} aria-label="Cancel" />
              </>
            ) : (
              <>
                <span className="w-32 shrink-0 text-[12px] text-mute-400">{KINDS.find((k) => k.value === en.kind)?.label}</span>
                <span className="min-w-0 flex-1 truncate"><Secret value={en.value} /></span>
                <Button kind="ghost" icon={Pencil} aria-label="Edit" onClick={() => { setEditing(en.id); setDraft(en.value); setDraftKind(en.kind); }} />
                <Button kind="ghost" icon={Trash2} aria-label="Delete" onClick={() => del(en.id)} />
              </>
            )}
          </li>
        ))}
      </ul>
    </Card>
  );
}

function Tester() {
  const [text, setText] = useState("Contact: jane.doe@example.com, +1 (555) 012-3456\nIP 192.168.1.20 · card 4111 1111 1111 1111");
  const [hits, setHits] = useState<{ start: number; end: number; category: string }[]>([]);
  const run = async () => setHits(await api.testText(text));
  const parts: { t: string; cat?: string }[] = [];
  let pos = 0;
  [...hits].sort((a, b) => a.start - b.start).forEach((h) => {
    if (h.start < pos) return;
    if (h.start > pos) parts.push({ t: text.slice(pos, h.start) });
    parts.push({ t: text.slice(h.start, h.end), cat: h.category });
    pos = h.end;
  });
  if (pos < text.length) parts.push({ t: text.slice(pos) });
  return (
    <Card title="Test detection" subtitle="Type or paste sample text to see what your current filters would hide. Runs locally; nothing is stored.">
      <textarea value={text} onChange={(e) => setText(e.target.value)} rows={3} className="focus-ring w-full resize-none rounded-lg border border-ink-700 bg-ink-900 p-3 font-mono text-[12.5px]" />
      <div className="mt-2 flex items-center gap-3">
        <Button icon={FlaskConical} onClick={run}>Check text</Button>
        <span className="text-[12.5px] text-mute-500">{hits.length ? `${hits.length} sensitive item(s) found` : "Press Check text"}</span>
      </div>
      {hits.length > 0 && (
        <pre className="mt-3 whitespace-pre-wrap rounded-lg bg-ink-900 p-3 font-mono text-[12.5px] leading-relaxed">
          {parts.map((p, i) => p.cat ? <mark key={i} title={CATEGORY_LABEL[p.cat]} className="rounded bg-zinc-100 px-0.5 text-ink-950">{p.t}</mark> : <span key={i}>{p.t}</span>)}
        </pre>
      )}
    </Card>
  );
}

export default function Filters() {
  const { settings, patch } = useApp();
  if (!settings) return null;
  return (
    <>
      <PageHeader title="Privacy Filters" subtitle="Choose what counts as sensitive. Changes apply instantly." />
      <Card className="mb-4">
        {FILTERS.map(({ key, title, desc, icon }) => (
          <Row key={key} icon={icon} title={title} desc={desc}>
            <Toggle checked={settings.filters[key]} onChange={(v) => patch((s) => { s.filters[key] = v; })} />
          </Row>
        ))}
      </Card>
      <div className="mb-4"><EntryList /></div>
      <div className="mb-4"><Tester /></div>
      <Notice tone="info">
        <b>How accurate is this?</b> Emails, IPs, cards, IBANs and tokens are matched by strict, validated patterns. Names and addresses cannot be recognised reliably from text alone (that would need cloud-sized language models), so add yours above: saved entries are matched exactly, in several spellings.
      </Notice>
    </>
  );
}
