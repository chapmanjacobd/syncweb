export interface Folder {
    id: string;
}

export interface Device {
    id: string;
    name: string;
    paused: boolean;
}

export interface Mount {
    name: string;
    mountpoints: string[];
    size: string;
    type: string;
    fstype?: string;
    label?: string;
    children?: Mount[];
}

export interface FileItem {
    name: string;
    path: string;
    is_dir: boolean;
    local: boolean;
    size: number;
    modified?: string;
    type?: string;
    permission?: string;
}

export interface EventItem {
    time: string;
    type: string;
    message: string;
}

export interface CompletionData {
    folderId: string;
    deviceId: string;
    completion_pct: number;
    need_bytes: number;
    global_bytes: number;
    need_items: number;
    global_items: number;
}

export interface TreeEntry {
    name: string;
    type: string;
    size: number;
}

export interface LocalChangedData {
    files: FileItem[];
    page: number;
    perPage: number;
}

export interface NeedData {
    remote: FileItem[];
    local: FileItem[];
    queued: FileItem[];
    page: number;
    perPage: number;
}

export interface RemoteNeedData {
    files: FileItem[];
    page: number;
    perPage: number;
}

export interface State {
    folders: Folder[];
    devices: Device[];
    pendingDevices: Record<string, string>;
    pendingFolders: Record<string, { offeredBy: Record<string, any> }>;
    mounts: Mount[];
    currentFolder: string | null;
    currentPath: string;
    files: FileItem[];
    token: string;
    selectedItems: string[];
    events: EventItem[];
    sortBy: string;
    currentView: string;
    needTab: string;
    completionData: CompletionData[];
    treeData: TreeEntry[];
    localChangedData: LocalChangedData;
    needData: NeedData;
    remoteNeedData: RemoteNeedData;
    // Pagination and Layout
    filesPage: number;
    filesPerPage: number;
    isActivityOpen: boolean;
}
