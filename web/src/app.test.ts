import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
    state,
    loadFolders,
    selectFolder,
    loadFiles,
    goUp,
    moveFile,
    triggerDownload,
    toggleOffline,
    loadStatus,
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
    confirmAddFolder
} from './app';

// Mock fetch
const mockFetch = vi.fn();
global.fetch = mockFetch as any;
global.prompt = vi.fn();

describe('Syncweb UI', () => {
    let folderList: HTMLUListElement, fileList: HTMLUListElement, pathHeader: HTMLElement, offlineBtn: HTMLButtonElement, toast: HTMLElement;

    beforeEach(() => {
        // Reset DOM
        document.body.innerHTML = `
            <ul id="folder-list"></ul>
            <ul id="device-list"></ul>
            <ul id="mount-list"></ul>
            <ul id="file-list"></ul>
            <h2 id="current-path">/</h2>
            <button id="offline-btn"><i></i><span>Go Offline</span></button>
            <input id="search-input" value="">
            <div id="toast" style="display: none;"></div>
            <div id="add-folder-ui" style="display: none;">
                <input id="new-folder-id">
                <input id="new-folder-path">
                <div id="path-preview"></div>
            </div>
        `;

        folderList = document.getElementById('folder-list') as HTMLUListElement;
        fileList = document.getElementById('file-list') as HTMLUListElement;
        pathHeader = document.getElementById('current-path') as HTMLElement;
        offlineBtn = document.getElementById('offline-btn') as HTMLButtonElement;
        toast = document.getElementById('toast') as HTMLElement;

        // Reset state
        state.folders = [];
        state.currentFolder = null;
        state.currentPath = '/';
        state.files = [];
        state.token = 'test-token';

        vi.clearAllMocks();
    });

    describe('File Browser & Navigation', () => {
        it('loadFolders renders folders correctly', async () => {
            const mockFolders = [{ id: 'folder1' }, { id: 'folder2' }];
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockFolders
            });

            await loadFolders();

            expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/folders', expect.any(Object));
            expect(state.folders).toEqual(mockFolders);

            const items = folderList.getElementsByTagName('li');
            expect(items.length).toBe(3); // Root + 2 folders
            expect(items[0].textContent).toContain('[Root]');
            expect(items[1].textContent).toContain('folder1');
            expect(items[2].textContent).toContain('folder2');
        });

        it('loadFiles renders files correctly', async () => {
            const mockFiles = [
                { name: 'doc.txt', is_dir: false, local: true, size: 100, path: 'sync://f1/doc.txt' },
                { name: 'sub', is_dir: true, local: true, path: 'sync://f1/sub' }
            ];
            state.currentFolder = 'f1';
            state.currentPath = 'sync://f1/';

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockFiles
            });

            await loadFiles();

            expect(mockFetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/syncweb/ls?folder=f1'),
                expect.any(Object)
            );

            const items = fileList.getElementsByTagName('li');
            // Root link + 2 files
            expect(items.length).toBe(3);
            expect(items[0].textContent).toContain('[Root]'); // Up link
            expect(items[1].textContent).toContain('sub');
            expect(items[2].textContent).toContain('doc.txt');
        });

        it('selectFolder updates state and loads files', async () => {
            // Mock loadFiles internal fetch
            mockFetch.mockResolvedValue({
                ok: true,
                json: async () => []
            });

            await selectFolder('new-folder');

            expect(state.currentFolder).toBe('new-folder');
            expect(state.currentPath).toBe('sync://new-folder/');
            expect(mockFetch).toHaveBeenCalled(); // loadFiles called
        });

        it('goUp navigates to parent directory', async () => {
            state.currentPath = 'sync://f1/sub/nested/';
            state.currentFolder = 'f1';

            mockFetch.mockResolvedValue({
                ok: true,
                json: async () => []
            });

            goUp();

            expect(state.currentPath).toBe('sync://f1/sub/');
            expect(mockFetch).toHaveBeenCalled();
        });
    });

    describe('File Operations', () => {
        it('moveFile sends correct request', async () => {
            mockFetch.mockResolvedValueOnce({ ok: true }); // Move success
            mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] }); // Reload files

            await moveFile('src', 'dst');

            expect(mockFetch).toHaveBeenCalledWith('/api/file/move', expect.objectContaining({
                method: 'POST',
                body: JSON.stringify({ src: 'src', dst: 'dst' })
            }));
        });

        it('triggerDownload sends correct request', async () => {
            mockFetch.mockResolvedValueOnce({ ok: true });

            await triggerDownload('path/to/file');

            expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/download', expect.objectContaining({
                method: 'POST',
                body: JSON.stringify({ path: 'path/to/file' })
            }));
        });
    });

    describe('System Status', () => {
        it('toggleOffline calls API and updates button', async () => {
            const span = offlineBtn.querySelector('span') as HTMLSpanElement;
            span.innerText = 'Go Offline'; // Currently online

            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({ offline: true })
            });

            await toggleOffline();

            expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/toggle', expect.objectContaining({
                body: JSON.stringify({ offline: true })
            }));
            expect(span.innerText).toBe('Go Online');
        });

        it('loadStatus updates button state', async () => {
            mockFetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({ offline: true })
            });

            await loadStatus();

            expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/status', expect.any(Object));
            expect(offlineBtn.querySelector('span')!.innerText).toBe('Go Online');
        });
    });

    describe('Future Functionality (CLI Parity)', () => {
        describe('Folder Management', () => {
            it('previewLocalPath() fetches local files and updates UI', async () => {
                const pathInput = document.getElementById('new-folder-path') as HTMLInputElement;
                const previewDiv = document.getElementById('path-preview') as HTMLElement;
                pathInput.value = '/tmp/test';

                const mockFiles = [
                    { name: 'file1.txt', is_dir: false },
                    { name: 'dir1', is_dir: true }
                ];

                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockFiles
                });

                await previewLocalPath();

                expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/api/local/ls?path=%2Ftmp%2Ftest'), expect.any(Object));
                expect(previewDiv.innerHTML).toContain('file1.txt');
                expect(previewDiv.innerHTML).toContain('dir1');
            });

            it('confirmAddFolder() sends POST request and hides UI', async () => {
                const idInput = document.getElementById('new-folder-id') as HTMLInputElement;
                const pathInput = document.getElementById('new-folder-path') as HTMLInputElement;
                const ui = document.getElementById('add-folder-ui') as HTMLElement;

                idInput.value = 'new-id';
                pathInput.value = '/new/path';
                ui.style.display = 'block';

                mockFetch.mockResolvedValueOnce({ ok: true }); // Add folder
                mockFetch.mockResolvedValue({ ok: true, json: async () => [] }); // Reload folders

                await confirmAddFolder();

                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/folders/add', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ id: 'new-id', path: '/new/path' })
                }));
                expect(ui.style.display).toBe('none');
            });

            it('deleteFolder(id) sends POST request', async () => {
                mockFetch.mockResolvedValueOnce({ ok: true }); // Delete folder
                mockFetch.mockResolvedValue({ ok: true, json: async () => [] }); // Reload folders

                await deleteFolder('f1');

                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/folders/delete', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ id: 'f1' })
                }));
            });
        });

        describe('Device Management', () => {
            it('loadDevices() fetches and renders devices', async () => {
                const mockDevices = [{ id: 'dev1', name: 'Laptop', paused: false }];
                const mockPending = { 'dev2': '2026-03-04T00:00:00Z' };

                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockDevices
                });
                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockPending
                });

                await loadDevices();

                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/devices', expect.any(Object));
                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/pending', expect.any(Object));

                const deviceList = document.getElementById('device-list') as HTMLUListElement;
                const items = deviceList.getElementsByTagName('li');
                expect(items.length).toBe(2);
                expect(items[0].textContent).toContain('Pending: dev2');
                expect(items[1].textContent).toContain('Laptop');
            });

            it('addDevice(id) sends POST request', async () => {
                global.prompt = vi.fn()
                    .mockReturnValueOnce('new-dev-id') // ID
                    .mockReturnValueOnce('Desktop');   // Name

                mockFetch.mockResolvedValueOnce({ ok: true }); // Add device
                mockFetch.mockResolvedValue({ ok: true, json: async () => [] }); // Reload devices

                await addDevice();

                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/devices/add', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ id: 'new-dev-id', name: 'Desktop', introducer: false })
                }));
            });

            it('acceptDevice(id) calls addDevice with ID and only prompts for name', async () => {
                global.prompt = vi.fn().mockReturnValue('new-name');
                mockFetch.mockResolvedValueOnce({ ok: true }); // Add device
                mockFetch.mockResolvedValue({ ok: true, json: async () => [] }); // Reload devices

                await addDevice('pending-id');

                expect(window.prompt).toHaveBeenCalledWith(expect.stringContaining('Name'), '');
                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/devices/add', expect.objectContaining({
                    body: JSON.stringify({ id: 'pending-id', name: 'new-name', introducer: false })
                }));
            });
        });

        describe('Mountpoint Management', () => {
            it('loadMounts() fetches and renders mounts', async () => {
                const mockMounts = [
                    { name: 'sda1', mountpoints: ['/mnt/data'], size: '1T', type: 'part' },
                    { name: 'sdb1', mountpoints: [], size: '2T', type: 'part', fstype: 'ext4' }
                ];

                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockMounts
                });

                await loadMounts();

                expect(mockFetch).toHaveBeenCalledWith('/api/mounts', expect.any(Object));

                const mountList = document.getElementById('mount-list') as HTMLUListElement;
                const items = mountList.getElementsByTagName('li');
                expect(items.length).toBe(2);
                expect(items[0].textContent).toContain('/mnt/data');
                expect(items[1].textContent).toContain('sdb1');
                expect(items[1].textContent).toContain('[Unmounted]');
            });

            it('mountDevice() sends POST request', async () => {
                mockFetch.mockResolvedValueOnce({ ok: true }); // Mount
                mockFetch.mockResolvedValue({ ok: true, json: async () => [] }); // Reload mounts

                await mountDevice('/dev/sdb1', '/mnt/new');

                expect(mockFetch).toHaveBeenCalledWith('/api/mount', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ device: '/dev/sdb1', mountpoint: '/mnt/new' })
                }));
            });

            it('unmountPoint() sends POST request', async () => {
                mockFetch.mockResolvedValueOnce({ ok: true }); // Unmount
                mockFetch.mockResolvedValue({ ok: true, json: async () => [] }); // Reload mounts

                await unmountPoint('/mnt/data');

                expect(mockFetch).toHaveBeenCalledWith('/api/unmount', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ mountpoint: '/mnt/data' })
                }));
            });
        });

        describe('Search & Details', () => {
            it('searchFiles() calls find API and renders results', async () => {
                const searchInput = document.getElementById('search-input') as HTMLInputElement;
                searchInput.value = 'testfile';

                const mockResults = [
                    { name: 'testfile.txt', is_dir: false, local: true, size: 100, path: 'sync://f1/testfile.txt' }
                ];

                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockResults
                });

                await searchFiles();

                expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/api/syncweb/find?q=testfile'), expect.any(Object));
                expect(pathHeader.textContent).toContain('Search results for "testfile"');

                const items = fileList.getElementsByTagName('li');
                expect(items.length).toBe(1);
                expect(items[0].textContent).toContain('sync://f1/testfile.txt');
            });

            it('fileProperties(path) fetches detailed metadata', async () => {
                const mockStat = {
                    name: 'test.txt',
                    path: 'sync://f1/test.txt',
                    size: 1048576,
                    modified: '2026-03-04T12:00:00Z',
                    local: true
                };

                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockStat
                });

                global.alert = vi.fn();

                await showFileProperties('sync://f1/test.txt');

                expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/api/syncweb/stat?path=syncweb%3A%2F%2Ff1%2Ftest.txt'), expect.any(Object));
                expect(global.alert).toHaveBeenCalledWith(expect.stringContaining('File: test.txt'));
                expect(global.alert).toHaveBeenCalledWith(expect.stringContaining('1.00 MB'));
            });
        });
    });
});
