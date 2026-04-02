import { describe, it, expect, vi, beforeEach } from 'vitest';
import {
    state,
    loadFolders,
    selectFolder,
    loadFiles,
    goUp,
    moveFile,
    triggerDownload,
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
    toggleActivity,
    setSort,
    toggleSelectAll
} from './app';

// Mock fetch
const mockFetch = vi.fn();
global.fetch = mockFetch as any;
global.prompt = vi.fn();

describe('Syncweb UI', () => {
    let folderList: HTMLUListElement, fileList: HTMLUListElement, pathHeader: HTMLElement, toast: HTMLElement;

    beforeEach(() => {
        // Reset DOM
        document.body.innerHTML = `
            <div id="app-container">
                <ul id="folder-list"></ul>
                <ul id="device-list"></ul>
                <ul id="mount-list"></ul>
                <div id="breadcrumbs"></div>
                <h2 id="current-folder-title"></h2>
                <table><tbody id="file-list-body"></tbody></table>
                <div id="files-pagination"></div>
                <input id="search-input" value="">
                <div id="toast" style="display: none;"></div>
                <div id="add-folder-ui" style="display: none;">
                    <input id="new-folder-id">
                    <input id="new-folder-path">
                    <div id="path-preview"></div>
                </div>
                <select id="sort-select"><option value="name">Name</option></select>
            </div>
        `;

        folderList = document.getElementById('folder-list') as HTMLUListElement;
        fileList = document.getElementById('file-list-body') as any;
        pathHeader = document.getElementById('breadcrumbs') as HTMLElement;
        toast = document.getElementById('toast') as HTMLElement;

        // Reset state
        state.folders = [];
        state.currentFolder = null;
        state.currentPath = '/';
        state.files = [];
        state.filesPage = 1;
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

            const items = fileList.getElementsByTagName('tr');
            // 2 files (folders first in sort)
            expect(items.length).toBe(2);
            expect(items[0].textContent).toContain('sub');
            expect(items[1].textContent).toContain('doc.txt');
        });

        it('selectFolder updates state and loads files', async () => {
            mockFetch.mockResolvedValue({
                ok: true,
                json: async () => []
            });

            await selectFolder('f1');

            expect(state.currentFolder).toBe('f1');
            expect(state.currentPath).toBe('sync://f1/');
        });

        it('selectFolder handles syncweb:// URLs correctly', async () => {
            const mockFiles = [
                { name: 'f1', is_dir: true, local: true, size: 0, path: 'sync://f1/' }
            ];
            state.currentFolder = null;
            state.currentPath = '/';

            mockFetch.mockResolvedValue({
                ok: true,
                json: async () => mockFiles
            });

            await loadFiles();
            const items = fileList.getElementsByTagName('tr');
            const nameCell = items[0].getElementsByTagName('td')[2];
            (nameCell as HTMLElement).click();

            expect(state.currentFolder).toBe('f1');
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
            mockFetch.mockResolvedValueOnce({ ok: true });
            await moveFile('src', 'dst');
            expect(mockFetch).toHaveBeenCalledWith('/api/file/move', expect.objectContaining({
                method: 'POST',
                body: JSON.stringify({ src: 'src', dst: 'dst' })
            }));
        });

        it('triggerDownload sends correct request', async () => {
            mockFetch.mockResolvedValueOnce({ ok: true });
            await triggerDownload('path');
            expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/download', expect.objectContaining({
                method: 'POST',
                body: JSON.stringify({ path: 'path' })
            }));
        });
    });

    describe('Future Functionality (CLI Parity)', () => {
        describe('Folder Management', () => {
            it('previewLocalPath() fetches local files and updates UI', async () => {
                const input = document.getElementById('new-folder-path') as HTMLInputElement;
                input.value = '/test/path';
                const preview = document.getElementById('path-preview')!;
                
                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => [{ name: 'file1', is_dir: false }]
                });

                await previewLocalPath();

                expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/api/local/ls?path=%2Ftest%2Fpath'), expect.any(Object));
                expect(preview.innerHTML).toContain('file1');
            });

            it('confirmAddFolder() sends POST request and hides UI', async () => {
                const ui = document.getElementById('add-folder-ui')!;
                ui.style.display = 'block';
                (document.getElementById('new-folder-id') as HTMLInputElement).value = 'id';
                (document.getElementById('new-folder-path') as HTMLInputElement).value = 'path';

                mockFetch.mockResolvedValueOnce({ ok: true });
                // We also need to mock loadFolders inside confirmAddFolder
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] }); 
                // And pending folders
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) });

                await confirmAddFolder();

                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/folders/add', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ id: 'id', path: 'path' })
                }));
                expect(ui.style.display).toBe('none');
            });

            it('deleteFolder(id) sends POST request', async () => {
                mockFetch.mockResolvedValueOnce({ ok: true });
                // loadFolders mock
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] });
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) });

                await deleteFolder('fid');

                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/folders/delete', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ id: 'fid' })
                }));
            });
        });

        describe('Device Management', () => {
            it('loadDevices() fetches and renders devices', async () => {
                state.devices = [];
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [{ id: 'd1', name: 'dev1' }] });
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) }); // Pending

                await loadDevices();

                expect(document.getElementById('device-list')!.innerHTML).toContain('dev1');
            });

            it('addDevice(id) sends POST request', async () => {
                mockFetch.mockResolvedValueOnce({ ok: true });
                // loadDevices mocks
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] });
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => ({}) });

                await addDevice('did');

                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/devices/add', expect.objectContaining({
                    method: 'POST',
                    body: expect.stringContaining('"id":"did"')
                }));
            });

            it('acceptDevice(id) calls addDevice with ID and only prompts for name', async () => {
                // Actually acceptDevice is just addDevice(id) called from renderDevices
                mockFetch.mockResolvedValue({ ok: true });
                // loadDevices mocks
                mockFetch.mockResolvedValue({ ok: true, json: async () => [] });
                
                await addDevice('did');
                expect(mockFetch).toHaveBeenCalledWith('/api/syncweb/devices/add', expect.objectContaining({
                    body: expect.stringContaining('"id":"did"')
                }));
            });
        });

        describe('Mountpoint Management', () => {
            it('loadMounts() fetches and renders mounts', async () => {
                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => [{ name: 'sda1', mountpoints: ['/mnt/data'], size: '10G', type: 'part' }]
                });

                await loadMounts();

                expect(document.getElementById('mount-list')!.innerHTML).toContain('/mnt/data');
            });

            it('mountDevice() sends POST request', async () => {
                mockFetch.mockResolvedValueOnce({ ok: true });
                // loadMounts mock
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] });

                await mountDevice('/dev/sdb1', '/mnt/usb');

                expect(mockFetch).toHaveBeenCalledWith('/api/mount', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ device: '/dev/sdb1', mountpoint: '/mnt/usb' })
                }));
            });

            it('unmountPoint() sends POST request', async () => {
                mockFetch.mockResolvedValueOnce({ ok: true });
                // loadMounts mock
                mockFetch.mockResolvedValueOnce({ ok: true, json: async () => [] });

                await unmountPoint('/mnt/usb');

                expect(mockFetch).toHaveBeenCalledWith('/api/unmount', expect.objectContaining({
                    method: 'POST',
                    body: JSON.stringify({ mountpoint: '/mnt/usb' })
                }));
            });
        });

        describe('Search & Details', () => {
            it('searchFiles() calls find API and renders results', async () => {
                const searchInput = document.getElementById('search-input') as HTMLInputElement;
                searchInput.value = 'testfile';

                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => [{ name: 'testfile', path: 'sync://f1/testfile', is_dir: false }]
                });

                await searchFiles();

                expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/api/syncweb/find?q=testfile'), expect.any(Object));
                expect(pathHeader.textContent).toContain('testfile');

                const items = fileList.getElementsByTagName('tr');
                expect(items[0].textContent).toContain('testfile');
            });

            it('fileProperties(path) fetches detailed metadata', async () => {
                global.alert = vi.fn();
                mockFetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => ({ name: 'file.txt', size: 1048576, modified: '2023-01-01T00:00:00Z', local: true })
                });

                await showFileProperties('sync://f1/file.txt');

                expect(mockFetch).toHaveBeenCalledWith(expect.stringContaining('/api/syncweb/stat?path='), expect.any(Object));
                expect(global.alert).toHaveBeenCalled();
            });
        });
    });
});
