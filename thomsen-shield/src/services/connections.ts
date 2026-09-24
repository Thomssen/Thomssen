import { invoke } from "./ipc";
import type { LiveConnection } from "@/types";

export function getConnections(): Promise<LiveConnection[]> {
  return invoke<LiveConnection[]>("get_connections");
}

export function blockConnectionIp(ip: string): Promise<void> {
  return invoke<void>("block_connection_ip", { ip });
}

export function blockConnectionApp(path: string): Promise<void> {
  return invoke<void>("block_connection_app", { path });
}
