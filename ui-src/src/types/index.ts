export interface Sound {
  id: number;
  name: string;
  modifiedDate: number;
  path: string;
  hotkeys: number[];
  animationEvent?: (event: AnimationEvent) => void;
  hotkeySequence: string;
  isFavorite: boolean;
  localVolume: number | null;
  remoteVolume: number | null;
  repeat: boolean | null;
  lengthInMs: number | null;
  tags: string[];
  // Audio file tags — null = not yet read, '' = read but absent, string = value
  artist?: string | null;
  album?: string | null;
  year?: string | null;
  trackNumber?: string | null;
  discNumber?: string | null;
}

export interface Thumbnail {
  height: number;
  url: string;
  width: number;
  resolution: string;
  id: string;
}

export interface YoutubeDlInfo {
  title: string;
  uploader?: string;
  thumbnails?: Thumbnail[];
}

export interface PlayingSound {
  id: number;
  lengthInMs: number;
  readInMs: number;
  paused: boolean;
  repeat: boolean;
  sound: Sound;
}

export interface Tab {
  id: number;
  name: string;
  cover: string;
  sounds: Sound[];
  sortMode: SortMode;
  zoom: number | null;
  viewMode: BoardViewMode | null;
  importedFolders: string[];
  tags: string[];
}

export interface UpdateData {
  current: string;
  latest: string;
  outdated: boolean;
}

export interface Output {
  name: string;
  application: string;
  appIcon?: string; // Base64 image
}

export type Playing = PlayingSound | Output;

export enum PlaylistMode {
  Off,
  On,
  Shuffle,
}

export enum AudioBackend {
  None,
  PipeWire,
  PulseAudio,
}

export enum SortMode {
  ModifiedDate_Ascending,
  ModifiedDate_Descending,
  Alphabetical_Ascending,
  Alphabetical_Descending,
}

export enum Theme {
  System,
  Dark,
  Light,
}

export enum ViewMode {
  List,
  Grid,
  EmulatedLaunchpad,
}

/** How a soundboard's sounds are displayed — per-tab (see `Tab.viewMode`), distinct from the
 * unused global `ViewMode` above. */
export enum BoardViewMode {
  Cards,
  List,
  Album,
}

/**
 * Windows recording device
 */
export interface RecordingDevice {
  name: string;
  guid: string;
}

export interface Settings {
  outputs: string[];
  selectedTab: number;
  allowOverlapping: boolean;
  deleteToTrash: boolean;
  syncVolumes: boolean;
  theme: Theme;
  audioBackend: AudioBackend;
  language: string | null;
  viewMode: ViewMode;
  stopHotkey: number[];
  pushToTalkKeys: number[];
  tabHotkeysOnly: boolean;
  minimizeToTray: boolean;
  localVolume: number;
  remoteVolume: number;
  allowMultipleOutputs: boolean;
  useAsDefaultDevice: boolean;
  muteDuringPlayback: boolean;
  launchOnStartup: boolean;
  gridZoom: number;
  defaultLoop: boolean;
  normalize: boolean;
  normalizeStrength: number;
}
