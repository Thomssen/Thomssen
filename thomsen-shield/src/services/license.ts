import { invoke } from "./ipc";
import type { LicenseInfo } from "@/types";

export function getLicenseInfo(): Promise<LicenseInfo> {
  return invoke<LicenseInfo>("get_license_info");
}

export function activateLicense(key: string): Promise<LicenseInfo> {
  return invoke<LicenseInfo>("activate_license", { key });
}

export function deactivateLicense(): Promise<void> {
  return invoke<void>("deactivate_license");
}
