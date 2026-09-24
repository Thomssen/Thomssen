import { invoke } from "./ipc";
import type { BlockedEntry } from "@/types";

export function listBlocklist(): Promise<BlockedEntry[]> {
  return invoke<BlockedEntry[]>("list_blocklist");
}

export function addManualBlock(ip: string, note?: string): Promise<BlockedEntry> {
  return invoke<BlockedEntry>("add_manual_block", { ip, note });
}

export function unblockEntry(id: string): Promise<void> {
  return invoke<void>("unblock_entry", { id });
}

export function makeBlockPermanent(id: string): Promise<void> {
  return invoke<void>("make_block_permanent", { id });
}

export function clearBlockHistory(): Promise<void> {
  return invoke<void>("clear_block_history");
}
