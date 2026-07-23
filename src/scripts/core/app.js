/**
 * FlowCut — Main Application Controller
 * Initializes all modules, manages application lifecycle,
 * and coordinates the overall application state.
 */

class FlowCutApp {
    constructor() {
        this.initialized = false;
        this.currentProject = null;
        this.currentTool = 'select';
        this.modules = {};
        this.settings = {
            theme: 'dark',
            autoSaveInterval: 300000, // 5 minutes
            previewQuality: 'half',
            timelineZoom: 50,
            snapEnabled: true,
            snapThreshold: 10,
            undoMaxHistory: 100,
        };
    }

    /**
     * Initialize the application and all modules.
     */
    async init() {
        console.info('FlowCut: Initializing application...');

        try {
            // Initialize the engine first
            await ipc.initializeEngine();
            console.info('FlowCut: Engine initialized');

            // Initialize all UI modules
            this.modules.timeline = new FlowCutTimeline();
            this.modules.preview = new FlowCutPreview();
            this.modules.mediaBrowser = new FlowCutMediaBrowser();
            this.modules.properties = new FlowCutProperties();
            this.modules.menus = new FlowCutMenus();
            this.modules.dialogs = new FlowCutDialogs();

            // Set up event listeners
            this.setupEventListeners();

            // Set up panel resizing
            this.setupPanelResizing();

            // Set up keyboard shortcuts
            this.setupKeyboardShortcuts();

            // Set up context menu
            this.setupContextMenu();

            // Update system info in status bar
            this.updateSystemInfo();

            // Mark as initialized
            this.initialized = true;
            console.info('FlowCut: Application ready');

            // Show welcome state
            this.showWelcomeState();

        } catch (error) {
            console.error('FlowCut: Initialization failed', error);
            this.showToast('error', 'Initialization Error', 'Failed to initialize FlowCut engine. Please check your system configuration.');
        }
    }

    /**
     * Set up cross-module event listeners.
     */
    setupEventListeners() {
        const bus = window.FlowCutEventBus;
        const events = window.FlowCutEvents;

        // Project events
        bus.subscribe(events.PROJECT_CREATED, (data) => this.onProjectCreated(data));
        bus.subscribe(events.PROJECT_OPENED, (data) => this.onProjectOpened(data));
        bus.subscribe(events.PROJECT_SAVED, () => this.showToast('success', 'Project Saved', 'Your project has been saved successfully.'));
        bus.subscribe(events.PROJECT_CLOSED, () => this.onProjectClosed());

        // Media events
        bus.subscribe(events.MEDIA_IMPORTED, (data) => this.modules.mediaBrowser?.onMediaImported(data));
        bus.subscribe(events.MEDIA_SELECTED, (data) => this.modules.properties?.onMediaSelected(data));

        // Timeline events
        bus.subscribe(events.TIMELINE_SELECTION_CHANGED, (data) => this.modules.properties?.onClipSelected(data));
        bus.subscribe(events.TIMELINE_PLAYHEAD_MOVED, (data) => this.modules.preview?.onPlayheadMoved(data));
        bus.subscribe(events.TIMELINE_ZOOM_CHANGED, (data) => this.onZoomChanged(data));
        bus.subscribe(events.TIMELINE_CLIP_SPLIT, (data) => this.showToast('info', 'Clip Split', 'Clip has been split at the playhead position.'));

        // Preview events
        bus.subscribe(events.PREVIEW_FRAME_RENDERED, (data) => this.modules.preview?.onFrameRendered(data));

        // Export events
        bus.subscribe(events.EXPORT_STARTED, (data) => this.modules.dialogs?.showExportProgress(data));
        bus.subscribe(events.EXPORT_COMPLETED, (data) => this.showToast('success', 'Export Complete', `Video exported to ${data.outputPath}`));
        bus.subscribe(events.EXPORT_FAILED, (data) => this.showToast('error', 'Export Failed', data.error));

        // Tool change events
        bus.subscribe(events.UI_TOOL_CHANGED, (data) => this.onToolChanged(data));

        // Error events
        bus.subscribe(events.ERROR_IPC_FAILED, (data) => {
            console.error('IPC failure:', data);
        });
    }

    /**
     * Set up draggable panel resize handles.
     */
    setupPanelResizing() {
        const handles = document.querySelectorAll('.resize-handle');
        handles.forEach(handle => {
            handle.addEventListener('mousedown', (e) => this.startPanelResize(e, handle));
        });
    }

