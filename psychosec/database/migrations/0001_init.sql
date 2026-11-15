-- PSYCHOSEC core schema. All access goes through the backend using the service-role
-- key. RLS is enabled on every table with NO policies, which denies anon/authenticated
-- roles entirely (service role bypasses RLS).

create extension if not exists pgcrypto;

create type access_status as enum ('authorized', 'revoked', 'denied');
create type session_kind as enum ('app', 'admin', 'web');
create type login_result as enum (
  'success', 'failed_membership', 'failed_role', 'failed_banned', 'failed_revoked',
  'failed_oauth', 'failed_state', 'failed_admin_denied'
);
create type admin_role as enum ('admin', 'super_admin');

create table users (
  id                uuid primary key default gen_random_uuid(),
  discord_id        text not null unique check (discord_id ~ '^[0-9]{5,25}$'),
  username          text not null,
  display_name      text,
  avatar_url        text,
  access_status     access_status not null default 'authorized',
  guild_member      boolean not null default false,
  role_verified     boolean not null default false,
  created_at        timestamptz not null default now(),   -- first login
  last_login_at     timestamptz,
  last_activity_at  timestamptz
);

create table sessions (
  id                  uuid primary key default gen_random_uuid(),
  user_id             uuid not null references users(id) on delete cascade,
  kind                session_kind not null default 'app',
  token_hash          text not null unique,               -- sha256 hex; raw token never stored
  created_at          timestamptz not null default now(),
  last_seen_at        timestamptz not null default now(),
  expires_at          timestamptz not null,
  absolute_expires_at timestamptz not null,
  revoked_at          timestamptz,
  revoked_reason      text,
  application_version text,
  ip_address          inet
);
create index sessions_user_idx on sessions(user_id);
create index sessions_active_idx on sessions(expires_at) where revoked_at is null;

create table login_events (
  id                  bigserial primary key,
  user_id             uuid references users(id) on delete set null,
  discord_id          text,
  ip_address          inet,
  ip_country          text,                               -- approximate, from edge network
  result              login_result not null,
  created_at          timestamptz not null default now(),
  session_id          uuid references sessions(id) on delete set null,
  application_version text,
  client              text not null default 'cli' check (client in ('cli', 'admin', 'web'))
);
create index login_events_user_idx on login_events(user_id, created_at desc);
create index login_events_time_idx on login_events(created_at desc);

-- Operational events only. Never contains investigation content.
create table activity_events (
  id                  bigserial primary key,
  user_id             uuid references users(id) on delete set null,
  type                text not null check (type in (
    'login_succeeded','login_failed','session_expired','logout','membership_failed',
    'app_launched','update_checked','session_revoked','access_revoked','access_allowed','user_banned','user_unbanned')),
  created_at          timestamptz not null default now(),
  application_version text,
  detail              text
);
create index activity_time_idx on activity_events(created_at desc);
create index activity_user_idx on activity_events(user_id, created_at desc);

create table bans (
  id          uuid primary key default gen_random_uuid(),
  user_id     uuid not null references users(id) on delete cascade,
  reason      text not null,                              -- internal
  created_by  uuid not null references users(id),
  created_at  timestamptz not null default now(),
  expires_at  timestamptz,
  revoked_at  timestamptz,
  revoked_by  uuid references users(id)
);
create index bans_user_idx on bans(user_id);
create unique index one_active_ban_idx on bans(user_id) where revoked_at is null;

create table admin_users (
  user_id    uuid primary key references users(id) on delete cascade,
  role       admin_role not null default 'admin',
  created_at timestamptz not null default now()
);

create table admin_audit_logs (
  id             bigserial primary key,
  admin_user_id  uuid not null references users(id),
  action         text not null,
  target_user_id uuid references users(id) on delete set null,
  reason         text,
  detail         jsonb,
  ip_address     inet,
  created_at     timestamptz not null default now()
);
create index audit_time_idx on admin_audit_logs(created_at desc);

