import { State, Folder, Device, Mount, FileItem, EventItem, CompletionData, TreeEntry, LocalChangedData, NeedData, RemoteNeedData } from './types';

export const state: State = {
    folders: [],
    devices: [],
    pendingDevices: {},
    pendingFolders: {},
    mounts: [],
    currentFolder: null,
    currentPath: '/',
    files: [],
    token: '',
    selectedItems: [],
    events: [],
    sortBy: 'name',
    // New state for sync monitoring views
    currentView: 'files',
    needTab: 'remote',
    completionData: [],
    treeData: [],
    localChangedData: { files: [], page: 1, perPage: 100 },
    needData: { remote: [], local: [], queued: [], page: 1, perPage: 100 },
    remoteNeedData: { files: [], page: 1, perPage: 100 }
};

// Initialize token from localStorage or URL if in browser
if (typeof window !== 'undefined' && window.localStorage && window.location) {
    const urlParams = new URLSearchParams(window.location.search);
    state.token = urlParams.get('token') || localStorage.getItem('syncweb_token') || '';
    if (state.token) localStorage.setItem('syncweb_token', state.token);
}

export async function fetchAPI(url: string, options: RequestInit = {}): Promise<Response> {
    const headers: HeadersInit = {
        'X-Syncweb-Token': state.token,
        'Content-Type': 'application/json',
        ...(options.headers || {})
    };
    const resp = await fetch(url, { ...options, headers });
    if (resp.status === 401) {
        const newToken = prompt("Unauthorized. Enter API Token:") || '';
        if (newToken) {
            state.token = newToken;
            if (typeof window !== 'undefined' && window.localStorage) {
                localStorage.setItem('syncweb_token', newToken);
            }
            return fetchAPI(url, options);
        }
    }
    return resp;
}

export function logout(): void {
    state.token = '';
    if (typeof window !== 'undefined' && window.localStorage) {
        localStorage.removeItem('syncweb_token');
    }
    location.reload();
}

export function toggleSidebar(): void {
    const sidebar = document.querySelector('aside');
    sidebar?.classList.toggle('open');
}

export async function loadFolders(): Promise<void> {
    try {
        const resp = await fetchAPI('/api/syncweb/folders');
        state.folders = await resp.json();
        renderFolders();

        // Load pending folders
        const pendingResp = await fetchAPI('/api/syncweb/pending-folders');
        state.pendingFolders = await pendingResp.json();
        renderPendingFolders();
    } catch (e) {
        showToast("Failed to load folders", true);
    }
}

export async function loadPendingFolders(): Promise<void> {
    try {
        const resp = await fetchAPI('/api/syncweb/pending-folders');
        state.pendingFolders = await resp.json();
        renderPendingFolders();
    } catch (e) {
        showToast("Failed to load pending folders", true);
    }
}

export function renderPendingFolders(): void {
    const container = document.getElementById('pending-folder-container');
    const list = document.getElementById('pending-folder-list');
    if (!list || !container) return;

    const folderIds = Object.keys(state.pendingFolders);
    if (folderIds.length === 0) {
        container.style.display = 'none';
        return;
    }

    container.style.display = 'block';
    list.innerHTML = '';

    folderIds.forEach(folderId => {
        const info = state.pendingFolders[folderId];
        const offeredBy = info.offeredBy || {};
        const deviceIds = Object.keys(offeredBy);

        const li = document.createElement('li');
        li.className = 'folder-item';
        li.style.color = 'var(--accent-color)';
        li.innerHTML = `<span class="icon"><i data-lucide="inbox"></i></span> ${folderId} <span class="secondary-info">${deviceIds.length} peer(s)</span>`;
        li.title = `Click to join folder: ${folderId}`;
        li.onclick = () => joinFolder(folderId, deviceIds[0]);
        list.appendChild(li);
    });

    if ((window as any).lucide) (window as any).lucide.createIcons();
}

export async function loadDevices(): Promise<void> {
    try {
        const resp = await fetchAPI('/api/syncweb/devices');
        state.devices = await resp.json();

        const pendingResp = await fetchAPI('/api/syncweb/pending');
        state.pendingDevices = await pendingResp.json();

        renderDevices();
    } catch (e) {
        showToast("Failed to load devices", true);
    }
}

export function renderDevices(): void {
    const list = document.getElementById('device-list');
    if (!list) return;
    list.innerHTML = '';

    // Render pending devices first
    Object.keys(state.pendingDevices).forEach(id => {
        const li = document.createElement('li');
        li.className = 'folder-item';
        li.style.color = 'var(--accent-color)';
        li.innerHTML = `<span class="icon"><i data-lucide="bell"></i></span> Pending: ${id.substring(0, 7)}...`;
        li.title = `Click to accept device: ${id}`;
        li.onclick = () => addDevice(id);
        list.appendChild(li);
    });

    state.devices.forEach(d => {
        const li = document.createElement('li');
        li.className = 'folder-item';
        const statusIcon = d.paused ? 'pause-circle' : 'monitor';
        li.innerHTML = `<span class="icon"><i data-lucide="${statusIcon}"></i></span> ${d.name || d.id.substring(0, 7) + '...'}`;
        li.title = d.id;
        li.oncontextmenu = (e) => {
            e.preventDefault();
            if (confirm(`Delete device ${d.name || d.id}?`)) {
                deleteDevice(d.id);
            }
        };
        list.appendChild(li);
    });
    if ((window as any).lucide) (window as any).lucide.createIcons();
}