    /**
     * Handle panel resize drag.
     */
    startPanelResize(e, handle) {
        e.preventDefault();
        handle.classList.add('active');

        const direction = handle.dataset.direction;
        const panel = handle.dataset.panel;
        const panelElement = document.getElementById(panel);
        const startPos = direction === 'horizontal' ? e.clientX : e.clientY;
        const startSize = direction === 'horizontal'
            ? panelElement.offsetWidth
            : panelElement.offsetHeight;

        const onMouseMove = (ev) => {
            const currentPos = direction === 'horizontal' ? ev.clientX : ev.clientY;
            const diff = currentPos - startPos;

            if (direction === 'horizontal') {
                if (panel === 'media-browser') {
                    panelElement.style.width = Math.max(150, Math.min(400, startSize + diff)) + 'px';
                    document.documentElement.style.setProperty('--media-browser-width', panelElement.style.width);
                } else if (panel === 'properties') {
                    panelElement.style.width = Math.max(200, Math.min(500, startSize - diff)) + 'px';
                    document.documentElement.style.setProperty('--properties-panel-width', panelElement.style.width);
                }
            } else {
                panelElement.style.height = Math.max(150, Math.min(600, startSize - diff)) + 'px';
            }

            window.FlowCutEventBus.publish(window.FlowCutEvents.UI_PANEL_RESIZED, {
                panel, direction, size: direction === 'horizontal' ? panelElement.offsetWidth : panelElement.offsetHeight
            });
        };

        const onMouseUp = () => {
            handle.classList.remove('active');
            document.removeEventListener('mousemove', onMouseMove);
            document.removeEventListener('mouseup', onMouseUp);
        };

        document.addEventListener('mousemove', onMouseMove);
        document.addEventListener('mouseup', onMouseUp);
    }

    /**
     * Set up global keyboard shortcuts.
     */
    setupKeyboardShortcuts() {
        document.addEventListener('keydown', (e) => {
            // Skip if focused on input element
            if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA' || e.target.tagName === 'SELECT') return;

            const ctrl = e.ctrlKey || e.metaKey;
            const shift = e.shiftKey;

            // Ctrl+Z — Undo
            if (ctrl && !shift && e.key === 'z') {
                e.preventDefault();
                ipc.undoAction().then(result => {
                    if (result) window.FlowCutEventBus.publish(window.FlowCutEvents.UNDO_PERFORMED, result);
                });
                return;
            }

            // Ctrl+Shift+Z — Redo
            if (ctrl && shift && e.key === 'z') {
                e.preventDefault();
                ipc.redoAction().then(result => {
                    if (result) window.FlowCutEventBus.publish(window.FlowCutEvents.REDO_PERFORMED, result);
                });
                return;
            }

            // Space — Play/Pause
            if (e.key === ' ' || e.code === 'Space') {
                e.preventDefault();
                this.modules.preview?.togglePlayPause();
                return;
            }

            // S — Split at playhead
            if (e.key === 's' && !ctrl) {
                e.preventDefault();
                this.modules.timeline?.splitAtPlayhead();
                return;
            }

            // Delete / Backspace — Delete selected clip
            if (e.key === 'Delete' || e.key === 'Backspace') {
                e.preventDefault();
                this.modules.timeline?.deleteSelectedClip();
                return;
            }

            // V — Selection tool
            if (e.key === 'v' && !ctrl) {
                this.setTool('select');
                return;
            }

            // C — Razor tool
            if (e.key === 'c' && !ctrl) {
                this.setTool('razor');
                return;
            }

            // Y — Slip tool
            if (e.key === 'y' && !ctrl) {
                this.setTool('slip');
                return;
            }

            // U — Slide tool
            if (e.key === 'u' && !ctrl) {
                this.setTool('slide');
                return;
            }

            // B — Ripple edit tool
            if (e.key === 'b' && !ctrl) {
                this.setTool('ripple');
                return;
            }

            // +/- — Zoom
            if (e.key === '+' || e.key === '=') {
                this.modules.timeline?.zoomIn();
                return;
            }
            if (e.key === '-') {
                this.modules.timeline?.zoomOut();
                return;
            }

            // Left/Right — Frame step
            if (e.key === 'ArrowLeft') {
                e.preventDefault();
                this.modules.preview?.stepBackward();
                return;
            }
            if (e.key === 'ArrowRight') {
                e.preventDefault();
                this.modules.preview?.stepForward();
                return;
            }

            // Ctrl+S — Save project
            if (ctrl && e.key === 's') {
                e.preventDefault();
                ipc.saveProject();
                return;
            }

            // Ctrl+N — New project
            if (ctrl && e.key === 'n') {
                e.preventDefault();
                this.modules.dialogs?.showNewProjectDialog();
                return;
            }

            // Ctrl+O — Open project
            if (ctrl && e.key === 'o') {
                e.preventDefault();
                this.modules.dialogs?.showOpenProjectDialog();
                return;
            }

            // Ctrl+E — Export
            if (ctrl && e.key === 'e') {
                e.preventDefault();
                this.modules.dialogs?.showExportDialog();
                return;
            }

            // F11 — Fullscreen
            if (e.key === 'F11') {
                e.preventDefault();
                this.modules.preview?.toggleFullscreen();
                return;
            }
        });
    }

