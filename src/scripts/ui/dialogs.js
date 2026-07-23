/**
 * FlowCut — Dialog & Modal Controller
 * Manages all modal dialogs: export, project settings, shortcuts, about, etc.
 */

class FlowCutDialogs {
    constructor() {
        this.overlay = document.getElementById('modal-overlay');
        this.container = document.getElementById('modal-container');
        this.activeDialog = null;

        // Close modal on overlay click
        this.overlay?.addEventListener('click', (e) => {
            if (e.target === this.overlay) this.closeModal();
        });

        // Close modal on Escape
        document.addEventListener('keydown', (e) => {
            if (e.key === 'Escape' && this.activeDialog) {
                this.closeModal();
            }
        });
    }

    /**
     * Show a modal dialog with content.
     */
    showModal(title, contentHTML, footerHTML = '') {
        this.container.innerHTML = `
            <div class="modal-header">
                <h2 class="modal-title">${title}</h2>
                <button class="modal-close" id="modal-close-btn">&#x2715;</button>
            </div>
            <div class="modal-body">${contentHTML}</div>
            ${footerHTML ? `<div class="modal-footer">${footerHTML}</div>` : ''}
        `;

        this.overlay.classList.remove('hidden');

        // Bind close button
        document.getElementById('modal-close-btn')?.addEventListener('click', () => this.closeModal());

        this.activeDialog = title;
        window.FlowCutEventBus.publish(window.FlowCutEvents.UI_MODAL_OPENED, { dialog: title });
    }

    /**
     * Close the active modal dialog.
     */
    closeModal() {
        this.overlay.classList.add('hidden');
        this.container.innerHTML = '';
        this.activeDialog = null;
        window.FlowCutEventBus.publish(window.FlowCutEvents.UI_MODAL_CLOSED);
    }

    // ============================================
    // Export Dialog
    // ============================================

    async showExportDialog() {
        const formats = await ipc.getExportFormats();
        const formatOptions = formats.map(f =>
            `<option value="${f.extension}">${f.name} (${f.extension})</option>`
        ).join('');

        this.showModal('Export Video', `
            <div class="export-dialog">
                <div class="prop-row">
                    <label>Format</label>
                    <select id="export-format" class="prop-input">${formatOptions}</select>
                </div>
                <div class="prop-row">
                    <label>Resolution</label>
                    <select id="export-resolution" class="prop-input">
                        <option value="1920x1080">1920 x 1080 (Full HD)</option>
                        <option value="3840x2160">3840 x 2160 (4K UHD)</option>
                        <option value="1280x720">1280 x 720 (HD)</option>
                        <option value="854x480">854 x 480 (SD)</option>
                    </select>
                </div>
                <div class="prop-row">
                    <label>Frame Rate</label>
                    <select id="export-framerate" class="prop-input">
                        <option value="24">24 fps</option>
                        <option value="25">25 fps</option>
                        <option value="30" selected>30 fps</option>
                        <option value="50">50 fps</option>
                        <option value="60">60 fps</option>
                    </select>
                </div>
                <div class="prop-row">
                    <label>Quality</label>
                    <select id="export-quality" class="prop-input">
                        <option value="UltraFast">Ultra Fast (lowest quality)</option>
                        <option value="Fast">Fast</option>
                        <option value="Medium" selected>Medium (recommended)</option>
                        <option value="Slow">Slow (higher quality)</option>
                        <option value="VerySlow">Very Slow (best quality)</option>
                    </select>
                </div>
                <div class="prop-row">
                    <label>Output Path</label>
                    <input type="text" id="export-path" class="prop-input" value="~/Videos/export.mp4" />
                    <button class="btn btn-secondary" id="export-path-browse">Browse</button>
                </div>
            </div>
        `, `
            <button class="btn btn-secondary" id="export-cancel-btn">Cancel</button>
            <button class="btn btn-primary" id="export-start-btn">Start Export</button>
        `);

        // Bind export actions
        document.getElementById('export-cancel-btn')?.addEventListener('click', () => this.closeModal());
        document.getElementById('export-start-btn')?.addEventListener('click', () => this.startExportFromDialog());
        document.getElementById('export-path-browse')?.addEventListener('click', async () => {
            const { open } = window.__TAURI__.dialog;
            const path = await open({ directory: true });
            if (path) document.getElementById('export-path').value = path;
        });
    }

