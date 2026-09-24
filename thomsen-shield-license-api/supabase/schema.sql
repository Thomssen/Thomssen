-- Thomsen Shield license database.
-- Run this once in the Supabase SQL Editor for your project (or via the
-- Supabase CLI). Safe to re-run - every statement is idempotent.

create extension if not exists pgcrypto;

create table if not exists licenses (
  id uuid primary key default gen_random_uuid(),
  license_key text not null unique,
  email text not null,
  product text not null default 'thomsen-shield',
  status text not null default 'active' check (status in ('active', 'disabled', 'refunded')),

  -- "1 active PC at a time": null means not currently bound to any
  -- device. Not overly aggressive hardware locking - see the Rust
  -- client's `license::device_id`, a hash of the Windows install's own
  -- MachineGuid, nothing more invasive.
  device_id text,
  device_activated_at timestamptz,
  -- Incremented every time an admin manually frees this license from a
  -- device (api/admin/reset-device.ts) - a quick "has this been reset
  -- before, and how often" signal when looking at a support request.
  device_reset_count integer not null default 0,

  -- Idempotency for Stripe webhook retries - see api/stripe/webhook.ts.
  -- A second delivery of the same checkout.session.completed event
  -- upserts against this unique key and no-ops rather than minting a
  -- second license for one payment.
  stripe_checkout_session_id text unique,
  stripe_payment_intent_id text,

  created_at timestamptz not null default now(),
  activated_at timestamptz,
  last_validated_at timestamptz
);

create index if not exists licenses_email_idx on licenses (email);
create index if not exists licenses_status_idx on licenses (status);
create index if not exists licenses_license_key_idx on licenses (license_key);

-- Row-level security is intentionally left OFF: every read/write in this
-- project goes through the service-role key from server-side Vercel
-- functions only (see lib/supabase.ts) - there is no anon/browser client
-- anywhere in this system to write a policy for. If that ever changes,
-- enable RLS and write policies before introducing any client that uses
-- the anon key.
