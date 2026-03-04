const state = {
    folders: [],
    devices: [],
    pendingDevices: {},
    currentFolder: null,
    currentPath: '/',
    files: [],
    token: ''
};

// Initialize token from URL if in browser
if (typeof window !== 'undefined' && window.location) {
    state.token = new URLSearchParams(window.location.search).get('token') || '';
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
            return fetchAPI(url, options);
        }
    }
    return resp;
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
        li.innerHTML = `<span class="icon">🔔</span> Pending: ${id.substring(0, 7)}...`;
        li.title = `Click to accept device: ${id}`;
        li.onclick = () => addDevice(id);
        list.appendChild(li);
    });

    state.devices.forEach(d => {
        const li = document.createElement('li');
        li.className = 'folder-item';
        const statusIcon = d.paused ? '⏸️' : '💻';
        li.innerHTML = `<span class="icon">${statusIcon}</span> ${d.name || d.id.substring(0, 7) + '...'}`;
        li.title = d.id;
        li.oncontextmenu = (e) => {
            e.preventDefault();
            if (confirm(`Delete device ${d.name || d.id}?`)) {
                deleteDevice(d.id);
            }
        };
        list.appendChild(li);
    });
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
    rootLi.innerHTML = `<span class="icon">🏠</span> [Root]`;
    rootLi.onclick = () => selectFolder(null);
    list.appendChild(rootLi);

    state.folders.forEach(f => {
        const li = document.createElement('li');
        li.className = 'folder-item' + (state.currentFolder === f.id ? ' active' : '');
        li.innerHTML = `<span class="icon">📂</span> ${f.id}`;
        li.onclick = () => selectFolder(f.id);
        li.oncontextmenu = (e) => {
            e.preventDefault();
            if (confirm(`Delete folder ${f.id}?`)) {
                deleteFolder(f.id);
            }
        };
        list.appendChild(li);
    });
}

async function addFolder() {
    const id = prompt("Enter Folder ID:");
    if (!id) return;
    const path = prompt("Enter Local Path:");
    if (!path) return;

    try {
        const resp = await fetchAPI('/api/syncweb/folders/add', {
            method: 'POST',
            body: JSON.stringify({ id, path })
        });
        if (resp.ok) {
            showToast("Folder added");
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

    if (!isSearch) {
        // Parent dir
        if (state.currentFolder && state.currentPath !== `syncweb://${state.currentFolder}/`) {
            const li = document.createElement('li');
            li.className = 'file-item';
            li.innerHTML = `<span class="icon">⬆️</span> ..`;
            li.onclick = goUp;
            list.appendChild(li);
        } else if (state.currentFolder && state.currentPath === `syncweb://${state.currentFolder}/`) {
            const li = document.createElement('li');
            li.className = 'file-item';
            li.innerHTML = `<span class="icon">⬆️</span> [Root]`;
            li.onclick = () => selectFolder(null);
            list.appendChild(li);
        }
    }

    state.files.forEach(f => {
        const li = document.createElement('li');
        li.className = 'file-item';
        li.draggable = true;
        const displayName = isSearch ? f.path : f.name;
        li.innerHTML = `<span class="icon">${f.is_dir ? '📁' : '📄'}</span> ${displayName} ${!f.local && !f.is_dir ? '☁️' : ''}`;
        
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
        };

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
                const dstPath = f.path + '/' + srcPath.split('/').pop();
                moveFile(srcPath, dstPath);
            };
        }

        list.appendChild(li);
    });
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
    const isOffline = btn.innerText === 'Go Online';
    try {
        const resp = await fetchAPI('/api/syncweb/toggle', {
            method: 'POST',
            body: JSON.stringify({ offline: !isOffline })
        });
        const data = await resp.json();
        btn.innerText = data.offline ? 'Go Online' : 'Go Offline';
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
        if (btn) btn.innerText = data.offline ? 'Go Online' : 'Go Offline';
    } catch (e) {}
}

function showToast(msg, isError = false) {
    const t = document.getElementById('toast');
    if (!t) return;
    t.innerText = msg;
    t.style.background = isError ? '#ff4444' : 'var(--accent-color)';
    t.style.display = 'block';
    setTimeout(() => t.style.display = 'none', 3000);
}

function refresh() { loadFolders(); loadDevices(); loadFiles(); loadStatus(); }

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
        deleteFolder
    };
} else {
    // Start app
    loadFolders();
    loadDevices();
    loadStatus();

    // Add search listener for "Enter" key
    document.getElementById('search-input')?.addEventListener('keypress', (e) => {
        if (e.key === 'Enter') searchFiles();
    });
}