    /**
     * Set up context menu for timeline and media browser.
     */
    setupContextMenu() {
        const contextMenu = document.getElementById('context-menu');

        // Hide context menu on click outside
        document.addEventListener('click', (e) => {
            if (!contextMenu.contains(e.target)) {
                contextMenu.classList.add('hidden');
            }
        });

        // Prevent default context menu
        document.addEventListener('contextmenu', (e) => {
            e.preventDefault();
        });

        // Timeline context menu
        const timelineArea = document.getElementById('timeline-scroll');
        timelineArea?.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            this.showTimelineContextMenu(e);
        });

        // Media browser context menu
        const mediaGrid = document.getElementById('media-grid');
        mediaGrid?.addEventListener('contextmenu', (e) => {
            e.preventDefault();
            this.showMediaContextMenu(e);
        });
    }

    /**
     * Show context menu at position with items.
     */
    showContextMenu(x, y, items) {
        const contextMenu = document.getElementById('context-menu');
        contextMenu.innerHTML = items.map(item => {
            if (item.separator) return '<div class="context-menu-separator"></div>';
            const kbdHtml = item.shortcut ? `<kbd>${item.shortcut}</kbd>` : '';
            const dangerClass = item.danger ? 'danger' : '';
            return `<button class="context-menu-item ${dangerClass}" data-action="${item.action}">${item.label}${kbdHtml}</button>`;
        }).join('');

        contextMenu.style.left = x + 'px';
        contextMenu.style.top = y + 'px';
        contextMenu.classList.remove('hidden');

        // Bind click handlers
        contextMenu.querySelectorAll('.context-menu-item').forEach(btn => {
            btn.addEventListener('click', () => {
                const action = btn.dataset.action;
                contextMenu.classList.add('hidden');
                this.executeAction(action);
            });
        });
    }

    showTimelineContextMenu(e) {
        this.showContextMenu(e.clientX, e.clientY, [
            { label: 'Split at Playhead', action: 'split', shortcut: 'S' },
            { label: 'Delete', action: 'delete', shortcut: 'Del', danger: true },
            { separator: true },
            { label: 'Add Transition', action: 'add-transition' },
            { separator: true },
            { label: 'Add Video Track', action: 'add-video-track' },
            { label: 'Add Audio Track', action: 'add-audio-track' },
            { separator: true },
            { label: 'Properties', action: 'show-properties' },
        ]);
    }

    showMediaContextMenu(e) {
        this.showContextMenu(e.clientX, e.clientY, [
            { label: 'Add to Timeline', action: 'add-to-timeline' },
            { separator: true },
            { label: 'Get Info', action: 'get-media-info' },
            { label: 'Remove', action: 'remove-media', danger: true },
        ]);
    }

    /**
     * Execute a menu or toolbar action.
     */
    executeAction(action) {
        switch (action) {
            case 'new-project':
                this.modules.dialogs?.showNewProjectDialog();
                break;
            case 'open-project':
                this.modules.dialogs?.showOpenProjectDialog();
                break;
            case 'save-project':
                ipc.saveProject();
                break;
            case 'import-media':
                this.modules.mediaBrowser?.importMedia();
                break;
            case 'export':
                this.modules.dialogs?.showExportDialog();
                break;
            case 'undo':
                ipc.undoAction();
                break;
            case 'redo':
                ipc.redoAction();
                break;
            case 'split':
                this.modules.timeline?.splitAtPlayhead();
                break;
            case 'delete':
                this.modules.timeline?.deleteSelectedClip();
                break;
            case 'add-video-track':
                ipc.addTrack('Video', `Video ${this.modules.timeline?.getTrackCount('Video') + 1}`);
                break;
            case 'add-audio-track':
                ipc.addTrack('Audio', `Audio ${this.modules.timeline?.getTrackCount('Audio') + 1}`);
                break;
            case 'add-transition':
                this.modules.dialogs?.showAddTransitionDialog();
                break;
            case 'zoom-in':
                this.modules.timeline?.zoomIn();
                break;
            case 'zoom-out':
                this.modules.timeline?.zoomOut();
                break;
            case 'tool-select':
                this.setTool('select');
                break;
            case 'tool-razor':
                this.setTool('razor');
                break;
            case 'tool-slip':
                this.setTool('slip');
                break;
            case 'tool-slide':
                this.setTool('slide');
                break;
            case 'tool-ripple':
                this.setTool('ripple');
                break;
            case 'settings':
                this.modules.dialogs?.showSettingsDialog();
                break;
            case 'shortcuts':
                this.modules.dialogs?.showShortcutsDialog();
                break;
            case 'about':
                this.modules.dialogs?.showAboutDialog();
                break;
            default:
                console.warn('Unknown action:', action);
        }
    }

    // ============================================
    // Tool Management
    // ============================================

    setTool(tool) {
        this.currentTool = tool;
        document.querySelectorAll('.tool-btn[data-tool]').forEach(btn => {
            btn.classList.toggle('active', btn.dataset.tool === tool);
        });
        window.FlowCutEventBus.publish(window.FlowCutEvents.UI_TOOL_CHANGED, { tool });
    }

    onToolChanged(data) {
        this.currentTool = data.tool;
        // Update timeline interaction mode
        if (this.modules.timeline) {
            this.modules.timeline.setInteractionMode(data.tool);
        }
    }

    // ============================================
    // Project Lifecycle
    // ============================================

    async onProjectCreated(data) {
        this.currentProject = data;
        document.getElementById('project-name').textContent = data.name;
        document.title = `FlowCut — ${data.name}`;
        this.modules.timeline?.initializeDefaultTracks();
        this.showToast('success', 'Project Created', `Project "${data.name}" has been created.`);
    }

    async onProjectOpened(data) {
        this.currentProject = data;
        document.getElementById('project-name').textContent = data.name;
        document.title = `FlowCut — ${data.name}`;
        this.modules.timeline?.loadFromProject(data);
        this.modules.mediaBrowser?.loadFromProject(data);
        this.showToast('success', 'Project Opened', `Project "${data.name}" loaded.`);
    }

    onProjectClosed() {
        this.currentProject = null;
        document.getElementById('project-name').textContent = 'Untitled Project';
        document.title = 'FlowCut — Professional Video Editor';
        this.modules.timeline?.clearTimeline();
        this.modules.mediaBrowser?.clearMedia();
        this.modules.preview?.clearPreview();
        this.showWelcomeState();
    }

    // ============================================
    // Zoom Management
    // ============================================

    onZoomChanged(data) {
        const zoomSlider = document.getElementById('timeline-zoom');
        if (zoomSlider) zoomSlider.value = data.level;
    }

    // ============================================
    // System Info Update
    // ============================================

    async updateSystemInfo() {
        try {
            const sysInfo = await ipc.getSystemInfo();
            document.getElementById('project-resolution').textContent = `${sysInfo.cpu_cores} cores`;
            document.getElementById('memory-usage').textContent = `${sysInfo.total_memory_mb} MB RAM`;
        } catch (e) {
            console.warn('Could not fetch system info:', e);
        }
    }

    // ============================================
    // Toast Notifications
    // ============================================

    showToast(type, title, message, duration = 5000) {
        const container = document.getElementById('toast-container');
        const toast = document.createElement('div');
        toast.className = `toast ${type}`;

        const iconMap = {
            success: '&#x2713;',
            error: '&#x2717;',
            warning: '&#x26A0;',
            info: '&#x2139;'
        };

        toast.innerHTML = `
            <span class="toast-icon">${iconMap[type] || ''}</span>
            <div class="toast-content">
                <div class="toast-title">${title}</div>
                <div class="toast-message">${message}</div>
            </div>
            <button class="toast-dismiss">&#x2715;</button>
        `;

        container.appendChild(toast);

        // Auto-dismiss
        const timer = setTimeout(() => {
            toast.remove();
        }, duration);

        // Manual dismiss
        toast.querySelector('.toast-dismiss').addEventListener('click', () => {
            clearTimeout(timer);
            toast.remove();
        });
    }

    // ============================================
    // Welcome State
    // ============================================

    showWelcomeState() {
        // Show no-media overlay in preview
        const overlay = document.getElementById('no-media-overlay');
        if (overlay) overlay.style.display = '';
    }
}

// ============================================
// Bootstrap
// ============================================

window.FlowCutApp = FlowCutApp;

const app = new FlowCutApp();
document.addEventListener('DOMContentLoaded', () => {
    app.init();
});

window.app = app;
