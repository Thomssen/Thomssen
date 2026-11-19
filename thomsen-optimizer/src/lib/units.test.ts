import { describe, expect, it } from "vitest";
import { formatBytes1024, formatChange, formatDuration, formatMhz, formatTemp, fpsText } from "./units";

describe("units", () => {
  it("formats bytes with binary units", () => {
    expect(formatBytes1024(0)).toBe("0 B");
    expect(formatBytes1024(1536)).toBe("1.5 KB");
    expect(formatBytes1024(16 * 1024 ** 3)).toBe("16.0 GB");
    expect(formatBytes1024(null)).toBe("-");
  });
  it("never invents numbers for missing data", () => {
    expect(formatTemp(null)).toBe("Unavailable");
    expect(fpsText(null)).toBe("Unavailable");
    expect(formatChange(null)).toBe("n/a");
    expect(formatChange(Number.NaN)).toBe("n/a");
  });
  it("signs changes", () => {
    expect(formatChange(12.34)).toBe("+12.3%");
    expect(formatChange(-3)).toBe("-3.0%");
  });
  it("formats durations and clocks", () => {
    expect(formatDuration(59)).toBe("59s");
    expect(formatDuration(3700)).toBe("1h 1m");
    expect(formatDuration(90000)).toBe("1d 1h 0m");
    expect(formatMhz(3600)).toBe("3.60 GHz");
    expect(formatMhz(800)).toBe("800 MHz");
  });
});