-- Audit rows are append-only, even for the service role.
create function forbid_audit_change() returns trigger language plpgsql as $$
begin raise exception 'admin_audit_logs is append-only'; end $$;
create trigger audit_no_update before update or delete on admin_audit_logs
  for each row execute function forbid_audit_change();

create table application_releases (
  id              uuid primary key default gen_random_uuid(),
  version         text not null unique check (version ~ '^[0-9]+\.[0-9]+\.[0-9]+$'),
  download_url    text not null check (download_url like 'https://%'),
  checksum_sha256 text not null check (checksum_sha256 ~ '^[0-9a-f]{64}$'),
  signature       text not null,                          -- ed25519 over the release manifest
  notes           text,
  published_at    timestamptz not null default now(),
  withdrawn_at    timestamptz
);

-- Pending browser logins started by the CLI (PKCE-style).
create table login_requests (
  id            uuid primary key default gen_random_uuid(),
  login_id      text not null unique,
  client        text not null default 'cli' check (client in ('cli','admin','web')),
  challenge     text,                                     -- cli only: b64url(sha256(verifier))
  oauth_state   text unique,
  oauth_verifier text,                                    -- Discord PKCE verifier (server side)
  status        text not null default 'pending' check (status in ('pending','approved','denied','consumed')),
  user_id       uuid references users(id) on delete cascade,
  message       text,
  ip_address    inet,
  ip_country    text,
  application_version text,
  created_at    timestamptz not null default now(),
  expires_at    timestamptz not null
);
create index login_requests_exp_idx on login_requests(expires_at);

-- Fixed-window rate limiter.
create table rate_limits (
  key          text primary key,
  window_start timestamptz not null,
  count        int not null
);

create function rate_limit_hit(p_key text, p_window_seconds int, p_max int)
returns boolean language plpgsql as $$
declare c int;
begin
  insert into rate_limits(key, window_start, count) values (p_key, now(), 1)
  on conflict (key) do update set
    window_start = case when rate_limits.window_start < now() - make_interval(secs => p_window_seconds) then now() else rate_limits.window_start end,
    count        = case when rate_limits.window_start < now() - make_interval(secs => p_window_seconds) then 1 else rate_limits.count + 1 end
  returning count into c;
  return c <= p_max;
end $$;

-- Atomically consume an approved login request (single use).
create function consume_login_request(p_login_id text) returns login_requests
language plpgsql as $$
declare r login_requests;
begin
  update login_requests set status = 'consumed'
   where login_id = p_login_id and status = 'approved' and expires_at > now()
  returning * into r;
  return r;
end $$;

-- Retention. Schedule with pg_cron (see docs/deployment.md).
create function purge_old_records(p_login_days int default 90, p_session_days int default 30, p_activity_days int default 180)
returns void language plpgsql as $$
begin
  delete from login_events   where created_at < now() - make_interval(days => p_login_days);
  delete from activity_events where created_at < now() - make_interval(days => p_activity_days);
  delete from sessions where (revoked_at is not null and revoked_at < now() - make_interval(days => p_session_days))
                          or absolute_expires_at < now() - make_interval(days => p_session_days);
  delete from login_requests where expires_at < now() - interval '1 day';
  delete from rate_limits    where window_start < now() - interval '1 day';
end $$;

-- Row Level Security: enabled everywhere, no policies => anon/authenticated denied.
alter table users                enable row level security;
alter table sessions             enable row level security;
alter table login_events         enable row level security;
alter table activity_events      enable row level security;
alter table bans                 enable row level security;
alter table admin_users          enable row level security;
alter table admin_audit_logs     enable row level security;
alter table application_releases enable row level security;
alter table login_requests       enable row level security;
alter table rate_limits          enable row level security;

revoke all on all tables    in schema public from anon, authenticated;
revoke all on all functions in schema public from anon, authenticated, public;
