const state = {
    folders: [],
    devices: [],
    pendingDevices: {},
    mounts: [],
    currentFolder: null,
    currentPath: '/',
    files: [],
    token: '',
    selectedItems: [],
    events: [],
    sortBy: 'name'
};

// Initialize token from localStorage or URL if in browser
if (typeof window !== 'undefined' && window.localStorage && window.location) {
    state.token = new URLSearchParams(window.location.search).get('token') || localStorage.getItem('syncweb_token') || '';
    if (state.token) localStorage.setItem('syncweb_token', state.token);
}

async function fetchAPI(url, options = {}) {
    const headers = {
        'X-Syncweb-Token': state.token,
        'Content-Type': 'application/json',
        ...options.headers
    };
    const resp = await fetch(url, { ...options, headers });
    if (resp.status === 401) {
        const newToken = prompt("Unauthorized. Enter API Token:");
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

function logout() {
    state.token = '';
    if (typeof window !== 'undefined' && window.localStorage) {
        localStorage.removeItem('syncweb_token');
    }
    location.reload();
}

function toggleSidebar() {
    const sidebar = document.querySelector('aside');
    sidebar.classList.toggle('open');
}

async function loadFolders() {
    try {
        const resp = await fetchAPI('/api/syncweb/folders');
        state.folders = await resp.json();
        renderFolders();
    } catch (e) {
        showToast("Failed to load folders", true);
    }
}

async function loadDevices() {
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

function renderDevices() {
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
    if (window.lucide) lucide.createIcons();
}

async function addDevice(suggestedId = '') {
    const id = suggestedId || prompt("Enter Device ID:", suggestedId);
    if (!id) return;
    const name = prompt("Enter Device Name (optional):", "");
    
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

async function deleteDevice(id) {
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


function renderFolders() {
    const list = document.getElementById('folder-list');
    if (!list) return; // Guard for test env if DOM not ready
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
    if (window.lucide) lucide.createIcons();
}

async function addFolder() {
    document.getElementById('add-folder-ui').style.display = 'block';
    document.getElementById('path-preview').innerHTML = '';
    document.getElementById('new-folder-id').value = '';
    document.getElementById('new-folder-path').value = '';
}

async function previewLocalPath() {
    const path = document.getElementById('new-folder-path').value;
    if (!path) return;
    
    try {
        const resp = await fetchAPI(`/api/local/ls?path=${encodeURIComponent(path)}`);
        if (resp.ok) {
            const files = await resp.json();
            let html = '<strong>Contents:</strong><div style="margin-top: 0.5rem;">';
            files.slice(0, 5).forEach(f => {
                const icon = f.is_dir ? 'folder' : 'file-text';
                html += `<div style="display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.2rem;"><i data-lucide="${icon}" style="width: 14px; height: 14px;"></i> ${f.name}</div>`;
            });
            if (files.length > 5) html += `<div style="font-size: 0.8rem; margin-top: 0.2rem; color: var(--secondary-text);">... and ${files.length - 5} more</div>`;
            html += '</div>';
            document.getElementById('path-preview').innerHTML = html;
            if (window.lucide) lucide.createIcons();
        } else {
            document.getElementById('path-preview').innerHTML = '<span style="color: #ff4444;">Path not found or inaccessible</span>';
        }
    } catch (e) {
        document.getElementById('path-preview').innerHTML = '<span style="color: #ff4444;">Error accessing path</span>';
    }
}

async function confirmAddFolder() {
    const id = document.getElementById('new-folder-id').value;
    const path = document.getElementById('new-folder-path').value;
    
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
            document.getElementById('add-folder-ui').style.display = 'none';
            loadFolders();
        } else {
            const data = await resp.json();
            showToast(data.error || "Failed to add folder", true);
        }
    } catch (e) {
        showToast("Error adding folder", true);
    }
}


async function deleteFolder(id) {
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


async function loadMounts() {
    try {
        const resp = await fetchAPI('/api/mounts');
        state.mounts = await resp.json();
        renderMounts();
    } catch (e) {
        showToast("Failed to load mounts", true);
    }
}

function renderMounts() {
    const list = document.getElementById('mount-list');
    if (!list) return;
    list.innerHTML = '';

    const flatten = (devices) => {
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
                    const mp = prompt("Enter mountpoint path:", `/mnt/${d.label || d.name}`);
                    if (mp) mountDevice(`/dev/${d.name}`, mp);
                };
                list.appendChild(li);
            }

            if (d.children) flatten(d.children);
        });
    };

    flatten(state.mounts);
    if (window.lucide) lucide.createIcons();
}

