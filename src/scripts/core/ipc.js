/**
 * FlowCut — Tauri IPC Communication Layer
 * Provides a promise-based API for all Tauri backend commands.
 * Handles errors gracefully and logs all IPC activity.
 */

const { invoke } = window.__TAURI__.core;

class IPCError extends Error {
    constructor(command, message, details) {
        super(`IPC Error [${command}]: ${message}`);
        this.command = command;
        this.details = details;
    }
}

class FlowCutIPC {
    constructor() {
        this._connectionHealthy = true;
        this._retryCount = 0;
        this._maxRetries = 3;
    }

    /**
     * Execute a Tauri invoke command with error handling.
     * @param {string} command - Tauri command name
     * @param {Object} args - Command arguments
     * @returns {Promise<any>} Command result
     */
    async call(command, args = {}) {
        try {
            const result = await invoke(command, args);
            this._connectionHealthy = true;
            this._retryCount = 0;
            console.debug(`IPC [${command}] → success`, result);
            return result;
        } catch (error) {
            this._connectionHealthy = false;
            console.error(`IPC [${command}] → error:`, error);

            // Publish error event
            const eventBus = window.FlowCutEventBus;
            if (eventBus) {
                eventBus.publish(window.FlowCutEvents.ERROR_IPC_FAILED, {
                    command,
                    error: error.toString(),
                    args
                });
            }

            throw new IPCError(command, error.toString(), { args, rawError: error });
        }
    }

    // ============================================
    // Project Commands
    // ============================================

    async createProject(name, path) {
        return this.call('create_project', { name, path });
    }

    async openProject(path) {
        return this.call('open_project', { path });
    }

    async saveProject() {
        return this.call('save_project');
    }

    async closeProject() {
        return this.call('close_project');
    }

    async getProjectInfo() {
        return this.call('get_project_info');
    }

    // ============================================
    // Media Commands
    // ============================================

    async importMedia(paths) {
        return this.call('import_media', { paths });
    }

    async getMediaInfo(id) {
        return this.call('get_media_info', { id });
    }

    async removeMedia(id) {
        return this.call('remove_media', { id });
    }

    async listMedia() {
        return this.call('list_media');
    }

    // ============================================
    // Timeline Commands
    // ============================================

    async addClipToTrack(trackId, mediaId, startTime, duration) {
        return this.call('add_clip_to_track', { trackId, mediaId, startTime, duration });
    }

    async removeClipFromTrack(trackId, clipId) {
        return this.call('remove_clip_from_track', { trackId, clipId });
    }

    async moveClip(trackId, clipId, newStartTime) {
        return this.call('move_clip', { trackId, clipId, newStartTime });
    }

    async splitClip(trackId, clipId, splitTime) {
        return this.call('split_clip', { trackId, clipId, splitTime });
    }

    async trimClip(trackId, clipId, startTrim, endTrim) {
        return this.call('trim_clip', { trackId, clipId, startTrim, endTrim });
    }

    async getTimelineState() {
        return this.call('get_timeline_state');
    }

    async addTrack(trackType, name) {
        return this.call('add_track', { trackType, name });
    }

    async removeTrack(trackId) {
        return this.call('remove_track', { trackId });
    }

    async addTransition(fromClip, toClip, transitionType, duration) {
        return this.call('add_transition', { fromClip, toClip, transitionType, duration });
    }

    async removeTransition(transitionId) {
        return this.call('remove_transition', { transitionId });
    }

    // ============================================
    // Preview Commands
    // ============================================

    async renderPreviewFrame(timestamp) {
        return this.call('render_preview_frame', { timestamp });
    }

    async getPreviewInfo(mediaId) {
        return this.call('get_preview_info', { mediaId });
    }

    async seekPreview(timestamp) {
        return this.call('seek_preview', { timestamp });
    }

    // ============================================
    // Effects / Filters Commands
    // ============================================

    async applyFilter(clipId, filterType, params) {
        return this.call('apply_filter', { clipId, filterType, params });
    }

    async removeFilter(filterId) {
        return this.call('remove_filter', { filterId });
    }

    async listFilters() {
        return this.call('list_filters');
    }

    async getFilterParams(filterId) {
        return this.call('get_filter_params', { filterId });
    }

    async updateFilterParams(filterId, params) {
        return this.call('update_filter_params', { filterId, params });
    }

    // ============================================
    // Export Commands
    // ============================================

    async startExport(config) {
        return this.call('start_export', { config });
    }

    async getExportProgress(jobId) {
        return this.call('get_export_progress', { jobId });
    }

    async cancelExport(jobId) {
        return this.call('cancel_export', { jobId });
    }

    async getExportFormats() {
        return this.call('get_export_formats');
    }

    // ============================================
    // Engine Commands
    // ============================================

    async initializeEngine() {
        return this.call('initialize_engine');
    }

    async getEngineStatus() {
        return this.call('get_engine_status');
    }

    async getSystemInfo() {
        return this.call('get_system_info');
    }

    // ============================================
    // Keyboard Shortcuts
    // ============================================

    async getShortcuts() {
        return this.call('get_shortcuts');
    }

    async setShortcut(action, key, modifiers) {
        return this.call('set_shortcut', { action, key, modifiers });
    }

    // ============================================
    // Undo / Redo
    // ============================================

    async undoAction() {
        return this.call('undo_action');
    }

    async redoAction() {
        return this.call('redo_action');
    }

    async getUndoHistory() {
        return this.call('get_undo_history');
    }

    // ============================================
    // Connection Health Check
    // ============================================

    isHealthy() {
        return this._connectionHealthy;
    }

    getRetryCount() {
        return this._retryCount;
    }
}

// Global IPC instance
const ipc = new FlowCutIPC();
window.FlowCutIPC = ipc;
