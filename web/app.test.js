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
    searchFiles
} from './app.js';

// Mock fetch
global.fetch = vi.fn();
global.prompt = vi.fn();

describe('Syncweb UI', () => {
    let folderList, fileList, pathHeader, offlineBtn, toast;

    beforeEach(() => {
        // Reset DOM
        document.body.innerHTML = `
            <ul id="folder-list"></ul>
            <ul id="file-list"></ul>
            <h2 id="current-path">/</h2>
            <button id="offline-btn">Go Offline</button>
            <input id="search-input" value="">
            <div id="toast" style="display: none;"></div>
        `;

        folderList = document.getElementById('folder-list');
        fileList = document.getElementById('file-list');
        pathHeader = document.getElementById('current-path');
        offlineBtn = document.getElementById('offline-btn');
        toast = document.getElementById('toast');

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
            fetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockFolders
            });

            await loadFolders();

            expect(fetch).toHaveBeenCalledWith('/api/syncweb/folders', expect.any(Object));
            expect(state.folders).toEqual(mockFolders);
            
            const items = folderList.getElementsByTagName('li');
            expect(items.length).toBe(3); // Root + 2 folders
            expect(items[0].textContent).toContain('[Root]');
            expect(items[1].textContent).toContain('folder1');
            expect(items[2].textContent).toContain('folder2');
        });

        it('loadFiles renders files correctly', async () => {
            const mockFiles = [
                { name: 'doc.txt', is_dir: false, local: true, size: 100, path: 'syncweb://f1/doc.txt' },
                { name: 'sub', is_dir: true, local: true, path: 'syncweb://f1/sub' }
            ];
            state.currentFolder = 'f1';
            state.currentPath = 'syncweb://f1/';
            
            fetch.mockResolvedValueOnce({
                ok: true,
                json: async () => mockFiles
            });

            await loadFiles();

            expect(fetch).toHaveBeenCalledWith(
                expect.stringContaining('/api/syncweb/ls?folder=f1'),
                expect.any(Object)
            );
            
            const items = fileList.getElementsByTagName('li');
            // Root link + 2 files
            expect(items.length).toBe(3); 
            expect(items[0].textContent).toContain('[Root]'); // Up link
            expect(items[1].textContent).toContain('doc.txt');
            expect(items[2].textContent).toContain('sub');
        });

        it('selectFolder updates state and loads files', async () => {
            // Mock loadFiles internal fetch
            fetch.mockResolvedValue({
                ok: true,
                json: async () => []
            });

            await selectFolder('new-folder');

            expect(state.currentFolder).toBe('new-folder');
            expect(state.currentPath).toBe('syncweb://new-folder/');
            expect(fetch).toHaveBeenCalled(); // loadFiles called
        });

        it('goUp navigates to parent directory', async () => {
            state.currentPath = 'syncweb://f1/sub/nested/';
            state.currentFolder = 'f1';
            
            fetch.mockResolvedValue({
                ok: true,
                json: async () => []
            });

            goUp();

            expect(state.currentPath).toBe('syncweb://f1/sub/');
            expect(fetch).toHaveBeenCalled();
        });
    });

    describe('File Operations', () => {
        it('moveFile sends correct request', async () => {
            fetch.mockResolvedValueOnce({ ok: true }); // Move success
            fetch.mockResolvedValueOnce({ ok: true, json: async () => [] }); // Reload files

            await moveFile('src', 'dst');

            expect(fetch).toHaveBeenCalledWith('/api/file/move', expect.objectContaining({
                method: 'POST',
                body: JSON.stringify({ src: 'src', dst: 'dst' })
            }));
        });

        it('triggerDownload sends correct request', async () => {
            fetch.mockResolvedValueOnce({ ok: true });

            await triggerDownload('path/to/file');

            expect(fetch).toHaveBeenCalledWith('/api/syncweb/download', expect.objectContaining({
                method: 'POST',
                body: JSON.stringify({ path: 'path/to/file' })
            }));
        });
    });

    describe('System Status', () => {
        it('toggleOffline calls API and updates button', async () => {
            offlineBtn.innerText = 'Go Offline'; // Currently online
            
            fetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({ offline: true })
            });

            await toggleOffline();

            expect(fetch).toHaveBeenCalledWith('/api/syncweb/toggle', expect.objectContaining({
                body: JSON.stringify({ offline: true })
            }));
            expect(offlineBtn.innerText).toBe('Go Online');
        });

        it('loadStatus updates button state', async () => {
            fetch.mockResolvedValueOnce({
                ok: true,
                json: async () => ({ offline: true })
            });

            await loadStatus();

            expect(fetch).toHaveBeenCalledWith('/api/syncweb/status', expect.any(Object));
            expect(offlineBtn.innerText).toBe('Go Online');
        });
    });

    describe('Future Functionality (CLI Parity)', () => {
        describe('Folder Management', () => {
            it.todo('createFolder(path) initializes new sync folder');
            it.todo('deleteFolder(id) triggers delete API');
        });

        describe('Device Management', () => {
            it.todo('listDevices() fetches connected devices');
            it.todo('addDevice(id) adds new device');
            it.todo('acceptDevice(id) accepts pending device');
        });

        describe('Search & Details', () => {
            it('searchFiles() calls find API and renders results', async () => {
                const searchInput = document.getElementById('search-input');
                searchInput.value = 'testfile';
                
                const mockResults = [
                    { name: 'testfile.txt', is_dir: false, local: true, size: 100, path: 'syncweb://f1/testfile.txt' }
                ];
                
                fetch.mockResolvedValueOnce({
                    ok: true,
                    json: async () => mockResults
                });

                await searchFiles();

                expect(fetch).toHaveBeenCalledWith(expect.stringContaining('/api/syncweb/find?q=testfile'), expect.any(Object));
                expect(pathHeader.textContent).toContain('Search results for "testfile"');
                
                const items = fileList.getElementsByTagName('li');
                expect(items.length).toBe(1);
                expect(items[0].textContent).toContain('syncweb://f1/testfile.txt');
            });

            it.todo('fileProperties(path) fetches detailed metadata');
        });
    });
});