async function mountDevice(device, mountpoint) {
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

async function unmountPoint(mountpoint) {
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

async function selectFolder(id) {
    if (id === null) {
        state.currentFolder = null;
        state.currentPath = "/";
    } else {
        state.currentFolder = id;
        state.currentPath = `syncweb://${id}/`;
    }
    renderFolders();
    loadFiles();
}

async function loadFiles() {
    try {
        let url;
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

async function searchFiles() {
    const query = document.getElementById('search-input')?.value;
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

function renderFiles(isSearch = false) {
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
                return new Date(b.modified || 0) - new Date(a.modified || 0);
            case 'name':
            default:
                return a.name.localeCompare(b.name);
        }
    });

    if (!isSearch) {
        // Parent dir
        if (state.currentFolder && state.currentPath !== `syncweb://${state.currentFolder}/`) {
            const li = document.createElement('li');
            li.className = 'file-item';
            li.innerHTML = `<span class="icon"><i data-lucide="arrow-up"></i></span> ..`;
            li.onclick = goUp;
            list.appendChild(li);
        } else if (state.currentFolder && state.currentPath === `syncweb://${state.currentFolder}/`) {
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
                // Extract folder ID from path syncweb://id/
                const folderID = f.path.split('/')[2];
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
            e.dataTransfer.setData('text/plain', f.path);
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
                const srcPath = e.dataTransfer.getData('text/plain');
                if (srcPath === f.path) return;
                const dstPath = f.path + '/' + srcPath.split('/').pop();
                moveFile(srcPath, dstPath);
            };
        }

        list.appendChild(li);
    });
    if (window.lucide) lucide.createIcons();
}

function goUp() {
    const parts = state.currentPath.split('/');
    parts.pop(); // Remove trailing empty string
    parts.pop(); // Remove last dir
    state.currentPath = parts.join('/') + '/';
    loadFiles();
}