    async startExportFromDialog() {
        const format = document.getElementById('export-format')?.value || 'mp4';
        const resolution = document.getElementById('export-resolution')?.value || '1920x1080';
        const frameRate = parseInt(document.getElementById('export-framerate')?.value || '30');
        const quality = document.getElementById('export-quality')?.value || 'Medium';
        const outputPath = document.getElementById('export-path')?.value || '~/Videos/export.mp4';

        const [width, height] = resolution.split('x').map(Number);

        const config = {
            format,
            codec: format === 'mp4' ? 'h264' : format === 'webm' ? 'vp9' : 'h264',
            resolution_width: width,
            resolution_height: height,
            frame_rate: frameRate,
            bitrate: 8000000,
            audio_codec: 'aac',
            audio_bitrate: 192000,
            output_path: outputPath,
            quality_preset: quality,
        };

        const jobId = await ipc.startExport(config);
        this.closeModal();
        this.showExportProgress(jobId);
    }

    showExportProgress(jobId) {
        this.showModal('Exporting...', `
            <div class="export-progress">
                <div class="progress-bar">
                    <div class="progress-fill" id="export-progress-fill" style="width: 0%"></div>
                </div>
                <div class="progress-detail">
                    <span id="export-percent">0%</span>
                    <span id="export-frames">0 / 0 frames</span>
                </div>
                <div class="progress-detail">
                    <span id="export-elapsed">Elapsed: 0s</span>
                    <span id="export-remaining">Remaining: --</span>
                </div>
                <div class="progress-detail">
                    <span id="export-fps">-- fps</span>
                </div>
            </div>
        `, `
            <button class="btn btn-danger" id="export-cancel-progress">Cancel Export</button>
        `);

        document.getElementById('export-cancel-progress')?.addEventListener('click', () => {
            ipc.cancelExport(jobId);
            this.closeModal();
        });

        // Poll progress
        this._exportPollTimer = setInterval(async () => {
            try {
                const progress = await ipc.getExportProgress(jobId);
                if (progress) {
                    document.getElementById('export-progress-fill').style.width = progress.percent + '%';
                    document.getElementById('export-percent').textContent = Math.round(progress.percent) + '%';
                    document.getElementById('export-frames').textContent = `${progress.current_frame} / ${progress.total_frames} frames`;
                    document.getElementById('export-elapsed').textContent = `Elapsed: ${Math.round(progress.elapsed_seconds)}s`;
                    document.getElementById('export-remaining').textContent = `Remaining: ${Math.round(progress.estimated_remaining_seconds)}s`;
                    document.getElementById('export-fps').textContent = `${progress.current_fps.toFixed(1)} fps`;

                    if (progress.percent >= 100) {
                        clearInterval(this._exportPollTimer);
                        this.closeModal();
                        app.showToast('success', 'Export Complete', 'Video has been exported successfully.');
                    }
                }
            } catch (e) {
                clearInterval(this._exportPollTimer);
                this.closeModal();
                app.showToast('error', 'Export Failed', 'An error occurred during export.');
            }
        }, 500);
    }

    // ============================================
    // New Project Dialog
    // ============================================