export async function addDevice(suggestedId: string = ''): Promise<void> {
    const id = suggestedId || prompt("Enter Device ID:", suggestedId) || '';
    if (!id) return;
    const name = prompt("Enter Device Name (optional):", "") || '';

    try {
        const resp = await fetchAPI('/api/syncweb/devices/add', {
            method: 'POST',
            body: JSON.stringify({ id, name, introducer: false })
        });
        if (resp.ok) {
            showToast("Device added");
            loadDevices();
        } else {
            const data = await resp.json();
            showToast(data.error || "Failed to add device", true);
        }
    } catch (e) {
        showToast("Error adding device", true);
    }
}

export async function deleteDevice(id: string): Promise<void> {
    try {
        const resp = await fetchAPI('/api/syncweb/devices/delete', {
            method: 'POST',
            body: JSON.stringify({ id })
        });
        if (resp.ok) {
            showToast("Device deleted");
            loadDevices();
        } else {
            showToast("Failed to delete device", true);
        }
    } catch (e) {
        showToast("Error deleting device", true);
    }
}

export function renderFolders(): void {
    const list = document.getElementById('folder-list');
    if (!list) return;
    list.innerHTML = '';

    const rootLi = document.createElement('li');
    rootLi.className = 'folder-item' + (state.currentFolder === null ? ' active' : '');
    rootLi.innerHTML = `<span class="icon"><i data-lucide="home"></i></span> [Root]`;
    rootLi.onclick = () => selectFolder(null);
    list.appendChild(rootLi);

    state.folders.forEach(f => {
        const li = document.createElement('li');
        li.className = 'folder-item' + (state.currentFolder === f.id ? ' active' : '');
        li.innerHTML = `<span class="icon"><i data-lucide="folder"></i></span> ${f.id}`;
        li.onclick = () => selectFolder(f.id);
        li.oncontextmenu = (e) => {
            e.preventDefault();
            if (confirm(`Delete folder ${f.id}?`)) {
                deleteFolder(f.id);
            }
        };
        list.appendChild(li);
    });
    if ((window as any).lucide) (window as any).lucide.createIcons();
}

export async function addFolder(): Promise<void> {
    document.getElementById('add-folder-ui')!.style.display = 'block';
    document.getElementById('path-preview')!.innerHTML = '';
    document.getElementById('new-folder-id')!.nodeValue = '';
    document.getElementById('new-folder-path')!.nodeValue = '';
}

export async function joinFolder(folderId: string, deviceId: string = ''): Promise<void> {
    const defaultPath = prompt(`Enter local path for folder "${folderId}":`, `/home/user/Syncweb/${folderId}`) || '';
    if (!defaultPath) return;

    try {
        const resp = await fetchAPI('/api/syncweb/folders/join', {
            method: 'POST',
            body: JSON.stringify({ folder_id: folderId, device_id: deviceId, path: defaultPath })
        });
        if (resp.ok) {
            showToast(`Joined folder: ${folderId}`);
            loadFolders();
        } else {
            const data = await resp.json();
            showToast(data.error || "Failed to join folder", true);
        }
    } catch (e) {
        showToast("Error joining folder", true);
    }
}

export async function previewLocalPath(): Promise<void> {
    const path = (document.getElementById('new-folder-path') as HTMLInputElement).value;
    if (!path) return;

    try {
        const resp = await fetchAPI(`/api/local/ls?path=${encodeURIComponent(path)}`);
        if (resp.ok) {
            const files = await resp.json();
            let html = '<strong>Contents:</strong><div style="margin-top: 0.5rem;">';
            files.slice(0, 5).forEach((f: any) => {
                const icon = f.is_dir ? 'folder' : 'file-text';
                html += `<div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.2rem;"><i data-lucide="${icon}" style="width: 14px; height: 14px;"></i> ${f.name}</div>`;
            });
            if (files.length > 5) html += `<div style="font-size: 0.8rem; margin-top: 0.2rem; color: var(--secondary-text);">... and ${files.length - 5} more</div>`;
            html += '</div>';
            document.getElementById('path-preview')!.innerHTML = html;
            if ((window as any).lucide) (window as any).lucide.createIcons();
        } else {
            document.getElementById('path-preview')!.innerHTML = '<span style="color: #ff4444;">Path not found or inaccessible</span>';
        }
    } catch (e) {
        document.getElementById('path-preview')!.innerHTML = '<span style="color: #ff4444;">Error accessing path</span>';
    }
}

export async function confirmAddFolder(): Promise<void> {
    const id = (document.getElementById('new-folder-id') as HTMLInputElement).value;
    const path = (document.getElementById('new-folder-path') as HTMLInputElement).value;

    if (!id || !path) {
        showToast("ID and Path required", true);
        return;
    }

    try {
        const resp = await fetchAPI('/api/syncweb/folders/add', {
            method: 'POST',
            body: JSON.stringify({ id, path })
        });
        if (resp.ok) {
            showToast("Folder added");
            document.getElementById('add-folder-ui')!.style.display = 'none';
            loadFolders();
        } else {
            const data = await resp.json();
            showToast(data.error || "Failed to add folder", true);
        }
    } catch (e) {
        showToast("Error adding folder", true);
    }
}

export async function deleteFolder(id: string): Promise<void> {
    try {
        const resp = await fetchAPI('/api/syncweb/folders/delete', {
            method: 'POST',
            body: JSON.stringify({ id })
        });
        if (resp.ok) {
            showToast("Folder deleted");
            if (state.currentFolder === id) selectFolder(null);
            loadFolders();
        } else {
            showToast("Failed to delete folder", true);
        }
    } catch (e) {
        showToast("Error deleting folder", true);
    }
}

export async function loadMounts(): Promise<void> {
    try {
        const resp = await fetchAPI('/api/mounts');
        state.mounts = await resp.json();
        renderMounts();
    } catch (e) {
        showToast("Failed to load mounts", true);
    }
}

