export const metadata = { title: "Privacy policy · PSYCHOSEC" };

const retention = { login: process.env.NEXT_PUBLIC_LOGIN_IP_RETENTION_DAYS ?? "90", session: "30", activity: "180" };
const H = ({ children }: { children: React.ReactNode }) => <h2 className="mb-2 mt-9 text-base font-medium text-fg">{children}</h2>;

export default function Privacy() {
  return (
    <main className="max-w-2xl space-y-3 py-12 text-sm leading-relaxed text-dim">
      <h1 className="text-2xl font-semibold text-fg">Privacy policy</h1>
      <p>This policy covers the PSYCHOSEC Windows application, psychosec.org and the administration service. PSYCHOSEC is operated for authorized members of the PSYCHOSEC Discord community.</p>

      <H>What stays on your computer</H>
      <p>Everything you investigate: usernames, domains, IP addresses, email addresses, files, case notes, findings and exported reports. This data is stored locally in an encrypted database and is <b className="text-fg">never sent to PSYCHOSEC servers</b>. The lookups themselves are sent directly from your computer to the public services needed to answer them (see below).</p>

      <H>What the PSYCHOSEC service collects</H>
      <ul className="list-disc space-y-1 pl-5">
        <li><b className="text-fg">Discord identity:</b> your Discord user ID, username, display name and avatar URL, whether you are a member of the server, and whether you hold the required role.</li>
        <li><b className="text-fg">Sign-in records:</b> time, result and the <b className="text-fg">public IP address</b> from which you signed in (read by our server from the connection, never supplied by the application), an approximate country derived by the hosting network, and the application version.</li>
        <li><b className="text-fg">Session records:</b> session creation, last activity and expiry times, application version, and the IP address at session creation.</li>
        <li><b className="text-fg">Operational events:</b> application launched, update checked, signed out, session expired or revoked.</li>
        <li><b className="text-fg">Administrative actions</b> affecting your account (access changes, bans), with the reason.</li>
      </ul>
      <p>We do not collect hardware fingerprints, precise location, browsing history, your investigation content, or any information about other activity on your computer.</p>

      <H>Why we record IP addresses</H>
      <p>To protect the service and administer access: detecting account sharing and credential abuse, investigating suspicious sign-ins, and enforcing bans. They are not used for advertising or sold.</p>

      <H>Who can access it</H>
      <p>Only administrators approved by PSYCHOSEC, through an authenticated, audited dashboard. Every administrative action is logged. Our processors (below) host the data.</p>

      <H>How long we keep it</H>
      <ul className="list-disc space-y-1 pl-5">
        <li>Sign-in records including IP addresses: {retention.login} days.</li>
        <li>Ended or expired sessions: {retention.session} days after they end.</li>
        <li>Operational events: {retention.activity} days.</li>
        <li>Account record and audit log: while you hold access, and as needed to document moderation decisions.</li>
      </ul>

      <H>Your choices and deletion requests</H>
      <p>You may ask for access to, or deletion of, the personal data we hold about you by contacting a PSYCHOSEC administrator through the Discord server. Deleting your account record also removes your sign-in and session records; audit entries about administrative actions may be retained in anonymised form where needed for security.</p>

      <H>External services that process information</H>
      <ul className="list-disc space-y-1 pl-5">
        <li><b className="text-fg">Discord</b>: sign-in (OAuth2) and server membership checks.</li>
        <li><b className="text-fg">Vercel</b>: hosting of the website, API and dashboard (receives request metadata such as IP address).</li>
        <li><b className="text-fg">Supabase</b>: database hosting for the records listed above.</li>
        <li><b className="text-fg">Services you query</b> from the application, sent directly from your computer: Cloudflare DNS-over-HTTPS (domain names you look up), RDAP registries via rdap.org, crt.sh, ipwho.is, public profile endpoints of the platforms searched by username, and, only if you configure your own keys, AbuseIPDB and Have I Been Pwned. Each has its own privacy policy.</li>
      </ul>

      <H>Security</H>
      <p>All communication uses HTTPS. Session tokens are stored on the server only as hashes and on your computer in Windows Credential Manager. Access decisions are made on our servers, never in the application.</p>
    </main>
  );
}