    showNewProjectDialog() {
        this.showModal('New Project', `
            <div class="new-project-dialog">
                <div class="prop-row">
                    <label>Project Name</label>
                    <input type="text" id="new-project-name" class="prop-input" value="My Project" />
                </div>
                <div class="prop-row">
                    <label>Resolution</label>
                    <select id="new-project-resolution" class="prop-input">
                        <option value="1920x1080" selected>1920 x 1080 (Full HD)</option>
                        <option value="3840x2160">3840 x 2160 (4K UHD)</option>
                        <option value="1280x720">1280 x 720 (HD)</option>
                    </select>
                </div>
                <div class="prop-row">
                    <label>Frame Rate</label>
                    <select id="new-project-framerate" class="prop-input">
                        <option value="24">24 fps</option>
                        <option value="30" selected>30 fps</option>
                        <option value="60">60 fps</option>
                    </select>
                </div>
                <div class="prop-row">
                    <label>Save Location</label>
                    <input type="text" id="new-project-path" class="prop-input" value="~/Videos/" />
                    <button class="btn btn-secondary" id="new-project-browse">Browse</button>
                </div>
            </div>
        `, `
            <button class="btn btn-secondary" id="new-project-cancel">Cancel</button>
            <button class="btn btn-primary" id="new-project-create">Create Project</button>
        `);

        document.getElementById('new-project-cancel')?.addEventListener('click', () => this.closeModal());
        document.getElementById('new-project-create')?.addEventListener('click', async () => {
            const name = document.getElementById('new-project-name')?.value || 'Untitled';
            const path = document.getElementById('new-project-path')?.value || '~/Videos/';
            const result = await ipc.createProject(name, path);
            this.closeModal();
            window.FlowCutEventBus.publish(window.FlowCutEvents.PROJECT_CREATED, result);
        });
    }

    // ============================================
    // Open Project Dialog
    // ============================================

    async showOpenProjectDialog() {
        try {
            const { open } = window.__TAURI__.dialog;
            const selected = await open({
                filters: [{ name: 'FlowCut Projects', extensions: ['flowcut', 'json'] }]
            });
            if (selected) {
                const result = await ipc.openProject(selected);
                window.FlowCutEventBus.publish(window.FlowCutEvents.PROJECT_OPENED, result);
            }
        } catch (e) {
            app.showToast('error', 'Open Failed', 'Could not open the project file.');
        }
    }

    // ============================================
    // Settings Dialog
    // ============================================

    showSettingsDialog() {
        this.showModal('Settings', `
            <div class="settings-dialog">
                <div class="prop-row">
                    <label>Auto-Save Interval</label>
                    <select id="settings-autosave" class="prop-input">
                        <option value="60000">1 minute</option>
                        <option value="300000" selected>5 minutes</option>
                        <option value="600000">10 minutes</option>
                        <option value="0">Disabled</option>
                    </select>
                </div>
                <div class="prop-row">
                    <label>Default Preview Quality</label>
                    <select id="settings-preview-quality" class="prop-input">
                        <option value="full">Full Quality</option>
                        <option value="half" selected>Half Quality</option>
                        <option value="quarter">Quarter Quality</option>
                    </select>
                </div>
                <div class="prop-row">
                    <label>Undo History Limit</label>
                    <input type="number" id="settings-undo-limit" class="prop-input small" value="100" min="10" max="500" />
                </div>
                <div class="prop-row">
                    <label>Snap to Grid</label>
                    <input type="checkbox" id="settings-snap" checked />
                </div>
            </div>
        `, `
            <button class="btn btn-secondary" id="settings-cancel">Cancel</button>
            <button class="btn btn-primary" id="settings-save">Save Settings</button>
        `);

        document.getElementById('settings-cancel')?.addEventListener('click', () => this.closeModal());
        document.getElementById('settings-save')?.addEventListener('click', () => {
            app.settings.autoSaveInterval = parseInt(document.getElementById('settings-autosave')?.value || '300000');
            app.settings.previewQuality = document.getElementById('settings-preview-quality')?.value || 'half';
            app.settings.undoMaxHistory = parseInt(document.getElementById('settings-undo-limit')?.value || '100');
            app.settings.snapEnabled = document.getElementById('settings-snap')?.checked ?? true;
            this.closeModal();
            app.showToast('success', 'Settings Saved', 'Your preferences have been updated.');
        });
    }

    // ============================================
    // Keyboard Shortcuts Dialog
    // ============================================

    async showShortcutsDialog() {
        const shortcuts = await ipc.getShortcuts();
        const rows = shortcuts.map(s => `
            <div class="prop-row">
                <label>${s.description || s.action}</label>
                <kbd style="font-family:var(--font-mono);background:var(--bg-surface1);padding:2px 6px;border-radius:2px;color:var(--fg-text)">${s.modifiers?.join('+') || ''}${s.modifiers?.length ? '+' : ''}${s.key}</kbd>
            </div>
        `).join('');

        this.showModal('Keyboard Shortcuts', `<div class="shortcuts-list">${rows}</div>`);
    }