export function renderMounts(): void {
    const list = document.getElementById('mount-list');
    if (!list) return;
    list.innerHTML = '';

    const flatten = (devices: Mount[]) => {
        devices.forEach(d => {
            if (d.mountpoints && d.mountpoints.length > 0) {
                d.mountpoints.forEach(mp => {
                    if (mp && !mp.startsWith('[')) {
                        const li = document.createElement('li');
                        li.className = 'folder-item';
                        li.innerHTML = `<span class="icon"><i data-lucide="hard-drive"></i></span> ${mp} <span class="secondary-info">${d.size}</span>`;
                        li.title = `${d.name} - ${d.label || 'no label'}`;
                        li.oncontextmenu = (e) => {
                            e.preventDefault();
                            if (confirm(`Unmount ${mp}?`)) {
                                unmountPoint(mp);
                            }
                        };
                        list.appendChild(li);
                    }
                });
            } else if (d.fstype && d.type === 'part') {
                const li = document.createElement('li');
                li.className = 'folder-item';
                li.innerHTML = `<span class="icon"><i data-lucide="plug-zap"></i></span> ${d.name} <span class="secondary-info">[Unmounted]</span>`;
                li.style.opacity = '0.6';
                li.onclick = () => {
                    const mp = prompt("Enter mountpoint path:", `/mnt/${d.label || d.name}`) || '';
                    if (mp) mountDevice(`/dev/${d.name}`, mp);
                };
                list.appendChild(li);
            }

            if (d.children) flatten(d.children);
        });
    };

    flatten(state.mounts);
    if ((window as any).lucide) (window as any).lucide.createIcons();
}

export async function mountDevice(device: string, mountpoint: string): Promise<void> {
    try {
        const resp = await fetchAPI('/api/mount', {
            method: 'POST',
            body: JSON.stringify({ device, mountpoint })
        });
        if (resp.ok) {
            showToast("Mounted successfully");
            loadMounts();
        } else {
            showToast("Mount failed", true);
        }
    } catch (e) {
        showToast("Error mounting", true);
    }
}

export async function unmountPoint(mountpoint: string): Promise<void> {
    try {
        const resp = await fetchAPI('/api/unmount', {
            method: 'POST',
            body: JSON.stringify({ mountpoint })
        });
        if (resp.ok) {
            showToast("Unmounted successfully");
            loadMounts();
        } else {
            showToast("Unmount failed", true);
        }
    } catch (e) {
        showToast("Error unmounting", true);
    }
}

export async function selectFolder(id: string | null): Promise<void> {
    if (id === null) {
        state.currentFolder = null;
        state.currentPath = "/";
    } else {
        state.currentFolder = id;
        state.currentPath = `sync://${id}/`;
    }
    renderFolders();
    loadFiles();
}

export async function loadFiles(): Promise<void> {
    try {
        let url: string;
        if (!state.currentFolder) {
            url = '/api/syncweb/ls';
        } else {
            const prefix = state.currentPath.split('/').slice(3).join('/');
            url = `/api/syncweb/ls?folder=${encodeURIComponent(state.currentFolder)}&prefix=${encodeURIComponent(prefix)}`;
        }
        const resp = await fetchAPI(url);
        state.files = await resp.json();
        state.selectedItems = []; // Clear selection on reload
        updateBulkActions();
        renderFiles();
    } catch (e) {
        showToast("Failed to load files", true);
    }
}

export async function searchFiles(): Promise<void> {
    const query = (document.getElementById('search-input') as HTMLInputElement)?.value;
    if (!query) {
        loadFiles();
        return;
    }
    try {
        const resp = await fetchAPI(`/api/syncweb/find?q=${encodeURIComponent(query)}`);
        state.files = await resp.json();
        state.selectedItems = []; // Clear selection on reload
        updateBulkActions();
        state.currentPath = `Search results for "${query}"`;
        renderFiles(true);
    } catch (e) {
        showToast("Search failed", true);
    }
}

