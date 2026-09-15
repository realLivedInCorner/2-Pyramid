export type VersionStatus = "latest" | "stable" | "beta";

export type VersionEra =
  | "classic"      // 1.6 – 1.12
  | "modern"       // 1.13 – 1.16
  | "cavesCliffs"  // 1.17 – 1.19
  | "trailsTales"  // 1.20
  | "trickyTrials" // 1.21
  | "bravery"      // 26.1+
  | "bedrock";     // Bedrock Latest

export interface VersionEntry {
  /** 唯一 key，与后端 versionMap 对齐 */
  label: string;
  /** 显示的版本区间，如 "1.6 → 1.8" */
  range: string;
  packFormat: number;
  era: VersionEra;
  status?: VersionStatus;
}

export const MINECRAFT_VERSIONS: VersionEntry[] = [
  { label: "1.6-1.8",         range: "1.6 → 1.8",         packFormat: 1,  era: "classic" },
  { label: "1.9-1.10",        range: "1.9 → 1.10",        packFormat: 2,  era: "classic" },
  { label: "1.11-1.12",       range: "1.11 → 1.12",       packFormat: 3,  era: "classic" },
  { label: "1.13-1.14",       range: "1.13 → 1.14",       packFormat: 4,  era: "modern" },
  { label: "1.15-1.16.1",     range: "1.15 → 1.16.1",     packFormat: 5,  era: "modern" },
  { label: "1.16.2-1.16.5",   range: "1.16.2 → 1.16.5",   packFormat: 6,  era: "modern" },
  { label: "1.17",            range: "1.17",              packFormat: 7,  era: "cavesCliffs" },
  { label: "1.18",            range: "1.18",              packFormat: 8,  era: "cavesCliffs" },
  { label: "1.19-1.19.2",     range: "1.19 → 1.19.2",     packFormat: 9,  era: "cavesCliffs" },
  { label: "1.19.3",          range: "1.19.3",            packFormat: 12, era: "cavesCliffs" },
  { label: "1.19.4",          range: "1.19.4",            packFormat: 13, era: "cavesCliffs" },
  { label: "1.20-1.20.1",     range: "1.20 → 1.20.1",     packFormat: 15, era: "trailsTales" },
  { label: "1.20.2",          range: "1.20.2",            packFormat: 18, era: "trailsTales" },
  { label: "1.20.3-1.20.4",   range: "1.20.3 → 1.20.4",   packFormat: 22, era: "trailsTales" },
  { label: "1.20.5-1.20.6",   range: "1.20.5 → 1.20.6",   packFormat: 32, era: "trailsTales" },
  { label: "1.21-1.21.1",     range: "1.21 → 1.21.1",     packFormat: 34, era: "trickyTrials" },
  { label: "1.21.2-1.21.3",   range: "1.21.2 → 1.21.3",   packFormat: 42, era: "trickyTrials" },
  { label: "1.21.4",          range: "1.21.4",            packFormat: 46, era: "trickyTrials" },
  { label: "1.21.5",          range: "1.21.5",            packFormat: 55, era: "trickyTrials" },
  { label: "1.21.6",          range: "1.21.6",            packFormat: 63, era: "trickyTrials" },
  { label: "1.21.7-1.21.8",   range: "1.21.7 → 1.21.8",   packFormat: 64, era: "trickyTrials" },
  { label: "1.21.9-1.21.10",  range: "1.21.9 → 1.21.10",  packFormat: 69, era: "trickyTrials" },
  { label: "1.21.11",         range: "1.21.11",           packFormat: 75, era: "trickyTrials", status: "stable" },
  { label: "26.1-26.1.2",     range: "26.1 → 26.1.2",     packFormat: 84, era: "bravery",    status: "latest" },
  { label: "26.2",            range: "26.2",              packFormat: 88, era: "bravery",    status: "latest" },
  { label: "Bedrock Latest",  range: "Bedrock",           packFormat: 1000, era: "bedrock",  status: "beta" },
];

export const ERA_ORDER: VersionEra[] = [
  "classic",
  "modern",
  "cavesCliffs",
  "trailsTales",
  "trickyTrials",
  "bravery",
  "bedrock",
];

export const DEFAULT_VERSION_LABEL = "1.21-1.21.1";

export function groupVersionsByEra(versions: VersionEntry[]) {
  const groups: Record<VersionEra, VersionEntry[]> = {
    classic: [],
    modern: [],
    cavesCliffs: [],
    trailsTales: [],
    trickyTrials: [],
    bravery: [],
    bedrock: [],
  };
  for (const v of versions) groups[v.era].push(v);
  return ERA_ORDER.filter((era) => groups[era].length > 0).map((era) => ({
    era,
    items: groups[era],
  }));
}
