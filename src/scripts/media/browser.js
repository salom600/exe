/**
 * FlowCut — Media Browser Controller
 * Manages media file import, display, search/filter, and drag-to-timeline.
 */

class FlowCutMediaBrowser {
    constructor() {
        this.grid = document.getElementById('media-grid');
        this.searchInput = document.getElementById('media-search-input');
        this.mediaItems = [];
        this.selectedMediaId = null;
        this.currentFilter = 'all';

        this.bindEvents();
    }

    /**
     * Bind media browser UI events.
     */
    bindEvents() {
        // Import button
        const importBtn = document.getElementById('import-btn');
        if (importBtn) {
            importBtn.addEventListener('click', () => this.importMedia());
        }

        // Search input
        if (this.searchInput) {
            this.searchInput.addEventListener('input', (e) => {
                this.filterMedia(e.target.value);
                window.FlowCutEventBus.publish(window.FlowCutEvents.MEDIA_SEARCH_CHANGED, { query: e.target.value });
            });
        }

        // Filter tabs
        document.querySelectorAll('.media-tabs .tab').forEach(tab => {
            tab.addEventListener('click', () => {
                document.querySelectorAll('.media-tabs .tab').forEach(t => t.classList.remove('active'));
                tab.classList.add('active');
                this.currentFilter = tab.dataset.filter;
                this.renderMediaGrid();
                window.FlowCutEventBus.publish(window.FlowCutEvents.MEDIA_FILTER_CHANGED, { filter: this.currentFilter });
            });
        });

        // Media selection
        this.grid?.addEventListener('click', (e) => {
            const item = e.target.closest('.media-item');
            if (item) {
                this.selectMedia(item.dataset.id);
            }
        });

        // Drag-to-timeline
        this.grid?.addEventListener('dragstart', (e) => {
            const item = e.target.closest('.media-item');
            if (item) {
                e.dataTransfer.setData('text/plain', item.dataset.id);
                item.classList.add('dragging');
            }
        });

        this.grid?.addEventListener('dragend', (e) => {
            const item = e.target.closest('.media-item');
            if (item) item.classList.remove('dragging');
        });
    }

    /**
     * Import media files using Tauri file dialog.
     */
    async importMedia() {
        try {
            // Open file dialog via Tauri
            const { open } = window.__TAURI__.dialog;
            const selected = await open({
                multiple: true,
                filters: [
                    { name: 'Video Files', extensions: ['mp4', 'mkv', 'mov', 'avi', 'webm', 'flv', 'wmv', 'ts'] },
                    { name: 'Audio Files', extensions: ['mp3', 'wav', 'flac', 'aac', 'ogg', 'm4a'] },
                    { name: 'Image Files', extensions: ['png', 'jpg', 'jpeg', 'gif', 'bmp', 'webp', 'svg'] },
                    { name: 'All Files', extensions: ['*'] },
                ]
            });

            if (!selected) return;

            const paths = Array.isArray(selected) ? selected : [selected];
            const result = await ipc.importMedia(paths);

            if (result) {
                this.mediaItems = [...this.mediaItems, ...result];
                this.renderMediaGrid();
                window.FlowCutEventBus.publish(window.FlowCutEvents.MEDIA_IMPORTED, { items: result });
                app.showToast('success', 'Media Imported', `Imported ${result.length} file(s).`);
            }
        } catch (e) {
            console.error('Media import failed:', e);
            app.showToast('error', 'Import Failed', 'Could not import media files.');
        }
    }

    /**
     * Select a media item by ID.
     */
    selectMedia(id) {
        this.selectedMediaId = id;
        document.querySelectorAll('.media-item').forEach(item => {
            item.classList.toggle('selected', item.dataset.id === id);
        });

        const mediaItem = this.mediaItems.find(m => m.id === id);
        if (mediaItem) {
            window.FlowCutEventBus.publish(window.FlowCutEvents.MEDIA_SELECTED, { item: mediaItem });
        }
    }

    /**
     * Filter media items by search query.
     */
    filterMedia(query) {
        this.renderMediaGrid(query);
    }

    /**
     * Render the media thumbnail grid.
     */
    renderMediaGrid(searchQuery = '') {
        if (!this.grid) return;

        // Filter items
        let items = this.mediaItems;
        if (this.currentFilter !== 'all') {
            items = items.filter(m => m.media_type?.toLowerCase() === this.currentFilter);
        }
        if (searchQuery) {
            items = items.filter(m => m.name?.toLowerCase().includes(searchQuery.toLowerCase()));
        }

        if (items.length === 0) {
            this.grid.innerHTML = `
                <div class="media-empty">
                    <svg viewBox="0 0 48 48" width="48" height="48" fill="none">
                        <rect x="4" y="4" width="40" height="40" rx="4" stroke="#6c7086" stroke-width="2"/>
                        <path d="M16 20L24 28L32 20" stroke="#6c7086" stroke-width="2"/>
                    </svg>
                    <p>${this.mediaItems.length === 0 ? 'Import media files to start editing' : 'No matching media found'}</p>
                </div>
            `;
            return;
        }

        this.grid.innerHTML = items.map(item => {
            const mediaType = (item.media_type || 'video').toLowerCase();
            const durationStr = item.duration ? this.formatDuration(item.duration) : '';
            const thumbContent = item.thumbnail_path
                ? `<img src="${item.thumbnail_path}" alt="${item.name}" />`
                : `<span class="thumb-placeholder">${mediaType === 'video' ? '&#9654;' : mediaType === 'audio' ? '&#9835;' : '&#9638;'}</span>`;

            return `
                <div class="media-item" data-id="${item.id}" draggable="true">
                    <div class="media-thumb">${thumbContent}</div>
                    <span class="media-badge ${mediaType}">${mediaType}</span>
                    <div class="media-info">
                        <div class="media-name">${item.name || 'Unknown'}</div>
                        <div class="media-duration">${durationStr}</div>
                    </div>
                </div>
            `;
        }).join('');
    }

    /**
     * Format duration in human-readable format.
     */
    formatDuration(seconds) {
        if (!seconds) return '--:--';
        const mins = Math.floor(seconds / 60);
        const secs = Math.floor(seconds % 60);
        return `${mins}:${secs.toString().padStart(2, '0')}`;
    }

    /**
     * Handle media imported event.
     */
    onMediaImported(data) {
        this.mediaItems = [...this.mediaItems, ...data.items];
        this.renderMediaGrid();
    }

    /**
     * Load media from project data.
     */
    loadFromProject(projectData) {
        this.mediaItems = projectData.media_pool || [];
        this.renderMediaGrid();
    }

    /**
     * Clear all media items.
     */
    clearMedia() {
        this.mediaItems = [];
        this.selectedMediaId = null;
        this.renderMediaGrid();
    }
}

window.FlowCutMediaBrowser = FlowCutMediaBrowser;