export function renderFiles(isSearch: boolean = false): void {
    const list = document.getElementById('file-list');
    const pathHeader = document.getElementById('current-path');
    if (!list || !pathHeader) return;

    pathHeader.textContent = state.currentPath;
    list.innerHTML = '';

    // Sort files
    const sortedFiles = [...state.files].sort((a, b) => {
        if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;

        switch (state.sortBy) {
            case 'size':
                return b.size - a.size;
            case 'date':
                return new Date(b.modified || '0').getTime() - new Date(a.modified || '0').getTime();
            case 'name':
            default:
                return a.name.localeCompare(b.name);
        }
    });

    if (!isSearch) {
        // Parent dir
        if (state.currentFolder && state.currentPath !== `sync://${state.currentFolder}/`) {
            const li = document.createElement('li');
            li.className = 'file-item';
            li.innerHTML = `<span class="icon"><i data-lucide="arrow-up"></i></span> ..`;
            li.onclick = goUp;
            list.appendChild(li);
        } else if (state.currentFolder && state.currentPath === `sync://${state.currentFolder}/`) {
            const li = document.createElement('li');
            li.className = 'file-item';
            li.innerHTML = `<span class="icon"><i data-lucide="arrow-up"></i></span> [Root]`;
            li.onclick = () => selectFolder(null);
            list.appendChild(li);
        }
    }

    sortedFiles.forEach(f => {
        const li = document.createElement('li');
        li.className = 'file-item';
        if (f.is_dir) li.classList.add('is-dir');
        if (state.selectedItems.includes(f.path)) li.classList.add('selected');
        li.draggable = true;
        const displayName = isSearch ? f.path : f.name;
        const icon = f.is_dir ? 'folder' : 'file';
        const cloudIcon = !f.local && !f.is_dir ? ' <span class="icon" style="display:inline-block; margin-left: 0.5rem;"><i data-lucide="cloud"></i></span>' : '';
        const sizeInfo = f.is_dir ? '' : `<span class="secondary-info">${formatSize(f.size)}</span>`;

        const checkbox = document.createElement('input');
        checkbox.type = 'checkbox';
        checkbox.checked = state.selectedItems.includes(f.path);
        checkbox.style.marginRight = '0.5rem';
        checkbox.onclick = (e) => {
            e.stopPropagation();
            toggleSelection(f.path);
        };

        li.innerHTML = `<span class="icon"><i data-lucide="${icon}"></i></span> <span style="flex: 1">${displayName}${cloudIcon}</span> ${sizeInfo}`;
        li.prepend(checkbox);

        li.onclick = () => {
            if (!state.currentFolder && f.is_dir) {
                // Extract folder ID from path sync://id/ or syncweb://id/
                const parts = f.path.split('/');
                const folderID = parts[0].startsWith('syncweb:') ? parts[2] : parts[2]; // Both are at index 2
                selectFolder(folderID);
                return;
            }

            if (f.is_dir) {
                state.currentPath = f.path + (f.path.endsWith('/') ? '' : '/');
                loadFiles();
            } else if (!f.local) {
                triggerDownload(f.path);
            }
        };

        li.oncontextmenu = (e) => {
            e.preventDefault();
            showFileProperties(f.path);
        };

        // Drag and Drop
        li.ondragstart = (e) => {
            e.dataTransfer!.setData('text/plain', f.path);
            list.classList.add('dragging-active');
        };
        li.ondragend = () => list.classList.remove('dragging-active');

        if (f.is_dir) {
            li.ondragover = (e) => {
                e.preventDefault();
                li.classList.add('drag-over');
            };
            li.ondragleave = () => li.classList.remove('drag-over');
            li.ondrop = async (e) => {
                e.preventDefault();
                li.classList.remove('drag-over');
                const srcPath = e.dataTransfer!.getData('text/plain');
                if (srcPath === f.path) return;
                const dstPath = f.path + '/' + srcPath.split('/').pop();
                moveFile(srcPath, dstPath);
            };
        }

        list.appendChild(li);
    });
    if ((window as any).lucide) (window as any).lucide.createIcons();
}

export function goUp(): void {
    const parts = state.currentPath.split('/');
    parts.pop(); // Remove trailing empty string
    parts.pop(); // Remove last dir
    state.currentPath = parts.join('/') + '/';
    loadFiles();
}

export async function moveFile(src: string, dst: string): Promise<void> {
    try {
        const resp = await fetchAPI('/api/file/move', {
            method: 'POST',
            body: JSON.stringify({ src, dst })
        });
        if (resp.ok) {
            showToast("Moved successfully");
            loadFiles();
        } else {
            showToast("Move failed", true);
        }
    } catch (e) {
        showToast("Move error", true);
    }
}

export async function showFileProperties(path: string): Promise<void> {
    try {
        const resp = await fetchAPI(`/api/syncweb/stat?path=${encodeURIComponent(path)}`);
        const data = await resp.json();
        if (resp.ok) {
            alert(`File: ${data.name}\nPath: ${data.path}\nSize: ${(data.size / 1024 / 1024).toFixed(2)} MB\nModified: ${new Date(data.modified).toLocaleString()}\nLocal: ${data.local ? 'Yes' : 'No'}`);
        } else {
            showToast(data.error || "Failed to fetch properties", true);
        }
    } catch (e) {
        showToast("Error fetching properties", true);
    }
}

export async function triggerDownload(path: string): Promise<void> {
    try {
        const resp = await fetchAPI('/api/syncweb/download', {
            method: 'POST',
            body: JSON.stringify({ path })
        });
        if (resp.ok) showToast("Download triggered");
    } catch (e) {}
}

export async function toggleOffline(): Promise<void> {
    const btn = document.getElementById('offline-btn');
    const span = btn?.querySelector('span');
    const isOffline = span?.innerText === 'Go Online';
    try {
        const resp = await fetchAPI('/api/syncweb/toggle', {
            method: 'POST',
            body: JSON.stringify({ offline: !isOffline })
        });
        const data = await resp.json();
        if (span) span.innerText = data.offline ? 'Go Online' : 'Go Offline';
        const icon = btn!.querySelector('i');
        if (icon) icon.setAttribute('data-lucide', data.offline ? 'power-off' : 'power');
        if ((window as any).lucide) (window as any).lucide.createIcons();
        showToast(data.offline ? "Backend Stopped" : "Backend Started");
    } catch (e) {
        showToast("Toggle failed", true);
    }
}

export async function loadStatus(): Promise<void> {
    try {
        const resp = await fetchAPI('/api/syncweb/status');
        const data = await resp.json();
        const btn = document.getElementById('offline-btn');
        if (btn) {
            const span = btn.querySelector('span');
            if (span) span.innerText = data.offline ? 'Go Online' : 'Go Offline';
            const icon = btn.querySelector('i');
            if (icon) icon.setAttribute('data-lucide', data.offline ? 'power-off' : 'power');
            if ((window as any).lucide) (window as any).lucide.createIcons();
        }
    } catch (e) {}
}

export function showToast(message: string, isError: boolean = false): void {
    const t = document.getElementById('toast');
    if (!t) return;
    t.textContent = message;
    t.style.background = isError ? '#ff4444' : 'var(--accent-color)';
    t.style.color = isError ? '#fff' : 'var(--bg-color)';
    t.style.display = 'block';

    // Clear existing timeout if any
    const toastEl = t as any;
    if (toastEl.timeout) clearTimeout(toastEl.timeout);

    toastEl.timeout = setTimeout(() => {
        t.style.display = 'none';
    }, 4000);
}

export function toggleSelection(path: string): void {
    const idx = state.selectedItems.indexOf(path);
    if (idx === -1) {
        state.selectedItems.push(path);
    } else {
        state.selectedItems.splice(idx, 1);
    }
    updateBulkActions();
    renderFiles(state.currentPath.startsWith('Search results'));
}