async function moveFile(src, dst) {
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

async function showFileProperties(path) {
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

async function triggerDownload(path) {
    try {
        const resp = await fetchAPI('/api/syncweb/download', {
            method: 'POST',
            body: JSON.stringify({ path })
        });
        if (resp.ok) showToast("Download triggered");
    } catch (e) {}
}

async function toggleOffline() {
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
        const icon = btn.querySelector('i');
        if (icon) icon.setAttribute('data-lucide', data.offline ? 'power-off' : 'power');
        if (window.lucide) lucide.createIcons();
        showToast(data.offline ? "Backend Stopped" : "Backend Started");
    } catch (e) {
        showToast("Toggle failed", true);
    }
}

async function loadStatus() {
    try {
        const resp = await fetchAPI('/api/syncweb/status');
        const data = await resp.json();
        const btn = document.getElementById('offline-btn');
        if (btn) {
            const span = btn.querySelector('span');
            if (span) span.innerText = data.offline ? 'Go Online' : 'Go Offline';
            const icon = btn.querySelector('i');
            if (icon) icon.setAttribute('data-lucide', data.offline ? 'power-off' : 'power');
            if (window.lucide) lucide.createIcons();
        }
    } catch (e) {}
}

function showToast(message, isError = false) {
    const t = document.getElementById('toast');
    if (!t) return;
    t.textContent = message;
    t.style.background = isError ? '#ff4444' : 'var(--accent-color)';
    t.style.color = isError ? '#fff' : 'var(--bg-color)';
    t.style.display = 'block';

    // Clear existing timeout if any
    if (t.timeout) clearTimeout(t.timeout);

    t.timeout = setTimeout(() => {
        t.style.display = 'none';
    }, 4000);
}

function toggleSelection(path) {
    const idx = state.selectedItems.indexOf(path);
    if (idx === -1) {
        state.selectedItems.push(path);
    } else {
        state.selectedItems.splice(idx, 1);
    }
    updateBulkActions();
    renderFiles(state.currentPath.startsWith('Search results'));
}

function updateBulkActions() {
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

function clearSelection() {
    state.selectedItems = [];
    updateBulkActions();
    renderFiles(state.currentPath.startsWith('Search results'));
}

async function bulkDelete() {
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

async function bulkMove() {
    if (state.selectedItems.length === 0) return;
    const dstFolder = prompt("Enter destination folder ID (or empty for root):", state.currentFolder || "");
    if (dstFolder === null) return;
    
    let dstPath;
    if (!dstFolder) {
        dstPath = "/";
    } else {
        dstPath = `syncweb://${dstFolder}/`;
    }

    const subPath = prompt("Enter subpath within destination (e.g. 'Photos/2023'):", "");
    if (subPath === null) return;
    if (subPath) {
        dstPath += subPath.endsWith('/') ? subPath : subPath + '/';
    }

    let success = 0;
    for (const src of state.selectedItems) {
        const name = src.split('/').pop();
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

async function bulkCopy() {
    if (state.selectedItems.length === 0) return;
    const dstFolder = prompt("Enter destination folder ID (or empty for root):", state.currentFolder || "");
    if (dstFolder === null) return;
    
    let dstPath;
    if (!dstFolder) {
        dstPath = "/";
    } else {
        dstPath = `syncweb://${dstFolder}/`;
    }

    const subPath = prompt("Enter subpath within destination (e.g. 'Photos/2023'):", "");
    if (subPath === null) return;
    if (subPath) {
        dstPath += subPath.endsWith('/') ? subPath : subPath + '/';
    }

    let success = 0;
    for (const src of state.selectedItems) {
        const name = src.split('/').pop();
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

function updateSort() {
    state.sortBy = document.getElementById('sort-select').value;
    renderFiles(state.currentPath.startsWith('Search results'));
}

function formatSize(bytes) {
    if (bytes === 0) return '0 B';
    const k = 1024;
    const sizes = ['B', 'KB', 'MB', 'GB', 'TB'];
    const i = Math.floor(Math.log(bytes) / Math.log(k));
    return parseFloat((bytes / Math.pow(k, i)).toFixed(2)) + ' ' + sizes[i];
}

async function loadEvents() {
    try {
        const resp = await fetchAPI('/api/syncweb/events');
        state.events = await resp.json();
        renderEvents();
    } catch (e) {}
}

function renderEvents() {
    const list = document.getElementById('activity-list');
    if (!list) return;
    
    // Sort events by time descending
    const sortedEvents = [...state.events].sort((a, b) => new Date(b.time) - new Date(a.time));
    
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
    if (window.lucide) lucide.createIcons();
}

function refresh() { loadFolders(); loadDevices(); loadMounts(); loadFiles(); loadStatus(); loadEvents(); }

// Export for testing
if (typeof module !== 'undefined' && module.exports) {
    module.exports = {
        state,
        fetchAPI,
        loadFolders,
        renderFolders,
        selectFolder,
        loadFiles,
        renderFiles,
        goUp,
        moveFile,
        triggerDownload,
        toggleOffline,
        loadStatus,
        showToast,
        refresh,
        searchFiles,
        loadDevices,
        addDevice,
        deleteDevice,
        showFileProperties,
        addFolder,
        deleteFolder,
        loadMounts,
        mountDevice,
        unmountPoint,
        previewLocalPath,
        confirmAddFolder,
        toggleSelection,
        clearSelection,
        bulkDelete,
        bulkMove,
        bulkCopy,
        logout
    };
} else {
    // Start app
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