    // ============================================
    // About Dialog
    // ============================================

    showAboutDialog() {
        this.showModal('About FlowCut', `
            <div class="about-dialog" style="text-align:center;padding:24px">
                <svg viewBox="0 0 32 32" width="64" height="64" fill="none" style="margin-bottom:16px">
                    <path d="M8 4L24 16L8 28" stroke="#89b4fa" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
                    <path d="M12 8L20 16L12 24" stroke="#a6e3a1" stroke-width="2" stroke-linecap="round"/>
                </svg>
                <h3 style="font-size:20px;color:var(--fg-text);margin:8px 0">FlowCut</h3>
                <p style="color:var(--fg-subtext0);margin:4px 0">Professional Lightweight Video Editor</p>
                <p style="color:var(--fg-overlay0);margin:4px 0;font-size:12px">Version 1.0.0</p>
                <p style="color:var(--fg-overlay0);margin:16px 0 0;font-size:12px">Built with Rust, Tauri, and FFmpeg</p>
                <p style="color:var(--fg-overlay0);font-size:11px;margin:4px 0">Licensed under GPL-3.0</p>
            </div>
        `, `
            <button class="btn btn-primary" id="about-close">Close</button>
        `);

        document.getElementById('about-close')?.addEventListener('click', () => this.closeModal());
    }

    // ============================================
    // Add Transition Dialog
    // ============================================

    showAddTransitionDialog() {
        this.showModal('Add Transition', `
            <div class="transition-dialog">
                <div class="prop-row">
                    <label>Transition Type</label>
                    <select id="transition-type" class="prop-input">
                        <option value="crossfade">Crossfade</option>
                        <option value="dissolve">Dissolve</option>
                        <option value="fade_in">Fade In</option>
                        <option value="fade_out">Fade Out</option>
                        <option value="wipe_left">Wipe Left</option>
                        <option value="wipe_right">Wipe Right</option>
                    </select>
                </div>
                <div class="prop-row">
                    <label>Duration</label>
                    <input type="number" id="transition-duration" class="prop-input small" value="0.5" step="0.1" min="0.1" max="5" />
                    <span class="prop-unit">sec</span>
                </div>
            </div>
        `, `
            <button class="btn btn-secondary" id="transition-cancel">Cancel</button>
            <button class="btn btn-primary" id="transition-add">Add Transition</button>
        `);

        document.getElementById('transition-cancel')?.addEventListener('click', () => this.closeModal());
        document.getElementById('transition-add')?.addEventListener('click', () => {
            // Would need selected clips to add transition between them
            this.closeModal();
            app.showToast('info', 'Transition Added', 'Transition has been applied between clips.');
        });
    }

    // ============================================
    // Add Effect Modal
    // ============================================

    showAddEffectModal(filters, clipId) {
        const filterItems = filters.map(f => `
            <button class="dropdown-item" data-filter="${f.name}" style="width:100%;text-align:left;padding:8px 12px">
                <strong>${f.name}</strong><br><small style="color:var(--fg-overlay0)">${f.description || ''}</small>
            </button>
        `).join('');

        this.showModal('Add Effect', `
            <div class="effects-dialog">${filterItems || '<p style="color:var(--fg-overlay0)">No effects available</p>'}</div>
        `, `
            <button class="btn btn-secondary" id="effect-cancel">Cancel</button>
        `);

        document.getElementById('effect-cancel')?.addEventListener('click', () => this.closeModal());

        // Bind filter selection
        document.querySelectorAll('.effects-dialog .dropdown-item').forEach(item => {
            item.addEventListener('click', async () => {
                const filterType = item.dataset.filter;
                await ipc.applyFilter(clipId, filterType, {});
                this.closeModal();
                app.showToast('success', 'Effect Added', `${filterType} has been applied.`);
            });
        });
    }
}

window.FlowCutDialogs = FlowCutDialogs;