export function updateBulkActions(): void {
    const bar = document.getElementById('bulk-actions');
    const count = document.getElementById('selected-count');
    if (!bar || !count) return;

    if (state.selectedItems.length > 0) {
        bar.style.display = 'flex';
        count.textContent = `${state.selectedItems.length} selected`;
    } else {
        bar.style.display = 'none';
    }
}

export function clearSelection(): void {
    state.selectedItems = [];
    updateBulkActions();
    renderFiles(state.currentPath.startsWith('Search results'));
}

export async function bulkDelete(): Promise<void> {
    if (state.selectedItems.length === 0) return;
    if (!confirm(`Delete ${state.selectedItems.length} items?`)) return;

    let success = 0;
    for (const path of state.selectedItems) {
        try {
            const resp = await fetchAPI('/api/file/delete', {
                method: 'POST',
                body: JSON.stringify({ path })
            });
            if (resp.ok) success++;
            else showToast(`Failed to delete ${path}`, true);
        } catch (e) {
            showToast(`Error deleting ${path}`, true);
        }
    }
    showToast(`${success} items deleted`);
    loadFiles();
}

export async function bulkMove(): Promise<void> {
    if (state.selectedItems.length === 0) return;
    const dstFolder = prompt("Enter destination folder ID (or empty for root):", state.currentFolder || "") || '';
    if (dstFolder === null) return;

    let dstPath: string;
    if (!dstFolder) {
        dstPath = "/";
    } else {
        dstPath = `sync://${dstFolder}/`;
    }

    const subPath = prompt("Enter subpath within destination (e.g. 'Photos/2023'):", "") || '';
    if (subPath === null) return;
    if (subPath) {
        dstPath += subPath.endsWith('/') ? subPath : subPath + '/';
    }

    let success = 0;
    for (const src of state.selectedItems) {
        const name = src.split('/').pop()!;
        const dst = dstPath + name;
        try {
            const resp = await fetchAPI('/api/file/move', {
                method: 'POST',
                body: JSON.stringify({ src, dst })
            });
            if (resp.ok) success++;
            else showToast(`Failed to move ${src}`, true);
        } catch (e) {
            showToast(`Error moving ${src}`, true);
        }
    }
    showToast(`${success} items moved`);
    loadFiles();
}

export async function bulkCopy(): Promise<void> {
    if (state.selectedItems.length === 0) return;
    const dstFolder = prompt("Enter destination folder ID (or empty for root):", state.currentFolder || "") || '';
    if (dstFolder === null) return;

    let dstPath: string;
    if (!dstFolder) {
        dstPath = "/";
    } else {
        dstPath = `sync://${dstFolder}/`;
    }

    const subPath = prompt("Enter subpath within destination (e.g. 'Photos/2023'):", "") || '';
    if (subPath === null) return;
    if (subPath) {
        dstPath += subPath.endsWith('/') ? subPath : subPath + '/';
    }

    let success = 0;
    for (const src of state.selectedItems) {
        const name = src.split('/').pop()!;
        const dst = dstPath + name;
        try {
            const resp = await fetchAPI('/api/file/copy', {
                method: 'POST',
                body: JSON.stringify({ src, dst })
            });
            if (resp.ok) success++;
            else showToast(`Failed to copy ${src}`, true);
        } catch (e) {
            showToast(`Error copying ${src}`, true);
        }
    }
    showToast(`${success} items copied`);
    loadFiles();
}

export function updateSort(): void {
    state.sortBy = (document.getElementById('sort-select') as HTMLSelectElement).value;
    renderFiles(state.currentPath.startsWith('Search results'));
}

export function formatSize(bytes: number): string {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

export async function loadEvents(): Promise<void> {
    try {
        const resp = await fetchAPI('/api/syncweb/events');
        state.events = await resp.json();
        renderEvents();
    } catch (e) {}
}

export function renderEvents(): void {
    const list = document.getElementById('activity-list');
    if (!list) return;

    // Sort events by time descending
    const sortedEvents = [...state.events].sort((a, b) => new Date(b.time).getTime() - new Date(a.time).getTime());

    list.innerHTML = '';
    sortedEvents.forEach(ev => {
        const li = document.createElement('li');
        li.className = 'folder-item';
        li.style.padding = '0.4rem 0.5rem';
        li.style.fontSize = '0.8rem';

        let icon = 'info';
        if (ev.type === 'ItemFinished') icon = 'check-circle';
        if (ev.type === 'ItemStarted') icon = 'refresh-cw';
        if (ev.type.includes('Rejected')) icon = 'alert-triangle';

        const time = new Date(ev.time).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
        li.innerHTML = `<span class="icon" style="width:14px; height:14px;"><i data-lucide="${icon}" style="width:12px; height:12px;"></i></span>
                        <div style="overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
                            <span style="color: var(--secondary-text)">[${time}]</span> ${ev.message}
                        </div>`;
        list.appendChild(li);
    });
    if ((window as any).lucide) (window as any).lucide.createIcons();
}

export function refresh(): void { 
    loadFolders(); 
    loadDevices(); 
    loadMounts(); 
    loadFiles(); 
    loadStatus(); 
    loadEvents(); 
}

// ============== Sync Monitor View Functions ==============

export function switchView(viewName: string): void {
    state.currentView = viewName;

    // Update tab active state
    document.querySelectorAll('.view-tab').forEach(tab => {
        tab.classList.remove('active');
        if (tab.textContent!.toLowerCase().includes(viewName.replace('-', ' '))) {
            tab.classList.add('active');
        }
    });

    // Update content visibility
    document.querySelectorAll('.view-content').forEach(content => {
        content.classList.remove('active');
    });
    document.getElementById(`view-${viewName}`)?.classList.add('active');

    // Load data for the selected view
    switch (viewName) {
        case 'completion':
            populateFolderDeviceSelects();
            loadCompletion();
            break;
        case 'tree':
            populateFolderSelect('tree-folder-select');
            break;
        case 'local-changed':
            populateFolderSelect('local-changed-folder-select');
            break;
        case 'need':
            populateFolderSelect('need-folder-select');
            loadNeed();
            break;
        case 'remote-need':
            populateFolderSelect('remote-need-folder-select');
            populateDeviceSelect('remote-need-device-select');
            break;
    }

    if ((window as any).lucide) (window as any).lucide.createIcons();
}

export function switchNeedTab(tabName: string): void {
    state.needTab = tabName;
    document.querySelectorAll('#view-need .view-tab').forEach(tab => tab.classList.remove('active'));
    document.getElementById(`need-tab-${tabName}`)?.classList.add('active');
    renderNeedTable();
}

export function populateFolderSelect(elementId: string): void {
    const select = document.getElementById(elementId) as HTMLSelectElement;
    if (!select) return;

    const currentValue = select.value;
    select.innerHTML = '<option value="">Select a folder...</option>';
    state.folders.forEach(f => {
        const option = document.createElement('option');
        option.value = f.id;
        option.textContent = f.id;
        select.appendChild(option);
    });
    if (currentValue && state.folders.some(f => f.id === currentValue)) {
        select.value = currentValue;
    }
}

export function populateDeviceSelect(elementId: string): void {
    const select = document.getElementById(elementId) as HTMLSelectElement;
    if (!select) return;

    const currentValue = select.value;
    select.innerHTML = '<option value="">Select a device...</option>';
    state.devices.forEach(d => {
        const option = document.createElement('option');
        option.value = d.id;
        option.textContent = d.name || d.id.substring(0, 7) + '...';
        select.appendChild(option);
    });
    if (currentValue && state.devices.some(d => d.id === currentValue)) {
        select.value = currentValue;
    }
}

export function populateFolderDeviceSelects(): void {
    populateFolderSelect('completion-folder-select');
    populateDeviceSelect('completion-device-select');
}

// Completion View
export async function loadCompletion(): Promise<void> {
    const folderId = (document.getElementById('completion-folder-select') as HTMLSelectElement)?.value || '';
    const deviceId = (document.getElementById('completion-device-select') as HTMLSelectElement)?.value || '';

    const grid = document.getElementById('completion-grid');
    if (!grid) return;

    grid.innerHTML = '<div style="text-align: center; padding: 2rem;">Loading...</div>';

    try {
        // Get all folders if none selected
        const foldersToCheck = folderId ? [folderId] : state.folders.map(f => f.id);
        const devicesToCheck = deviceId ? [deviceId] : state.devices.map(d => d.id);

        const completionCards: (CompletionData & { folderId: string; deviceId: string })[] = [];

        for (const folder of foldersToCheck) {
            for (const device of devicesToCheck) {
                try {
                    const resp = await fetchAPI(`/api/syncweb/completion?folder_id=${encodeURIComponent(folder)}&device_id=${encodeURIComponent(device)}`);
                    if (resp.ok) {
                        const data = await resp.json();
                        completionCards.push({
                            folderId: folder,
                            deviceId: device,
                            ...data
                        });
                    }
                } catch (e) {
                    // Skip if completion not available for this pair
                }
            }
        }

        if (completionCards.length === 0) {
            grid.innerHTML = '<div style="text-align: center; padding: 2rem; color: var(--secondary-text);">No completion data available</div>';
            return;
        }

        grid.innerHTML = '';
        completionCards.forEach(card => {
            const device = state.devices.find(d => d.id === card.deviceId);
            const deviceName = device?.name || card.deviceId.substring(0, 7) + '...';
            const pct = card.completion_pct || 0;
            const needBytes = card.need_bytes || 0;
            const globalBytes = card.global_bytes || 0;
            const needItems = card.need_items || 0;
            const globalItems = card.global_items || 0;

            const cardEl = document.createElement('div');
            cardEl.className = 'completion-card';
            cardEl.innerHTML = `
                <h4><i data-lucide="folder" style="width: 16px;"></i> ${card.folderId}</h4>
                <div style="font-size: 0.85rem; color: var(--secondary-text); margin-bottom: 0.5rem;">
                    <i data-lucide="laptop" style="width: 12px; display: inline;"></i> ${deviceName}
                </div>
                <div class="completion-pct">${pct.toFixed(1)}%</div>
                <div class="progress-bar">
                    <div class="progress-fill" style="width: ${pct}%"></div>
                </div>
                <div class="progress-stats">
                    <span>${formatSize(needBytes)} needed</span>
                    <span>${needItems} items</span>
                </div>
                <div class="progress-stats">
                    <span>${formatSize(globalBytes)} total</span>
                    <span>${globalItems} items</span>
                </div>
            `;
            grid.appendChild(cardEl);
        });

        if ((window as any).lucide) (window as any).lucide.createIcons();
    } catch (e) {
        grid.innerHTML = '<div style="text-align: center; padding: 2rem; color: #ff4444;">Failed to load completion data</div>';
    }
}

// Tree View
export async function loadTreeView(): Promise<void> {
    const folderId = (document.getElementById('tree-folder-select') as HTMLSelectElement)?.value;
    const prefix = (document.getElementById('tree-prefix') as HTMLInputElement)?.value || '';
    const levels = (document.getElementById('tree-levels') as HTMLInputElement)?.value || '-1';
    const dirsOnly = (document.getElementById('tree-dirs-only') as HTMLInputElement)?.checked || false;

    const treeView = document.getElementById('tree-view');
    if (!treeView) return;

    if (!folderId) {
        treeView.innerHTML = '<div style="text-align: center; padding: 2rem; color: var(--secondary-text);">Select a folder to view tree</div>';
        return;
    }

    treeView.innerHTML = '<div style="text-align: center; padding: 2rem;">Loading...</div>';

    try {
        const url = `/api/syncweb/tree?folder_id=${encodeURIComponent(folderId)}&prefix=${encodeURIComponent(prefix)}&levels=${levels}&dirs_only=${dirsOnly}`;
        const resp = await fetchAPI(url);
        if (!resp.ok) throw new Error('Failed to load tree');

        const data = await resp.json();
        const tree = data.tree || [];

        if (tree.length === 0) {
            treeView.innerHTML = '<div style="text-align: center; padding: 2rem; color: var(--secondary-text);">No entries found</div>';
            return;
        }

        treeView.innerHTML = '';
        const rootNode = buildTreeNodes(tree, 0);
        treeView.appendChild(rootNode);

        if ((window as any).lucide) (window as any).lucide.createIcons();
    } catch (e) {
        treeView.innerHTML = '<div style="text-align: center; padding: 2rem; color: #ff4444;">Failed to load tree view</div>';
    }
}

export function buildTreeNodes(entries: TreeEntry[], depth: number = 0): HTMLElement {
    const container = document.createElement('div');

    entries.forEach(entry => {
        const isDir = entry.type === 'DIRECTORY';
        const itemEl = document.createElement('div');
        itemEl.className = 'tree-item' + (isDir ? ' expanded' : '');

        const toggleEl = document.createElement('span');
        toggleEl.className = 'tree-toggle';
        toggleEl.innerHTML = isDir ? '<i data-lucide="chevron-down"></i>' : '<span style="width: 16px;"></span>';

        const iconEl = document.createElement('span');
        iconEl.innerHTML = `<i data-lucide="${isDir ? 'folder' : 'file'}" style="width: 16px;"></i>`;

        const nameEl = document.createElement('span');
        nameEl.style.flex = '1';
        nameEl.textContent = entry.name;

        const sizeEl = document.createElement('span');
        sizeEl.style.color = 'var(--secondary-text)';
        sizeEl.style.fontSize = '0.85rem';
        sizeEl.textContent = isDir ? '' : formatSize(entry.size);

        itemEl.appendChild(toggleEl);
        itemEl.appendChild(iconEl);
        itemEl.appendChild(nameEl);
        itemEl.appendChild(sizeEl);

        if (isDir) {
            const childrenEl = document.createElement('div');
            childrenEl.className = 'tree-children';
            // Placeholder for children - would need to load on expand
            itemEl.appendChild(childrenEl);

            itemEl.onclick = (e) => {
                e.stopPropagation();
                itemEl.classList.toggle('expanded');
                itemEl.classList.toggle('collapsed');
                toggleEl.innerHTML = itemEl.classList.contains('expanded')
                    ? '<i data-lucide="chevron-down"></i>'
                    : '<i data-lucide="chevron-right"></i>';
                if ((window as any).lucide) (window as any).lucide.createIcons();
            };
        }

        container.appendChild(itemEl);
    });

    return container;
}

// Local Changed View
export async function loadLocalChanged(page: number = 1): Promise<void> {
    const folderId = (document.getElementById('local-changed-folder-select') as HTMLSelectElement)?.value;

    if (!folderId) {
        document.getElementById('local-changed-tbody')!.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: var(--secondary-text);">Select a folder</td></tr>';
        document.getElementById('local-changed-pagination')!.innerHTML = '';
        return;
    }

    const tbody = document.getElementById('local-changed-tbody')!;
    tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem;">Loading...</td></tr>';

    try {
        const url = `/api/syncweb/local-changed?folder_id=${encodeURIComponent(folderId)}&page=${page}&per_page=100`;
        const resp = await fetchAPI(url);
        if (!resp.ok) throw new Error('Failed to load local changed files');

        const data = await resp.json();
        state.localChangedData = {
            files: data.files || [],
            page: data.page || page,
            perPage: data.per_page || 100
        };

        renderLocalChangedTable();
    } catch (e) {
        tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: #ff4444;">Failed to load data</td></tr>';
    }
}

export function renderLocalChangedTable(): void {
    const tbody = document.getElementById('local-changed-tbody')!;
    const pagination = document.getElementById('local-changed-pagination')!;

    if (state.localChangedData.files.length === 0) {
        tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: var(--secondary-text);">No locally changed files</td></tr>';
        pagination.innerHTML = '';
        return;
    }

    tbody.innerHTML = '';
    state.localChangedData.files.forEach(f => {
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td><i data-lucide="${f.type === 'DIRECTORY' ? 'folder' : 'file'}" style="width: 16px; display: inline; margin-right: 0.5rem;"></i> ${f.name}</td>
            <td>${formatSize(f.size)}</td>
            <td>${f.modified ? new Date(f.modified).toLocaleString() : '-'}</td>
            <td>${f.type || 'FILE'}</td>
            <td>${f.permission || '-'}</td>
        `;
        tbody.appendChild(tr);
    });

    renderPagination(pagination, state.localChangedData.page, state.localChangedData.perPage, loadLocalChanged);
    if ((window as any).lucide) (window as any).lucide.createIcons();
}

// Need View
export async function loadNeed(page: number = 1): Promise<void> {
    const folderId = (document.getElementById('need-folder-select') as HTMLSelectElement)?.value;

    if (!folderId) {
        document.getElementById('need-tbody')!.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: var(--secondary-text);">Select a folder</td></tr>';
        document.getElementById('need-pagination')!.innerHTML = '';
        return;
    }

    const tbody = document.getElementById('need-tbody')!;
    tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem;">Loading...</td></tr>';

    try {
        const url = `/api/syncweb/need?folder_id=${encodeURIComponent(folderId)}&page=${page}&per_page=100`;
        const resp = await fetchAPI(url);
        if (!resp.ok) throw new Error('Failed to load need files');

        const data = await resp.json();
        state.needData = {
            remote: data.remote || [],
            local: data.local || [],
            queued: data.queued || [],
            page: data.page || page,
            perPage: data.per_page || 100
        };

        renderNeedTable();
    } catch (e) {
        tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: #ff4444;">Failed to load data</td></tr>';
    }
}

export function renderNeedTable(): void {
    const tbody = document.getElementById('need-tbody')!;
    const pagination = document.getElementById('need-pagination')!;

    let files: FileItem[] = [];
    let statusType = '';

    switch (state.needTab) {
        case 'local':
            files = state.needData.local;
            statusType = 'local';
            break;
        case 'queued':
            files = state.needData.queued;
            statusType = 'queued';
            break;
        case 'remote':
        default:
            files = state.needData.remote;
            statusType = 'syncing';
            break;
    }

    if (files.length === 0) {
        tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: var(--secondary-text);">No files in this category</td></tr>';
        pagination.innerHTML = '';
        return;
    }

    tbody.innerHTML = '';
    files.forEach(f => {
        const tr = document.createElement('tr');
        const badgeClass = statusType === 'syncing' ? 'syncing' : statusType === 'queued' ? 'queued' : 'ok';
        const badgeText = statusType === 'syncing' ? 'From Remote' : statusType === 'queued' ? 'Queued' : 'Local';

        tr.innerHTML = `
            <td><i data-lucide="${f.type === 'DIRECTORY' ? 'folder' : 'file'}" style="width: 16px; display: inline; margin-right: 0.5rem;"></i> ${f.name}</td>
            <td>${formatSize(f.size)}</td>
            <td>${f.modified ? new Date(f.modified).toLocaleString() : '-'}</td>
            <td>${f.type || 'FILE'}</td>
            <td><span class="status-badge ${badgeClass}">${badgeText}</span></td>
        `;
        tbody.appendChild(tr);
    });

    renderPagination(pagination, state.needData.page, state.needData.perPage, loadNeed);
    if ((window as any).lucide) (window as any).lucide.createIcons();
}

// Remote Need View
export async function loadRemoteNeed(page: number = 1): Promise<void> {
    const folderId = (document.getElementById('remote-need-folder-select') as HTMLSelectElement)?.value;
    const deviceId = (document.getElementById('remote-need-device-select') as HTMLSelectElement)?.value;

    if (!folderId || !deviceId) {
        document.getElementById('remote-need-tbody')!.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: var(--secondary-text);">Select folder and device</td></tr>';
        document.getElementById('remote-need-pagination')!.innerHTML = '';
        return;
    }

    const tbody = document.getElementById('remote-need-tbody')!;
    tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem;">Loading...</td></tr>';

    try {
        const url = `/api/syncweb/remote-need?folder_id=${encodeURIComponent(folderId)}&device_id=${encodeURIComponent(deviceId)}&page=${page}&per_page=100`;
        const resp = await fetchAPI(url);
        if (!resp.ok) throw new Error('Failed to load remote need files');

        const data = await resp.json();
        state.remoteNeedData = {
            files: data.files || [],
            page: data.page || page,
            perPage: data.per_page || 100
        };

        renderRemoteNeedTable();
    } catch (e) {
        tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: #ff4444;">Failed to load data</td></tr>';
    }
}

export function renderRemoteNeedTable(): void {
    const tbody = document.getElementById('remote-need-tbody')!;
    const pagination = document.getElementById('remote-need-pagination')!;

    if (state.remoteNeedData.files.length === 0) {
        tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; padding: 2rem; color: var(--secondary-text);">No files needed by remote device</td></tr>';
        pagination.innerHTML = '';
        return;
    }

    tbody.innerHTML = '';
    state.remoteNeedData.files.forEach(f => {
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td><i data-lucide="${f.type === 'DIRECTORY' ? 'folder' : 'file'}" style="width: 16px; display: inline; margin-right: 0.5rem;"></i> ${f.name}</td>
            <td>${formatSize(f.size)}</td>
            <td>${f.modified ? new Date(f.modified).toLocaleString() : '-'}</td>
            <td>${f.type || 'FILE'}</td>
            <td>${f.permission || '-'}</td>
        `;
        tbody.appendChild(tr);
    });

    renderPagination(pagination, state.remoteNeedData.page, state.remoteNeedData.perPage, loadRemoteNeed);
    if ((window as any).lucide) (window as any).lucide.createIcons();
}

export function renderPagination(container: HTMLElement, currentPage: number, perPage: number, loadFunction: (page: number) => void): void {
    if (!container) return;

    container.innerHTML = '';

    const prevBtn = document.createElement('button');
    prevBtn.textContent = 'Previous';
    prevBtn.disabled = currentPage <= 1;
    prevBtn.onclick = () => loadFunction(currentPage - 1);
    container.appendChild(prevBtn);

    const pageSpan = document.createElement('span');
    pageSpan.textContent = `Page ${currentPage}`;
    pageSpan.style.margin = '0 1rem';
    container.appendChild(pageSpan);

    const nextBtn = document.createElement('button');
    nextBtn.textContent = 'Next';
    nextBtn.disabled = true; // Would need total count to enable properly
    nextBtn.onclick = () => loadFunction(currentPage + 1);
    container.appendChild(nextBtn);
}

// Start app
if (typeof window !== 'undefined' && window.document) {
    loadFolders();
    loadDevices();
    loadMounts();
    loadStatus();
    loadEvents();
    setInterval(loadEvents, 10000); // Refresh events every 10s

    // Add search listener for "Enter" key
    document.getElementById('search-input')?.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') searchFiles();
    });
}
